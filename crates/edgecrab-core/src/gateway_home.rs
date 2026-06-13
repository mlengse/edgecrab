//! Gateway home-channel resolution — single source for handoff, send_message, /sethome.
//!
//! Config YAML values take precedence; env vars are the fallback (useful for
//! containers and platforms without dedicated config structs yet).

use edgecrab_types::Platform;

use crate::config::GatewayConfig;

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

/// Resolve the configured home channel for proactive delivery / session handoff.
pub fn resolve_gateway_home_channel(gateway: &GatewayConfig, platform: Platform) -> Option<String> {
    home_channel_from_config(gateway, platform).or_else(|| home_channel_from_env(platform))
}

fn home_channel_from_config(gateway: &GatewayConfig, platform: Platform) -> Option<String> {
    non_empty(match platform {
        Platform::Telegram => gateway.telegram.home_channel.clone(),
        Platform::Discord => gateway.discord.home_channel.clone(),
        Platform::Slack => gateway.slack.home_channel.clone(),
        Platform::Whatsapp => gateway.whatsapp.home_channel.clone(),
        Platform::Signal => gateway.signal.home_channel.clone(),
        _ => None,
    })
}

fn home_channel_from_env(platform: Platform) -> Option<String> {
    let key = match platform {
        Platform::Telegram => "TELEGRAM_HOME_CHANNEL",
        Platform::Discord => "DISCORD_HOME_CHANNEL",
        Platform::Slack => "SLACK_HOME_CHANNEL",
        Platform::Whatsapp => "WHATSAPP_HOME_CHANNEL",
        Platform::Signal => "SIGNAL_HOME_CHANNEL",
        Platform::Matrix => "MATRIX_HOME_CHANNEL",
        Platform::Feishu => "FEISHU_HOME_CHANNEL",
        Platform::Wecom => "WECOM_HOME_CHANNEL",
        Platform::Email => "EMAIL_HOME_CHANNEL",
        Platform::Mattermost => "MATTERMOST_HOME_CHANNEL",
        Platform::DingTalk => "DINGTALK_HOME_CHANNEL",
        Platform::Sms => "SMS_HOME_CHANNEL",
        Platform::Webhook => "WEBHOOK_HOME_CHANNEL",
        Platform::HomeAssistant => "HOMEASSISTANT_HOME_CHANNEL",
        Platform::BlueBubbles => "BLUEBUBBLES_HOME_CHANNEL",
        Platform::Weixin => "WEIXIN_HOME_CHANNEL",
        Platform::Api => "API_HOME_CHANNEL",
        Platform::Cli | Platform::Cron | Platform::Acp => return None,
    };
    non_empty(std::env::var(key).ok())
}

/// Platforms that support `/handoff` from CLI.
pub fn handoff_platform_from_name(name: &str) -> Option<Platform> {
    match name.trim().to_ascii_lowercase().as_str() {
        "telegram" => Some(Platform::Telegram),
        "discord" => Some(Platform::Discord),
        "slack" => Some(Platform::Slack),
        "whatsapp" => Some(Platform::Whatsapp),
        "signal" => Some(Platform::Signal),
        "matrix" => Some(Platform::Matrix),
        _ => None,
    }
}

/// Human-readable list for CLI errors.
pub const HANDOFF_PLATFORM_HINT: &str = "telegram, discord, slack, whatsapp, signal, matrix";

impl GatewayConfig {
    /// Persist a home channel for a named gateway platform (`/sethome`).
    pub fn set_home_channel(
        &mut self,
        platform: &str,
        channel: Option<String>,
    ) -> Result<(), String> {
        let channel = non_empty(channel);
        match platform.trim().to_ascii_lowercase().as_str() {
            "telegram" => {
                self.telegram.enabled = true;
                self.enable_platform("telegram");
                self.telegram.home_channel = channel;
            }
            "discord" => {
                self.discord.enabled = true;
                self.enable_platform("discord");
                self.discord.home_channel = channel;
            }
            "slack" => {
                self.slack.enabled = true;
                self.enable_platform("slack");
                self.slack.home_channel = channel;
            }
            "whatsapp" => {
                self.whatsapp.enabled = true;
                self.enable_platform("whatsapp");
                self.whatsapp.home_channel = channel;
            }
            "signal" => {
                self.signal.enabled = true;
                self.enable_platform("signal");
                self.signal.home_channel = channel;
            }
            "matrix" => {
                self.enable_platform("matrix");
                if let Some(ref id) = channel {
                    // SAFETY: `/sethome` persists matrix home for this process; mirrors env override path.
                    #[allow(unsafe_code)]
                    unsafe {
                        std::env::set_var("MATRIX_HOME_CHANNEL", id);
                    }
                } else {
                    #[allow(unsafe_code)]
                    unsafe {
                        std::env::remove_var("MATRIX_HOME_CHANNEL");
                    }
                }
            }
            other => {
                return Err(format!(
                    "Unsupported platform '{other}'. Supported: {HANDOFF_PLATFORM_HINT}"
                ));
            }
        }
        Ok(())
    }

    /// List platform names that are enabled and accept `/sethome`.
    pub fn home_channel_platforms(&self) -> Vec<&'static str> {
        let mut platforms = Vec::new();
        let candidates = [
            (
                "telegram",
                self.platform_enabled("telegram") || self.telegram.enabled,
            ),
            (
                "discord",
                self.platform_enabled("discord") || self.discord.enabled,
            ),
            (
                "slack",
                self.platform_enabled("slack") || self.slack.enabled,
            ),
            (
                "whatsapp",
                self.platform_enabled("whatsapp") || self.whatsapp.enabled,
            ),
            (
                "signal",
                self.platform_enabled("signal") || self.signal.enabled,
            ),
            ("matrix", self.platform_enabled("matrix")),
        ];
        for (name, enabled) in candidates {
            if enabled {
                platforms.push(name);
            }
        }
        platforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_home_channel_beats_empty_string() {
        let mut gateway = GatewayConfig::default();
        gateway.telegram.home_channel = Some("  ".into());
        assert!(resolve_gateway_home_channel(&gateway, Platform::Telegram).is_none());
        gateway.telegram.home_channel = Some("chat-1".into());
        assert_eq!(
            resolve_gateway_home_channel(&gateway, Platform::Telegram).as_deref(),
            Some("chat-1")
        );
    }

    #[test]
    fn env_fallback_for_matrix() {
        let gateway = GatewayConfig::default();
        unsafe { std::env::set_var("MATRIX_HOME_CHANNEL", " !room:abc ") };
        let resolved = resolve_gateway_home_channel(&gateway, Platform::Matrix);
        unsafe { std::env::remove_var("MATRIX_HOME_CHANNEL") };
        assert_eq!(resolved.as_deref(), Some("!room:abc"));
    }

    #[test]
    fn set_home_channel_whatsapp_roundtrip() {
        let mut gateway = GatewayConfig::default();
        gateway
            .set_home_channel("whatsapp", Some("+15551234567".into()))
            .expect("set");
        assert_eq!(
            gateway.whatsapp.home_channel.as_deref(),
            Some("+15551234567")
        );
    }
}
