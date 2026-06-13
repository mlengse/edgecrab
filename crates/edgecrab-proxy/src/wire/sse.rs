//! Encode provider [`StreamChunk`]s as OpenAI chat completion SSE.

use std::collections::BTreeMap;
use std::convert::Infallible;

use axum::response::sse::Event;
use edgequake_llm::traits::StreamChunk;
use futures::{Stream, StreamExt};
use serde::Serialize;

use crate::stream_agg::StreamAccumulator;
use crate::wire::openai::unix_now;

#[derive(Debug, Serialize)]
struct StreamChunkOut {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<StreamChoiceOut>,
}

#[derive(Debug, Serialize)]
struct StreamChoiceOut {
    index: u32,
    delta: StreamDeltaOut,
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct StreamDeltaOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<DeltaToolCallOut>>,
}

#[derive(Debug, Serialize)]
struct DeltaToolCallOut {
    index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function: Option<DeltaFunctionOut>,
}

#[derive(Debug, Serialize)]
struct DeltaFunctionOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<String>,
}

/// Live state for incremental OpenAI tool_call SSE emission.
struct SseToolState {
    emitted_id: bool,
    emitted_name: bool,
}

/// Convert a stream of `Result<StreamChunk>` into OpenAI SSE events.
pub fn stream_chunks_to_sse(
    chat_id: String,
    model: String,
    chunks: impl Stream<Item = edgequake_llm::Result<StreamChunk>> + Send + 'static,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        let created = unix_now();
        yield Ok(Event::default().data(
            serde_json::to_string(&chunk_json(
                &chat_id, &model, created,
                StreamDeltaOut { role: Some("assistant"), content: None, tool_calls: None },
                None,
            )).unwrap_or_default(),
        ));

        let mut tool_state: BTreeMap<usize, SseToolState> = BTreeMap::new();
        let mut acc = StreamAccumulator::default();
        futures::pin_mut!(chunks);

        while let Some(item) = chunks.next().await {
            match item {
                Ok(StreamChunk::Content(text)) if !text.is_empty() => {
                    let _ = acc.push(StreamChunk::Content(text.clone()));
                    yield Ok(Event::default().data(
                        serde_json::to_string(&chunk_json(
                            &chat_id, &model, created,
                            StreamDeltaOut {
                                role: None,
                                content: Some(text),
                                tool_calls: None,
                            },
                            None,
                        )).unwrap_or_default(),
                    ));
                }
                Ok(StreamChunk::ThinkingContent { text, .. }) if !text.is_empty() => {
                    let _ = acc.push(StreamChunk::ThinkingContent {
                        text: text.clone(),
                        tokens_used: None,
                        budget_total: None,
                    });
                }
                Ok(StreamChunk::ToolCallDelta {
                    index,
                    id,
                    function_name,
                    function_arguments,
                    thought_signature,
                }) => {
                    let _ = acc.push(StreamChunk::ToolCallDelta {
                        index,
                        id: id.clone(),
                        function_name: function_name.clone(),
                        function_arguments: function_arguments.clone(),
                        thought_signature,
                    });
                    let state = tool_state.entry(index).or_insert(SseToolState {
                        emitted_id: false,
                        emitted_name: false,
                    });
                    let mut delta_tool = DeltaToolCallOut {
                        index,
                        id: None,
                        r#type: None,
                        function: None,
                    };
                    let mut fn_delta = DeltaFunctionOut {
                        name: None,
                        arguments: None,
                    };
                    if let Some(ref tid) = id
                        && !state.emitted_id
                    {
                        delta_tool.id = Some(tid.clone());
                        delta_tool.r#type = Some("function");
                        state.emitted_id = true;
                    }
                    if let Some(ref name) = function_name
                        && !state.emitted_name
                    {
                        fn_delta.name = Some(name.clone());
                        state.emitted_name = true;
                    }
                    if let Some(args) = function_arguments
                        && !args.is_empty()
                    {
                        fn_delta.arguments = Some(args);
                    }
                    if fn_delta.name.is_some() || fn_delta.arguments.is_some() {
                        delta_tool.function = Some(fn_delta);
                    }
                    if delta_tool.id.is_some() || delta_tool.function.is_some() {
                        yield Ok(Event::default().data(
                            serde_json::to_string(&chunk_json(
                                &chat_id, &model, created,
                                StreamDeltaOut {
                                    role: None,
                                    content: None,
                                    tool_calls: Some(vec![delta_tool]),
                                },
                                None,
                            )).unwrap_or_default(),
                        ));
                    }
                }
                Ok(StreamChunk::Finished { reason, usage, .. }) => {
                    let _ = acc.push(StreamChunk::Finished {
                        reason: reason.clone(),
                        ttft_ms: None,
                        usage,
                    });
                    yield Ok(Event::default().data(
                        serde_json::to_string(&chunk_json(
                            &chat_id, &model, created,
                            StreamDeltaOut {
                                role: None,
                                content: None,
                                tool_calls: None,
                            },
                            Some(reason),
                        )).unwrap_or_default(),
                    ));
                    break;
                }
                Ok(other) => {
                    let _ = acc.push(other);
                }
                Err(err) => {
                    tracing::warn!(error = %err, "proxy stream chunk error");
                    yield Ok(Event::default().data(
                        serde_json::to_string(&chunk_json(
                            &chat_id, &model, created,
                            StreamDeltaOut {
                                role: None,
                                content: None,
                                tool_calls: None,
                            },
                            Some("stop".into()),
                        )).unwrap_or_default(),
                    ));
                    break;
                }
            }
        }

        // EOF flush: emit stop if the provider closed without Finished.
        yield Ok(Event::default().data(
            serde_json::to_string(&chunk_json(
                &chat_id, &model, created,
                StreamDeltaOut {
                    role: None,
                    content: None,
                    tool_calls: None,
                },
                Some("stop".into()),
            )).unwrap_or_default(),
        ));
        yield Ok(Event::default().data("[DONE]"));
    }
}

