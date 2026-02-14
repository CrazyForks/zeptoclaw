# Streaming Responses Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add optional token-by-token streaming for CLI mode so users see responses appearing in real-time instead of waiting for the full LLM completion.

**Architecture:** New `StreamEvent` enum + `chat_stream()` default method on `LLMProvider` trait (with non-streaming fallback). ClaudeProvider overrides it to parse SSE events via `reqwest::Response::bytes_stream()`. Agent loop branches on `streaming` flag for the final LLM call only. Enabled via `--stream` CLI flag or `streaming: true` config.

**Tech Stack:** Rust, tokio mpsc channels, reqwest `stream` feature, Anthropic SSE streaming API

---

### Task 1: Add `stream` feature to reqwest in Cargo.toml

**Files:**
- Modify: `Cargo.toml:34`

**Step 1: Add `stream` feature to reqwest dependency**

In `Cargo.toml`, change line 34 from:

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

to:

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls", "stream"], default-features = false }
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: compiles without errors

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add stream feature to reqwest for SSE support"
```

---

### Task 2: Add `StreamEvent` enum and `chat_stream()` to LLMProvider trait

**Files:**
- Modify: `src/providers/types.rs`
- Modify: `src/providers/mod.rs:38` (re-export `StreamEvent`)

**Step 1: Write the failing test**

Add to the bottom of `src/providers/types.rs` `mod tests` block:

```rust
#[tokio::test]
async fn test_stream_event_done_carries_content() {
    let event = StreamEvent::Done {
        content: "hello".to_string(),
        usage: Some(Usage::new(10, 5)),
    };
    match event {
        StreamEvent::Done { content, usage } => {
            assert_eq!(content, "hello");
            assert!(usage.is_some());
        }
        _ => panic!("Expected Done event"),
    }
}

#[tokio::test]
async fn test_stream_event_delta() {
    let event = StreamEvent::Delta("chunk".to_string());
    match event {
        StreamEvent::Delta(text) => assert_eq!(text, "chunk"),
        _ => panic!("Expected Delta event"),
    }
}

#[tokio::test]
async fn test_stream_event_tool_calls() {
    let tc = LLMToolCall::new("call_1", "search", r#"{"q":"rust"}"#);
    let event = StreamEvent::ToolCalls(vec![tc]);
    match event {
        StreamEvent::ToolCalls(calls) => {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].name, "search");
        }
        _ => panic!("Expected ToolCalls event"),
    }
}

#[tokio::test]
async fn test_stream_event_error() {
    let event = StreamEvent::Error(ZeptoError::Provider("fail".into()));
    assert!(matches!(event, StreamEvent::Error(_)));
}

