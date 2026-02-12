//! Agent module - Core AI agent logic and conversation handling
//!
//! This module provides the core agent loop and context building functionality
//! for PicoClaw. The agent is responsible for:
//!
//! - Processing inbound messages from channels
//! - Building conversation context with system prompts and history
//! - Calling LLM providers for responses
//! - Executing tool calls and feeding results back to the LLM
//! - Managing conversation sessions
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │  MessageBus │────>│  AgentLoop  │────>│ LLMProvider │
//! │  (inbound)  │     │             │     │  (Claude)   │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!                            │                   │
//!                            │                   │
//!                            ▼                   ▼
//!                     ┌─────────────┐     ┌─────────────┐
//!                     │   Session   │     │    Tools    │
//!                     │   Manager   │     │  Registry   │
//!                     └─────────────┘     └─────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use picoclaw::agent::AgentLoop;
//! use picoclaw::bus::MessageBus;
//! use picoclaw::config::Config;
//! use picoclaw::session::SessionManager;
//! use picoclaw::providers::ClaudeProvider;
//! use picoclaw::tools::EchoTool;
//!
//! async fn run_agent() {
//!     let config = Config::default();
//!     let session_manager = SessionManager::new_memory();
//!     let bus = Arc::new(MessageBus::new());
//!     let agent = AgentLoop::new(config, session_manager, bus);
//!
//!     // Configure provider
//!     let provider = ClaudeProvider::new("your-api-key");
//!     agent.set_provider(Box::new(provider)).await;
//!
//!     // Register tools
//!     agent.register_tool(Box::new(EchoTool)).await;
//!
//!     // Start the agent loop
//!     agent.start().await.unwrap();
//! }
//! ```

mod context;
mod r#loop;

pub use context::ContextBuilder;
pub use r#loop::AgentLoop;
