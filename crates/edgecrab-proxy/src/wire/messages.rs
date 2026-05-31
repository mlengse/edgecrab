//! OpenAI request messages → [`edgequake_llm::ChatMessage`].

use edgequake_llm::{ChatMessage, ToolCall, ToolDefinition, ToolChoice};

use crate::error::ProxyError;
use crate::wire::openai::{ChatCompletionRequest, ChatMessageIn, ToolIn};

#[allow(clippy::type_complexity)]
pub fn openai_messages_to_chat(
    req: &ChatCompletionRequest,
) -> Result<(Vec<ChatMessage>, Vec<ToolDefinition>, Option<ToolChoice>), ProxyError> {
    let mut out = Vec::with_capacity(req.messages.len());
    for msg in &req.messages {
        out.push(map_message(msg)?);
    }
    let tools = req
        .tools
        .as_ref()
        .map(|ts| ts.iter().map(map_tool).collect())
        .unwrap_or_default();
    let tool_choice = map_tool_choice(req.tool_choice.as_ref());
    Ok((out, tools, tool_choice))
}

fn map_message(msg: &ChatMessageIn) -> Result<ChatMessage, ProxyError> {
    let text = msg.content.as_text();
    match msg.role.as_str() {
        "system" => Ok(ChatMessage::system(&text)),
        "user" => Ok(ChatMessage::user(&text)),
        "assistant" => {
            if let Some(ref calls) = msg.tool_calls {
                let llm_calls: Vec<ToolCall> = calls.iter().map(map_tool_call).collect();
                return Ok(ChatMessage::assistant_with_tools(&text, llm_calls));
            }
            Ok(ChatMessage::assistant(&text))
        }
        "tool" => {
            let id = msg
                .tool_call_id
                .as_deref()
                .ok_or_else(|| ProxyError::BadRequest("tool message missing tool_call_id".into()))?;
            let mut chat = ChatMessage::tool_result(id, &text);
            chat.name = msg.name.clone();
            Ok(chat)
        }
        other => Err(ProxyError::BadRequest(format!("unsupported message role: {other}"))),
    }
}

fn map_tool_call(tc: &crate::wire::openai::ToolCallIn) -> ToolCall {
    ToolCall {
        id: tc.id.clone(),
        call_type: tc.call_type.clone().unwrap_or_else(|| "function".into()),
        function: edgequake_llm::FunctionCall {
            name: tc.function.name.clone(),
            arguments: tc.function.arguments.clone(),
        },
        thought_signature: None,
    }
}

fn map_tool(t: &ToolIn) -> ToolDefinition {
    ToolDefinition {
        tool_type: t.tool_type.clone(),
        function: edgequake_llm::FunctionDefinition {
            name: t.function.name.clone(),
            description: t
                .function
                .description
                .clone()
                .unwrap_or_else(|| t.function.name.clone()),
            parameters: t.function.parameters.clone(),
            strict: t.function.strict,
        },
    }
}

fn map_tool_choice(raw: Option<&serde_json::Value>) -> Option<ToolChoice> {
    let value = raw?;
    match value {
        serde_json::Value::String(s) if s == "auto" => Some(ToolChoice::auto()),
        serde_json::Value::String(s) if s == "none" => Some(ToolChoice::none()),
        serde_json::Value::String(s) if s == "required" => Some(ToolChoice::required()),
        serde_json::Value::Object(obj) => {
            if let Some(name) = obj
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
            {
                Some(ToolChoice::function(name))
            } else {
                Some(ToolChoice::auto())
            }
        }
        _ => Some(ToolChoice::auto()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::openai::{ChatCompletionRequest, ChatMessageIn, MessageContentIn};

    #[test]
    fn maps_tool_roundtrip_roles() {
        let req = ChatCompletionRequest {
            model: "mock/test".into(),
            messages: vec![
                ChatMessageIn {
                    role: "user".into(),
                    content: MessageContentIn::Text("hi".into()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: false,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
        };
        let (msgs, tools, _) = openai_messages_to_chat(&req).expect("map");
        assert_eq!(msgs.len(), 1);
        assert!(tools.is_empty());
    }

    #[test]
    fn maps_openai_tools_and_tool_choice() {
        let req = ChatCompletionRequest {
            model: "mock/test".into(),
            messages: vec![ChatMessageIn {
                role: "user".into(),
                content: MessageContentIn::Text("weather?".into()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            stream: false,
            temperature: None,
            max_tokens: None,
            tools: Some(vec![ToolIn {
                tool_type: "function".into(),
                function: crate::wire::openai::FunctionIn {
                    name: "get_weather".into(),
                    description: Some("Get weather".into()),
                    parameters: serde_json::json!({"type": "object"}),
                    strict: None,
                },
            }]),
            tool_choice: Some(serde_json::json!("required")),
        };
        let (_, tools, choice) = openai_messages_to_chat(&req).expect("map");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "get_weather");
        assert!(matches!(choice, Some(ToolChoice::Required(_))));
    }

    #[test]
    fn tool_role_requires_tool_call_id() {
        let req = ChatCompletionRequest {
            model: "mock/test".into(),
            messages: vec![ChatMessageIn {
                role: "tool".into(),
                content: MessageContentIn::Text("sunny".into()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            stream: false,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
        };
        assert!(openai_messages_to_chat(&req).is_err());
    }
}