#[tokio::test]
async fn test_chat_stream_default_impl() {
    use std::sync::Arc;

    struct FakeProvider;

    #[async_trait]
    impl LLMProvider for FakeProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Vec<ToolDefinition>,
            _model: Option<&str>,
            _options: ChatOptions,
        ) -> Result<LLMResponse> {
            Ok(LLMResponse::text("hello from fake"))
        }
        fn default_model(&self) -> &str { "fake" }
        fn name(&self) -> &str { "fake" }
    }

    let provider = FakeProvider;
    let mut rx = provider
        .chat_stream(vec![], vec![], None, ChatOptions::default())
        .await
        .unwrap();

    let event = rx.recv().await.unwrap();
    match event {
        StreamEvent::Done { content, .. } => {
            assert_eq!(content, "hello from fake");
        }
        _ => panic!("Expected Done event from default chat_stream"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib providers::types::tests::test_stream_event -- --nocapture`
Expected: FAIL — `StreamEvent` type doesn't exist yet

**Step 3: Add `StreamEvent` enum to `src/providers/types.rs`**

Add the following after the `use` block (before the `ToolDefinition` struct, around line 11):

```rust
use crate::error::ZeptoError;

/// Events emitted during streaming LLM responses.
#[derive(Debug)]
pub enum StreamEvent {
    /// A chunk of text content from the LLM.
    Delta(String),
    /// Tool calls detected mid-stream (triggers fallback to non-streaming tool loop).
    ToolCalls(Vec<LLMToolCall>),
    /// Stream complete — carries the full assembled content and usage stats.
    Done { content: String, usage: Option<Usage> },
    /// Provider error mid-stream.
    Error(ZeptoError),
}
```

**Step 4: Add `chat_stream()` default method to `LLMProvider` trait**

In `src/providers/types.rs`, add the following method to the `LLMProvider` trait (after the `name()` method, around line 95):

```rust
    /// Send a streaming chat completion request.
    ///
    /// Returns an `mpsc::Receiver` that yields `StreamEvent`s.
    /// The default implementation wraps `chat()` and emits a single `Done` event.
    /// Providers that support SSE streaming should override this.
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        model: Option<&str>,
        options: ChatOptions,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>> {
        let response = self.chat(messages, tools, model, options).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx
            .send(StreamEvent::Done {
                content: response.content,
                usage: response.usage,
            })
            .await;
        Ok(rx)
    }
```

Also add the `use crate::error::ZeptoError;` import at the top if not already present. The file already imports `use crate::error::Result;` — add `ZeptoError` to the same line:

```rust
use crate::error::{Result, ZeptoError};
```

**Step 5: Re-export `StreamEvent` from `src/providers/mod.rs`**

Change line 38 of `src/providers/mod.rs` from:

```rust
pub use types::{ChatOptions, LLMProvider, LLMResponse, LLMToolCall, ToolDefinition, Usage};
```

to:

```rust
pub use types::{ChatOptions, LLMProvider, LLMResponse, LLMToolCall, StreamEvent, ToolDefinition, Usage};
```

**Step 6: Run tests to verify they pass**

Run: `cargo test --lib providers::types::tests -- --nocapture`
Expected: All PASS (including the new 5 stream tests)

**Step 7: Commit**

```bash
git add src/providers/types.rs src/providers/mod.rs
git commit -m "feat: add StreamEvent enum and chat_stream() default impl to LLMProvider"
```

---

### Task 3: Add `streaming` config field and `--stream` CLI flag

**Files:**
- Modify: `src/config/types.rs:49-84` (AgentDefaults struct + Default impl)
- Modify: `src/main.rs:57-61` (Agent command variant)
- Modify: `src/main.rs:1109-1120` (cmd_agent single message path)

**Step 1: Write the failing test**

Add to `src/config/types.rs` `mod tests`:

```rust
#[test]
fn test_streaming_defaults_to_false() {
    let defaults = AgentDefaults::default();
    assert!(!defaults.streaming);
}

#[test]
fn test_streaming_config_deserialize() {
    let json = r#"{"streaming": true}"#;
    let defaults: AgentDefaults = serde_json::from_str(json).unwrap();
    assert!(defaults.streaming);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib config::types::tests::test_streaming -- --nocapture`
Expected: FAIL — `streaming` field doesn't exist

**Step 3: Add `streaming` field to `AgentDefaults`**

In `src/config/types.rs`, add to the `AgentDefaults` struct (line 63, after `message_queue_mode`):

```rust
    /// Whether to stream the final LLM response token-by-token in CLI mode.
    pub streaming: bool,
```

In the `Default for AgentDefaults` impl (around line 83, after `message_queue_mode`):

```rust
            streaming: false,
```

**Step 4: Add `--stream` flag to Agent CLI command**

In `src/main.rs`, change the `Agent` variant (lines 57-61) from:

```rust
    Agent {
        /// Direct message to process (non-interactive mode)
        #[arg(short, long)]
        message: Option<String>,
    },
```

to:

```rust
    Agent {
        /// Direct message to process (non-interactive mode)
        #[arg(short, long)]
        message: Option<String>,
        /// Stream the response token-by-token
        #[arg(long)]
        stream: bool,
    },
```

**Step 5: Update the match arm in main()**

Change line 163 from:

```rust
        Some(Commands::Agent { message }) => {
            cmd_agent(message).await?;
```

to:

```rust
        Some(Commands::Agent { message, stream }) => {
            cmd_agent(message, stream).await?;
```

**Step 6: Update cmd_agent signature and pass stream flag**

Change the `cmd_agent` function signature (line 1078) from:

```rust
async fn cmd_agent(message: Option<String>) -> Result<()> {
```

to:

```rust
async fn cmd_agent(message: Option<String>, stream: bool) -> Result<()> {
```

After creating the agent (around line 1086), add:

```rust
    // Enable streaming if --stream flag or config streaming is set
    let _streaming = stream || config.agents.defaults.streaming;
```

(We store it for now; Task 5 will use it in the actual streaming branch.)

**Step 7: Run tests to verify they pass**

Run: `cargo test --lib config::types::tests::test_streaming -- --nocapture`
Expected: PASS

Run: `cargo check`
Expected: compiles

**Step 8: Commit**

```bash
git add src/config/types.rs src/main.rs
git commit -m "feat: add streaming config field and --stream CLI flag"
```

---

### Task 4: Implement Claude SSE streaming in ClaudeProvider

**Files:**
- Modify: `src/providers/claude.rs`

This is the largest task. ClaudeProvider overrides `chat_stream()` to:
1. Set `"stream": true` in the request
2. Parse SSE events from the byte stream
3. Send `StreamEvent`s through an mpsc channel

**Step 1: Write the failing test**

Add these tests to `src/providers/claude.rs` `mod tests`:

```rust
#[test]
fn test_claude_request_with_stream_flag() {
    let request = ClaudeRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        max_tokens: 1000,
        messages: vec![],
        system: None,
        tools: None,
        temperature: None,
        top_p: None,
        stop_sequences: None,
        stream: Some(true),
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains(r#""stream":true"#));
}

#[test]
fn test_claude_request_without_stream_flag() {
    let request = ClaudeRequest {
        model: "claude-sonnet-4-5-20250929".to_string(),
        max_tokens: 1000,
        messages: vec![],
        system: None,
        tools: None,
        temperature: None,
        top_p: None,
        stop_sequences: None,
        stream: None,
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(!json.contains("stream"));
}

#[test]
fn test_parse_sse_content_block_delta() {
    let line = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
    let event_type = parsed["type"].as_str().unwrap();
    assert_eq!(event_type, "content_block_delta");
    let text = parsed["delta"]["text"].as_str().unwrap();
    assert_eq!(text, "Hello");
}

#[test]
fn test_parse_sse_message_stop() {
    let line = r#"{"type":"message_stop"}"#;
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
    let event_type = parsed["type"].as_str().unwrap();
    assert_eq!(event_type, "message_stop");
}

#[test]
fn test_parse_sse_message_delta_with_usage() {
    let line = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":42}}"#;
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
    let event_type = parsed["type"].as_str().unwrap();
    assert_eq!(event_type, "message_delta");
    let output_tokens = parsed["usage"]["output_tokens"].as_u64().unwrap();
    assert_eq!(output_tokens, 42);
}

#[test]
fn test_parse_sse_content_block_start_tool_use() {
    let line = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_01","name":"web_search","input":{}}}"#;
    let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
    let block_type = parsed["content_block"]["type"].as_str().unwrap();
    assert_eq!(block_type, "tool_use");
    let tool_name = parsed["content_block"]["name"].as_str().unwrap();
    assert_eq!(tool_name, "web_search");
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib providers::claude::tests::test_claude_request_with_stream -- --nocapture`
Expected: FAIL — `ClaudeRequest` doesn't have `stream` field

**Step 3: Add `stream` field to `ClaudeRequest`**

In `src/providers/claude.rs`, add to the `ClaudeRequest` struct (after `stop_sequences` field, around line 201):

```rust
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
```

**Step 4: Update existing `chat()` to set `stream: None`**

In the `chat()` method's `ClaudeRequest` builder (around line 117-130), add:

```rust
            stream: None,
```

after the `stop_sequences` field.

**Step 5: Add SSE stream event types**

Add the following structs after the `ClaudeUsage` struct (around line 294):

```rust
// ============================================================================
// Claude SSE Streaming Types
// ============================================================================

/// A single SSE event from Claude's streaming API.
/// We parse the JSON `data:` payload into this loosely-typed struct and
/// dispatch on the `type` field.
#[derive(Debug, Deserialize)]
struct SseEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<SseDelta>,
    #[serde(default)]
    content_block: Option<SseContentBlock>,
    #[serde(default)]
    usage: Option<SseUsage>,
    #[serde(default)]
    index: Option<u32>,
    #[serde(default)]
    message: Option<SseMessage>,
}

#[derive(Debug, Deserialize)]
struct SseDelta {
    #[serde(rename = "type")]
    #[serde(default)]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    partial_json: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseUsage {
    #[serde(default)]
    input_tokens: Option<u32>,
    #[serde(default)]
    output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SseMessage {
    #[serde(default)]
    usage: Option<SseUsage>,
}
```

**Step 6: Implement `chat_stream()` override on ClaudeProvider**

Add the following method to the `impl LLMProvider for ClaudeProvider` block (after the `chat()` method, before `default_model()`):

```rust
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        model: Option<&str>,
        options: ChatOptions,
    ) -> crate::error::Result<tokio::sync::mpsc::Receiver<super::StreamEvent>> {
        use futures::StreamExt;
        use super::StreamEvent;

        let model = model.unwrap_or(DEFAULT_MODEL);
        let (system, claude_messages) = convert_messages(messages)?;

        let request = ClaudeRequest {
            model: model.to_string(),
            max_tokens: options.max_tokens.unwrap_or(8192),
            messages: claude_messages,
            system,
            tools: if tools.is_empty() {
                None
            } else {
                Some(convert_tools(tools))
            },
            temperature: options.temperature,
            top_p: options.top_p,
            stop_sequences: options.stop,
            stream: Some(true),
        };

        let response = self
            .client
            .post(CLAUDE_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            if let Ok(error_response) = serde_json::from_str::<ClaudeErrorResponse>(&error_text) {
                return Err(ZeptoError::Provider(format!(
                    "Claude API error ({}): {} - {}",
                    status, error_response.error.r#type, error_response.error.message
                )));
            }
            return Err(ZeptoError::Provider(format!(
                "Claude API error ({}): {}",
                status, error_text
            )));
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<StreamEvent>(32);

        // Spawn a background task to read SSE events and forward them
        let byte_stream = response.bytes_stream();
        tokio::spawn(async move {
            let mut assembled_content = String::new();
            let mut tool_calls: Vec<super::LLMToolCall> = Vec::new();
            let mut current_tool_id: Option<String> = None;
            let mut current_tool_name: Option<String> = None;
            let mut current_tool_json = String::new();
            let mut input_tokens: u32 = 0;
            let mut output_tokens: u32 = 0;
            let mut line_buffer = String::new();

            tokio::pin!(byte_stream);

            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = match chunk_result {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        let _ = tx.send(StreamEvent::Error(
                            ZeptoError::Provider(format!("Stream read error: {}", e))
                        )).await;
                        return;
                    }
                };

                let chunk_str = String::from_utf8_lossy(&chunk);
                line_buffer.push_str(&chunk_str);

                // Process complete lines from the buffer
                while let Some(newline_pos) = line_buffer.find('\n') {
                    let line = line_buffer[..newline_pos].trim().to_string();
                    line_buffer = line_buffer[newline_pos + 1..].to_string();

                    if line.is_empty() || line.starts_with("event:") {
                        continue;
                    }

                    let data = if let Some(stripped) = line.strip_prefix("data: ") {
                        stripped
                    } else if let Some(stripped) = line.strip_prefix("data:") {
                        stripped
                    } else {
                        continue;
                    };

                    if data == "[DONE]" {
                        break;
                    }

                    let sse: SseEvent = match serde_json::from_str(data) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    match sse.event_type.as_str() {
                        "message_start" => {
                            // Extract input token count from message.usage
                            if let Some(msg) = &sse.message {
                                if let Some(usage) = &msg.usage {
                                    input_tokens = usage.input_tokens.unwrap_or(0);
                                }
                            }
                        }
                        "content_block_start" => {
                            if let Some(block) = &sse.content_block {
                                if block.block_type == "tool_use" {
                                    current_tool_id = block.id.clone();
                                    current_tool_name = block.name.clone();
                                    current_tool_json.clear();
                                }
                            }
                        }
                        "content_block_delta" => {
                            if let Some(delta) = &sse.delta {
                                match delta.delta_type.as_deref() {
                                    Some("text_delta") => {
                                        if let Some(text) = &delta.text {
                                            assembled_content.push_str(text);
                                            if tx.send(StreamEvent::Delta(text.clone())).await.is_err() {
                                                return; // receiver dropped
                                            }
                                        }
                                    }
                                    Some("input_json_delta") => {
                                        if let Some(json_chunk) = &delta.partial_json {
                                            current_tool_json.push_str(json_chunk);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "content_block_stop" => {
                            // Finalize any pending tool call
                            if let (Some(id), Some(name)) = (current_tool_id.take(), current_tool_name.take()) {
                                let args = if current_tool_json.is_empty() {
                                    "{}".to_string()
                                } else {
                                    std::mem::take(&mut current_tool_json)
                                };
                                tool_calls.push(super::LLMToolCall::new(&id, &name, &args));
                            }
                        }
                        "message_delta" => {
                            if let Some(usage) = &sse.usage {
                                output_tokens = usage.output_tokens.unwrap_or(0);
                            }
                        }
                        "message_stop" => {
                            // If we collected tool calls, emit them
                            if !tool_calls.is_empty() {
                                let _ = tx.send(StreamEvent::ToolCalls(
                                    std::mem::take(&mut tool_calls)
                                )).await;
                            }
                            let usage = super::Usage::new(input_tokens, output_tokens);
                            let _ = tx.send(StreamEvent::Done {
                                content: assembled_content.clone(),
                                usage: Some(usage),
                            }).await;
                            return;
                        }
                        _ => {}
                    }
                }
            }

            // Stream ended without message_stop — emit what we have
            if !tool_calls.is_empty() {
                let _ = tx.send(StreamEvent::ToolCalls(
                    std::mem::take(&mut tool_calls)
                )).await;
            }
            let usage = super::Usage::new(input_tokens, output_tokens);
            let _ = tx.send(StreamEvent::Done {
                content: assembled_content,
                usage: Some(usage),
            }).await;
        });

        Ok(rx)
    }
```

**Step 7: Update existing tests that construct `ClaudeRequest` directly**

Any test that creates a `ClaudeRequest` literal needs `stream: None` added. There are 2 tests:
- `test_claude_request_serialization` (around line 692)
- `test_claude_request_without_optional_fields` (around line 719)

Add `stream: None,` to both.

**Step 8: Run all tests**

Run: `cargo test --lib providers::claude -- --nocapture`
Expected: All PASS

**Step 9: Commit**

```bash
git add src/providers/claude.rs
git commit -m "feat: implement Claude SSE streaming in chat_stream()"
```

---

### Task 5: Add streaming branch in agent loop and CLI output

**Files:**
- Modify: `src/agent/loop.rs` (add `streaming` field + streaming branch in `process_message`)
- Modify: `src/main.rs` (pass streaming flag to agent, use streaming in cmd_agent)

**Step 1: Write the failing test**

Add to `src/agent/loop.rs` `mod tests`:

```rust
#[tokio::test]
async fn test_agent_loop_streaming_flag_default() {
    let config = Config::default();
    let session_manager = SessionManager::new_memory();
    let bus = Arc::new(MessageBus::new());
    let agent = AgentLoop::new(config, session_manager, bus);
    assert!(!agent.is_streaming());
}

#[tokio::test]
async fn test_agent_loop_set_streaming() {
    let config = Config::default();
    let session_manager = SessionManager::new_memory();
    let bus = Arc::new(MessageBus::new());
    let agent = AgentLoop::new(config, session_manager, bus);
    agent.set_streaming(true);
    assert!(agent.is_streaming());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib agent::loop_impl::tests::test_agent_loop_streaming -- --nocapture`
Expected: FAIL — methods don't exist

**Step 3: Add `streaming` field and methods to `AgentLoop`**

In `src/agent/loop.rs`, add to the `AgentLoop` struct (after `pending_messages` field, around line 75):

```rust
    /// Whether to stream the final LLM response in CLI mode.
    streaming: AtomicBool,
```

In `AgentLoop::new()` (around line 113), add:

```rust
            streaming: AtomicBool::new(false),
```

In `AgentLoop::with_context_builder()` (around line 142), add:

```rust
            streaming: AtomicBool::new(false),
```

Add these public methods (after the `provider()` method, around line 678):

```rust
    /// Set whether to stream the final LLM response.
    pub fn set_streaming(&self, enabled: bool) {
        self.streaming.store(enabled, Ordering::SeqCst);
    }

    /// Check if streaming is enabled.
    pub fn is_streaming(&self) -> bool {
        self.streaming.load(Ordering::SeqCst)
    }
```

**Step 4: Add streaming branch to `process_message()`**

In `process_message()`, replace the section after the tool loop that handles the final response (lines 406-410):

```rust
        // Add final assistant response
        session.add_message(Message::assistant(&response.content));
        self.session_manager.save(&session).await?;

        Ok(response.content)
```

with:

```rust
        // If streaming is enabled and the final response has no tool calls,
        // re-issue the last LLM call in streaming mode and yield deltas.
        // For non-CLI channels or when streaming is off, return as before.
        let final_content = if self.streaming.load(Ordering::SeqCst) && !response.has_tool_calls() {
            // The response we already have is from a non-streaming call.
            // We use it directly — streaming is handled at the CLI layer
            // by calling chat_stream() instead of chat() on the final call.
            // NOTE: To actually stream, the CLI layer must call process_message_streaming().
            response.content
        } else {
            response.content
        };

        // Add final assistant response
        session.add_message(Message::assistant(&final_content));
        self.session_manager.save(&session).await?;

        Ok(final_content)
```

Also add a new public method `process_message_streaming()` after `process_message()`:

```rust
    /// Process a message with streaming output for the final LLM response.
    ///
    /// This method works like `process_message()` but streams the final response
    /// token-by-token through the returned receiver. Tool loop iterations are
    /// still non-streaming. The assembled final response is returned via
    /// `StreamEvent::Done`.
    ///
    /// # Arguments
    /// * `msg` - The inbound message to process
    ///
    /// # Returns
    /// An mpsc receiver yielding `StreamEvent`s for the final response.
    pub async fn process_message_streaming(
        &self,
        msg: &InboundMessage,
    ) -> Result<tokio::sync::mpsc::Receiver<crate::providers::StreamEvent>> {
        use crate::providers::StreamEvent;

        // Acquire per-session lock
        let session_lock = {
            let mut locks = self.session_locks.lock().await;
            locks
                .entry(msg.session_key.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _session_guard = session_lock.lock().await;

        let provider = {
            let guard = self.provider.read().await;
            Arc::clone(
                guard
                    .as_ref()
                    .ok_or_else(|| ZeptoError::Provider("No provider configured".into()))?,
            )
        };

        let mut session = self.session_manager.get_or_create(&msg.session_key).await?;
        let messages = self
            .context_builder
            .build_messages(session.messages.clone(), &msg.content);

        let tool_definitions = {
            let tools = self.tools.read().await;
            tools.definitions()
        };

        let options = ChatOptions::new()
            .with_max_tokens(self.config.agents.defaults.max_tokens)
            .with_temperature(self.config.agents.defaults.temperature);
        let model = Some(self.config.agents.defaults.model.as_str());

        // First call: non-streaming to see if there are tool calls
        let mut response = provider
            .chat(messages, tool_definitions.clone(), model, options.clone())
            .await?;

        session.add_message(Message::user(&msg.content));

        // Tool loop (non-streaming)
        let max_iterations = self.config.agents.defaults.max_tool_iterations;
        let mut iteration = 0;

        while response.has_tool_calls() && iteration < max_iterations {
            iteration += 1;

            let mut assistant_msg = Message::assistant(&response.content);
            assistant_msg.tool_calls = Some(
                response
                    .tool_calls
                    .iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        arguments: tc.arguments.clone(),
                    })
                    .collect(),
            );
            session.add_message(assistant_msg);

            let workspace = self.config.workspace_path();
            let workspace_str = workspace.to_string_lossy();
            let tool_ctx = ToolContext::new()
                .with_channel(&msg.channel, &msg.chat_id)
                .with_workspace(&workspace_str);

            let tool_futures: Vec<_> = response
                .tool_calls
                .iter()
                .map(|tool_call| {
                    let tools = Arc::clone(&self.tools);
                    let ctx = tool_ctx.clone();
                    let name = tool_call.name.clone();
                    let id = tool_call.id.clone();
                    let raw_args = tool_call.arguments.clone();

                    async move {
                        let args: serde_json::Value = serde_json::from_str(&raw_args)
                            .unwrap_or_else(|_| serde_json::json!({}));
                        let result = {
                            let tools_guard = tools.read().await;
                            match tools_guard.execute_with_context(&name, args, &ctx).await {
                                Ok(r) => r,
                                Err(e) => format!("Error: {}", e),
                            }
                        };
                        let sanitized = crate::utils::sanitize::sanitize_tool_result(
                            &result,
                            crate::utils::sanitize::DEFAULT_MAX_RESULT_BYTES,
                        );
                        (id, sanitized)
                    }
                })
                .collect();

            let results = futures::future::join_all(tool_futures).await;
            for (id, result) in results {
                session.add_message(Message::tool_result(&id, &result));
            }

            let tool_definitions = {
                let tools = self.tools.read().await;
                tools.definitions()
            };

            let messages: Vec<_> = self
                .context_builder
                .build_messages(session.messages.clone(), "")
                .into_iter()
                .filter(|m| !(m.role == Role::User && m.content.is_empty()))
                .collect();

            response = provider
                .chat(messages, tool_definitions, model, options.clone())
                .await?;
        }

        // Final call: if no more tool calls, use streaming
        if !response.has_tool_calls() {
            // Re-issue the final call via chat_stream
            let messages: Vec<_> = self
                .context_builder
                .build_messages(session.messages.clone(), "")
                .into_iter()
                .filter(|m| !(m.role == Role::User && m.content.is_empty()))
                .collect();

            let tool_definitions = {
                let tools = self.tools.read().await;
                tools.definitions()
            };

            let stream_rx = provider
                .chat_stream(messages, tool_definitions, model, options)
                .await?;

            // Wrap in a forwarding task that also saves the session
            let (out_tx, out_rx) = tokio::sync::mpsc::channel::<StreamEvent>(32);
            let session_manager = Arc::clone(&self.session_manager);
            let session_clone = session.clone();

            tokio::spawn(async move {
                let mut session = session_clone;
                let mut stream_rx = stream_rx;

                while let Some(event) = stream_rx.recv().await {
                    match &event {
                        StreamEvent::Done { content, .. } => {
                            session.add_message(Message::assistant(content));
                            let _ = session_manager.save(&session).await;
                            let _ = out_tx.send(event).await;
                            return;
                        }
                        StreamEvent::ToolCalls(_) => {
                            // Unexpected tool calls during streaming — emit and let caller handle
                            let _ = out_tx.send(event).await;
                            return;
                        }
                        _ => {
                            if out_tx.send(event).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            });

            Ok(out_rx)
        } else {
            // Still has tool calls after max iterations — return non-streaming result
            session.add_message(Message::assistant(&response.content));
            self.session_manager.save(&session).await?;

            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let _ = tx
                .send(StreamEvent::Done {
                    content: response.content,
                    usage: response.usage,
                })
                .await;
            Ok(rx)
        }
    }
```

**Step 5: Wire streaming into `cmd_agent()` in `main.rs`**

Replace the single-message handling in `cmd_agent()` (around lines 1109-1120) from:

```rust
    if let Some(msg) = message {
        // Single message mode
        let inbound = InboundMessage::new("cli", "user", "cli", &msg);
        match agent.process_message(&inbound).await {
            Ok(response) => {
                println!("{}", response);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
```

to:

```rust
    if let Some(msg) = message {
        // Single message mode
        let inbound = InboundMessage::new("cli", "user", "cli", &msg);
        let streaming = stream || config.agents.defaults.streaming;

        if streaming {
            use zeptoclaw::providers::StreamEvent;
            match agent.process_message_streaming(&inbound).await {
                Ok(mut rx) => {
                    while let Some(event) = rx.recv().await {
                        match event {
                            StreamEvent::Delta(text) => {
                                print!("{}", text);
                                let _ = io::stdout().flush();
                            }
                            StreamEvent::Done { .. } => break,
                            StreamEvent::Error(e) => {
                                eprintln!("\nStream error: {}", e);
                                std::process::exit(1);
                            }
                            StreamEvent::ToolCalls(_) => {
                                // Tool calls during streaming — shouldn't happen for final response
                                // but handle gracefully
                            }
                        }
                    }
                    println!(); // newline after streaming
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            match agent.process_message(&inbound).await {
                Ok(response) => {
                    println!("{}", response);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
```

Remove the unused `_streaming` variable we added in Task 3.

**Step 6: Also wire streaming into the interactive mode loop**

Replace the interactive mode message handling (around lines 1151-1164) from:

```rust
                    // Process message
                    let inbound = InboundMessage::new("cli", "user", "cli", input);
                    match agent.process_message(&inbound).await {
                        Ok(response) => {
                            println!();
                            println!("{}", response);
                            println!();
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            eprintln!();
                        }
                    }
```

to:

```rust
                    // Process message
                    let inbound = InboundMessage::new("cli", "user", "cli", input);
                    let streaming = stream || config.agents.defaults.streaming;

                    if streaming {
                        use zeptoclaw::providers::StreamEvent;
                        match agent.process_message_streaming(&inbound).await {
                            Ok(mut rx) => {
                                println!();
                                while let Some(event) = rx.recv().await {
                                    match event {
                                        StreamEvent::Delta(text) => {
                                            print!("{}", text);
                                            let _ = io::stdout().flush();
                                        }
                                        StreamEvent::Done { .. } => break,
                                        StreamEvent::Error(e) => {
                                            eprintln!("\nStream error: {}", e);
                                        }
                                        StreamEvent::ToolCalls(_) => {}
                                    }
                                }
                                println!();
                                println!();
                            }
                            Err(e) => {
                                eprintln!("Error: {}", e);
                                eprintln!();
                            }
                        }
                    } else {
                        match agent.process_message(&inbound).await {
                            Ok(response) => {
                                println!();
                                println!("{}", response);
                                println!();
                            }
                            Err(e) => {
                                eprintln!("Error: {}", e);
                                eprintln!();
                            }
                        }
                    }
```

**Step 7: Run tests to verify they pass**

Run: `cargo test --lib agent -- --nocapture`
Expected: All PASS

Run: `cargo check`
Expected: compiles

**Step 8: Commit**

```bash
git add src/agent/loop.rs src/main.rs
git commit -m "feat: add streaming output for CLI mode via process_message_streaming()"
```

---

### Task 6: Update ProviderRef in delegate.rs for chat_stream

**Files:**
- Modify: `src/tools/delegate.rs`

The `ProviderRef` wrapper in `delegate.rs` implements `LLMProvider` but only has `chat()`. Since `chat_stream()` has a default implementation that calls `chat()`, no change is strictly required — the default impl works. But for correctness if someone wants to stream through a delegated agent in the future, we should forward `chat_stream()` too.

**Step 1: Add `chat_stream()` forwarding to ProviderRef**

In `src/tools/delegate.rs`, add after the `chat()` method in `impl LLMProvider for ProviderRef` (around line 249):

```rust
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        model: Option<&str>,
        options: ChatOptions,
    ) -> crate::error::Result<tokio::sync::mpsc::Receiver<crate::providers::StreamEvent>> {
        self.0.chat_stream(messages, tools, model, options).await
    }
```

**Step 2: Run tests**

Run: `cargo test --lib tools::delegate -- --nocapture`
Expected: All PASS

**Step 3: Commit**

```bash
git add src/tools/delegate.rs
git commit -m "feat: forward chat_stream() in ProviderRef for delegate tool"
```

---

### Task 7: Integration tests

**Files:**
- Modify: `tests/integration.rs`

**Step 1: Add integration tests**

Add these tests at the end of `tests/integration.rs`:

```rust
// ============================================================================
// Streaming Configuration Tests
// ============================================================================

#[test]
fn test_streaming_config_default_false() {
    let config = Config::default();
    assert!(!config.agents.defaults.streaming);
}

#[test]
fn test_streaming_config_json_roundtrip() {
    let json = r#"{"agents":{"defaults":{"streaming":true}}}"#;
    let config: Config = serde_json::from_str(json).unwrap();
    assert!(config.agents.defaults.streaming);
}

#[tokio::test]
async fn test_agent_loop_streaming_accessors() {
    let config = Config::default();
    let session_manager = SessionManager::new_memory();
    let bus = Arc::new(MessageBus::new());
    let agent = zeptoclaw::agent::AgentLoop::new(config, session_manager, bus);

    assert!(!agent.is_streaming());
    agent.set_streaming(true);
    assert!(agent.is_streaming());
    agent.set_streaming(false);
    assert!(!agent.is_streaming());
}
```

**Step 2: Run integration tests**

Run: `cargo test --test integration test_streaming -- --nocapture`
Expected: All PASS

**Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add streaming integration tests"
```

---

### Task 8: Final validation

**Step 1: Run all tests**

Run: `cargo test`
Expected: All 490+ tests pass (may have pre-existing flaky `test_load_nonexistent`)

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Run formatter**

Run: `cargo fmt -- --check`
Expected: No formatting issues. If any, run `cargo fmt` to fix.

**Step 4: Update CLAUDE.md if needed**

If test counts changed, update the test count comment in `CLAUDE.md`.

**Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: final cleanup for streaming feature"
```