fn chunk_json(
    id: &str,
    model: &str,
    created: u64,
    delta: StreamDeltaOut,
    finish_reason: Option<String>,
) -> StreamChunkOut {
    StreamChunkOut {
        id: id.to_string(),
        object: "chat.completion.chunk",
        created,
        model: model.to_string(),
        choices: vec![StreamChoiceOut {
            index: 0,
            delta,
            finish_reason,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::traits::StreamChunk;
    use futures::StreamExt;

    #[tokio::test]
    async fn sse_emits_multiple_events_including_stop() {
        let chunks = futures::stream::iter(vec![
            Ok(StreamChunk::Content("hi".into())),
            Ok(StreamChunk::Finished {
                reason: "stop".into(),
                ttft_ms: None,
                usage: None,
            }),
        ]);
        let sse = stream_chunks_to_sse("id".into(), "mock/m".into(), chunks);
        futures::pin_mut!(sse);
        let mut count = 0usize;
        while let Some(Ok(_ev)) = sse.next().await {
            count += 1;
        }
        // role + content + finish + eof stop + [DONE]
        assert!(count >= 3, "expected multiple SSE events, got {count}");
    }

    #[tokio::test]
    async fn sse_eof_flush_emits_extra_stop_without_provider_finished() {
        let chunks = futures::stream::iter(vec![Ok(StreamChunk::Content("partial".into()))]);
        let sse = stream_chunks_to_sse("id".into(), "mock/m".into(), chunks);
        futures::pin_mut!(sse);
        let mut count = 0usize;
        while let Some(Ok(_ev)) = sse.next().await {
            count += 1;
        }
        // role + content + EOF stop + [DONE] (no Finished from provider)
        assert!(count >= 4, "EOF must flush stop + DONE, got {count} events");
    }

    #[tokio::test]
    async fn sse_tool_delta_stream_produces_multiple_events() {
        let chunks = futures::stream::iter(vec![
            Ok(StreamChunk::ToolCallDelta {
                index: 0,
                id: Some("call_x".into()),
                function_name: Some("get_weather".into()),
                function_arguments: Some("{\"city\":".into()),
                thought_signature: None,
            }),
            Ok(StreamChunk::ToolCallDelta {
                index: 0,
                id: None,
                function_name: None,
                function_arguments: Some("\"Paris\"}".into()),
                thought_signature: None,
            }),
            Ok(StreamChunk::Finished {
                reason: "tool_calls".into(),
                ttft_ms: None,
                usage: None,
            }),
        ]);
        let sse = stream_chunks_to_sse("id".into(), "mock/m".into(), chunks);
        futures::pin_mut!(sse);
        let mut count = 0usize;
        while let Some(Ok(_ev)) = sse.next().await {
            count += 1;
        }
        // role + id/name + args delta + finish + [DONE]
        assert!(
            count >= 4,
            "tool deltas should fan out to multiple SSE events, got {count}"
        );
    }
}
