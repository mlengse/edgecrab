//! Gateway watcher for CLI → platform session handoff (Hermes parity).

use std::sync::Arc;
use std::time::Duration;

use edgecrab_core::{
    Agent,
    format_session_handoff_synthetic_message,
    gateway_home::{handoff_platform_from_name, resolve_gateway_home_channel},
};
use edgecrab_state::SessionDb;
use edgecrab_types::OriginChat;

use crate::platform::{MessageMetadata, OutgoingMessage, PlatformAdapter};
use crate::session::{SessionKey, SessionManager};

/// Poll interval for pending session handoffs.
const WATCHER_INTERVAL: Duration = Duration::from_secs(2);
const STARTUP_DELAY: Duration = Duration::from_secs(5);

pub struct SessionHandoffWatcher {
    db: Arc<SessionDb>,
    session_manager: Arc<SessionManager>,
    base_agent: Arc<Agent>,
    adapters: Vec<Arc<dyn PlatformAdapter>>,
    cancel: tokio_util::sync::CancellationToken,
}

impl SessionHandoffWatcher {
    pub fn new(
        db: Arc<SessionDb>,
        session_manager: Arc<SessionManager>,
        base_agent: Arc<Agent>,
        adapters: Vec<Arc<dyn PlatformAdapter>>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self {
            db,
            session_manager,
            base_agent,
            adapters,
            cancel,
        }
    }

    pub async fn run(self) {
        tokio::time::sleep(STARTUP_DELAY).await;
        let mut interval = tokio::time::interval(WATCHER_INTERVAL);
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(err) = self.tick().await {
                        tracing::debug!(error = %err, "session handoff watcher tick failed");
                    }
                }
            }
        }
    }

    async fn tick(&self) -> anyhow::Result<()> {
        let pending = self.db.list_pending_session_handoffs()?;
        for row in pending {
            let session_id = row.session_id.clone();
            if !self.db.claim_session_handoff(&session_id)? {
                continue;
            }
            match self.process_one(&row).await {
                Ok(()) => {
                    if let Err(err) = self.db.complete_session_handoff(&session_id) {
                        tracing::warn!(error = %err, %session_id, "complete_session_handoff failed");
                    }
                }
                Err(err) => {
                    tracing::warn!(error = %err, %session_id, "session handoff failed");
                    let _ = self.db.fail_session_handoff(&session_id, &err.to_string());
                }
            }
        }
        Ok(())
    }

    async fn process_one(
        &self,
        row: &edgecrab_state::PendingSessionHandoff,
    ) -> anyhow::Result<()> {
        let platform_name = row.platform.trim().to_ascii_lowercase();
        let platform = handoff_platform_from_name(&platform_name)
            .ok_or_else(|| anyhow::anyhow!("unknown platform '{platform_name}'"))?;
        let adapter = self
            .adapters
            .iter()
            .find(|a| a.platform() == platform)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("platform '{platform_name}' is not active in this gateway"))?;

        let config = edgecrab_core::AppConfig::load().unwrap_or_default();
        let home_channel = resolve_gateway_home_channel(&config.gateway, platform)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no home channel configured for {platform_name}; run /sethome on the destination chat first"
                )
            })?;

        let cli_title = row
            .title
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or(&row.session_id[..row.session_id.len().min(8)]);

        let thread_name = format!("EdgeCrab — {cli_title}");
        let thread_id = adapter
            .create_handoff_thread(&home_channel, &thread_name)
            .await
            .ok()
            .flatten();

        let routing_key = match thread_id.as_deref() {
            Some(thread) => format!("{home_channel}:{thread}"),
            None => home_channel.clone(),
        };

        let session_key = SessionKey::new(platform, &routing_key, Some(&home_channel));
        let origin_chat = OriginChat::new(platform.to_string(), home_channel.clone());

        let gateway_session = self
            .session_manager
            .rebind_cli_session(
                &session_key,
                &row.session_id,
                &self.base_agent,
                origin_chat,
            )
            .await?;

        self.db.rebind_session_routing(&row.session_id, &platform_name, &routing_key)?;

        let synthetic = format_session_handoff_synthetic_message(cli_title);
        let agent = {
            let guard = gateway_session.read().await;
            guard.agent.clone()
        };
        let response = agent.chat(&synthetic).await.map_err(|e| anyhow::anyhow!("{e}"))?;

        let metadata = MessageMetadata {
            channel_id: Some(home_channel.clone()),
            thread_id: thread_id.clone(),
            ..Default::default()
        };
        adapter
            .send(OutgoingMessage {
                text: response,
                metadata,
            })
            .await?;

        tracing::info!(
            session_id = %row.session_id,
            platform = %platform_name,
            home = %home_channel,
            ?thread_id,
            "session handoff complete"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgecrab_types::Platform;

    #[test]
    fn parse_supported_platforms() {
        assert_eq!(
            handoff_platform_from_name("telegram"),
            Some(Platform::Telegram)
        );
        assert!(handoff_platform_from_name("unknown").is_none());
    }
}
