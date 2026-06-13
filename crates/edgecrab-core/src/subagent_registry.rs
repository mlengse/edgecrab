//! Live subagent interrupt registry — Hermes `delegate_tool.interrupt_subagent` parity.
//!
//! Child agents register while `execute_loop` runs so the TUI can call
//! `interrupt_subagent(id)` without gateway RPC.

use std::collections::HashMap;
use std::sync::{Mutex, Weak};

use crate::Agent;

static ACTIVE: Mutex<Option<HashMap<String, Weak<Agent>>>> = Mutex::new(None);

fn map_mut<R>(f: impl FnOnce(&mut HashMap<String, Weak<Agent>>) -> R) -> R {
    let mut slot = ACTIVE.lock().expect("subagent registry mutex");
    if slot.is_none() {
        *slot = Some(HashMap::new());
    }
    f(slot.as_mut().expect("initialized"))
}

/// Register a running child for TUI interrupt lookup.
pub fn register_subagent(agent_id: &str, agent: std::sync::Arc<Agent>) {
    if agent_id.trim().is_empty() {
        return;
    }
    map_mut(|m| {
        m.insert(agent_id.to_string(), std::sync::Arc::downgrade(&agent));
    });
}

/// Drop registration when the child finishes (success, error, or cancel).
pub fn unregister_subagent(agent_id: &str) {
    if agent_id.trim().is_empty() {
        return;
    }
    map_mut(|m| {
        m.remove(agent_id);
    });
}

/// Request interrupt on a single running subagent. Returns true when found.
pub fn interrupt_subagent(agent_id: &str) -> bool {
    let id = agent_id.trim();
    if id.is_empty() {
        return false;
    }
    let agent = map_mut(|m| m.get(id).and_then(|weak| weak.upgrade()));
    let Some(agent) = agent else {
        return false;
    };
    agent.interrupt();
    true
}

/// RAII guard — unregister on drop.
pub struct SubagentRegistration {
    agent_id: String,
    /// Keeps the registered child alive while the guard is held (`Weak` in the global map).
    _agent: std::sync::Arc<Agent>,
}

impl SubagentRegistration {
    pub fn new(agent_id: String, agent: std::sync::Arc<Agent>) -> Self {
        register_subagent(&agent_id, agent.clone());
        Self {
            agent_id,
            _agent: agent,
        }
    }
}

impl Drop for SubagentRegistration {
    fn drop(&mut self) {
        unregister_subagent(&self.agent_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use edgequake_llm::{LLMProvider, MockProvider};

    use crate::AgentBuilder;

    fn test_agent() -> Arc<Agent> {
        let provider: Arc<dyn LLMProvider> = Arc::new(MockProvider::new());
        Arc::new(
            AgentBuilder::new("mock")
                .provider(provider)
                .build()
                .expect("agent"),
        )
    }

    #[tokio::test]
    async fn interrupt_hits_registered_child() {
        let agent = test_agent();
        assert!(!agent.is_cancelled());
        register_subagent("sa-0", agent.clone());
        assert!(interrupt_subagent("sa-0"));
        assert!(agent.is_cancelled());
        unregister_subagent("sa-0");
        assert!(!interrupt_subagent("sa-0"));
    }

    #[tokio::test]
    async fn registration_guard_unregisters_on_drop() {
        let agent = test_agent();
        {
            let _reg = SubagentRegistration::new("sa-1".into(), agent);
            assert!(interrupt_subagent("sa-1"));
        }
        assert!(!interrupt_subagent("sa-1"));
    }
}
