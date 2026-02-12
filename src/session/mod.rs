//! Session module - Session and conversation state management
//!
//! This module provides session management for PicoClaw, including:
//! - In-memory session storage with async access
//! - File-based persistence for sessions
//! - Session creation, retrieval, and deletion
//!
//! # Example
//!
//! ```
//! use picoclaw::session::{SessionManager, Message};
//!
//! #[tokio::main]
//! async fn main() {
//!     let manager = SessionManager::new_memory();
//!
//!     // Get or create a session
//!     let mut session = manager.get_or_create("telegram:chat123").await.unwrap();
//!
//!     // Add messages
//!     session.add_message(Message::user("Hello!"));
//!     session.add_message(Message::assistant("Hi there!"));
//!
//!     // Save the session
//!     manager.save(&session).await.unwrap();
//! }
//! ```

pub mod types;

pub use types::{Message, Role, Session, ToolCall};

use crate::config::Config;
use crate::error::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session manager for storing and retrieving conversation sessions.
///
/// The `SessionManager` provides both in-memory caching and optional
/// file-based persistence for sessions. Sessions are identified by
/// unique keys (e.g., "telegram:chat123").
///
/// # Thread Safety
///
/// The manager uses `Arc<RwLock>` internally, making it safe to clone
/// and share across async tasks.
///
/// # Persistence
///
/// When created with `new()`, sessions are persisted to disk in the
/// `~/.picoclaw/sessions/` directory. Use `new_memory()` for testing
/// or when persistence is not needed.
pub struct SessionManager {
    /// In-memory cache of sessions
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Optional path for file-based persistence
    storage_path: Option<PathBuf>,
}

impl SessionManager {
    /// Create a new session manager with file-based persistence.
    ///
    /// Sessions are stored in `~/.picoclaw/sessions/` as JSON files.
    /// The directory is created if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the sessions directory cannot be created.
    ///
    /// # Example
    /// ```no_run
    /// use picoclaw::session::SessionManager;
    ///
    /// let manager = SessionManager::new().unwrap();
    /// ```
    pub fn new() -> Result<Self> {
        let storage_path = Config::dir().join("sessions");
        std::fs::create_dir_all(&storage_path)?;
        Ok(Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Some(storage_path),
        })
    }

    /// Create an in-memory session manager without persistence.
    ///
    /// This is useful for testing or temporary sessions that don't
    /// need to survive application restarts.
    ///
    /// # Example
    /// ```
    /// use picoclaw::session::SessionManager;
    ///
    /// let manager = SessionManager::new_memory();
    /// ```
    pub fn new_memory() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage_path: None,
        }
    }

    /// Create a session manager with a custom storage path.
    ///
    /// # Arguments
    /// * `path` - Directory path for session storage
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    ///
    /// # Example
    /// ```no_run
    /// use picoclaw::session::SessionManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = SessionManager::with_path(PathBuf::from("/tmp/sessions")).unwrap();
    /// ```
    pub fn with_path(path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&path)?;
        Ok(Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            storage_path: Some(path),
        })
    }

    /// Get an existing session or create a new one.
    ///
    /// If the session exists in memory, it is returned immediately.
    /// If persistence is enabled and the session exists on disk, it
    /// is loaded into memory. Otherwise, a new empty session is created.
    ///
    /// # Arguments
    /// * `key` - Unique session identifier
    ///
    /// # Errors
    ///
    /// Returns an error if loading from disk fails.
    ///
    /// # Example
    /// ```
    /// use picoclaw::session::SessionManager;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = SessionManager::new_memory();
    ///     let session = manager.get_or_create("test-session").await.unwrap();
    ///     assert_eq!(session.key, "test-session");
    /// }
    /// ```
    pub async fn get_or_create(&self, key: &str) -> Result<Session> {
        // Check in-memory cache first
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(key) {
                return Ok(session.clone());
            }
        }

        // Try loading from disk if persistence is enabled
        if let Some(ref storage_path) = self.storage_path {
            let file_path = storage_path.join(format!("{}.json", Self::sanitize_key(key)));
            if file_path.exists() {
                let content = tokio::fs::read_to_string(&file_path).await?;
                let session: Session = serde_json::from_str(&content)?;

                // Cache it in memory
                let mut sessions = self.sessions.write().await;
                sessions.insert(key.to_string(), session.clone());
                return Ok(session);
            }
        }

        // Create new session
        let session = Session::new(key);
        let mut sessions = self.sessions.write().await;
        sessions.insert(key.to_string(), session.clone());
        Ok(session)
    }

    /// Get a session by key without creating it.
    ///
    /// # Arguments
    /// * `key` - Unique session identifier
    ///
    /// # Returns
    ///
    /// `Some(Session)` if found, `None` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if loading from disk fails.
    pub async fn get(&self, key: &str) -> Result<Option<Session>> {
        // Check in-memory cache first
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(key) {
                return Ok(Some(session.clone()));
            }
        }

        // Try loading from disk if persistence is enabled
        if let Some(ref storage_path) = self.storage_path {
            let file_path = storage_path.join(format!("{}.json", Self::sanitize_key(key)));
            if file_path.exists() {
                let content = tokio::fs::read_to_string(&file_path).await?;
                let session: Session = serde_json::from_str(&content)?;

                // Cache it in memory
                let mut sessions = self.sessions.write().await;
                sessions.insert(key.to_string(), session.clone());
                return Ok(Some(session));
            }
        }

        Ok(None)
    }

    /// Save a session to both memory and disk (if persistence is enabled).
    ///
    /// # Arguments
    /// * `session` - The session to save
    ///
    /// # Errors
    ///
    /// Returns an error if writing to disk fails.
    ///
    /// # Example
    /// ```
    /// use picoclaw::session::{SessionManager, Message};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = SessionManager::new_memory();
    ///     let mut session = manager.get_or_create("test").await.unwrap();
    ///     session.add_message(Message::user("Hello"));
    ///     manager.save(&session).await.unwrap();
    /// }
    /// ```
    pub async fn save(&self, session: &Session) -> Result<()> {
        // Update in-memory cache
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session.key.clone(), session.clone());
        }

        // Write to disk if persistence is enabled
        if let Some(ref storage_path) = self.storage_path {
            let file_path = storage_path.join(format!("{}.json", Self::sanitize_key(&session.key)));
            let content = serde_json::to_string_pretty(session)?;
            tokio::fs::write(&file_path, content).await?;
        }

        Ok(())
    }

    /// Delete a session from both memory and disk.
    ///
    /// # Arguments
    /// * `key` - Unique session identifier
    ///
    /// # Errors
    ///
    /// Returns an error if deleting from disk fails.
    ///
    /// # Example
    /// ```
    /// use picoclaw::session::SessionManager;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = SessionManager::new_memory();
    ///     manager.get_or_create("test").await.unwrap();
    ///     manager.delete("test").await.unwrap();
    /// }
    /// ```
    pub async fn delete(&self, key: &str) -> Result<()> {
        // Remove from memory
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(key);
        }

        // Remove from disk if persistence is enabled
        if let Some(ref storage_path) = self.storage_path {
            let file_path = storage_path.join(format!("{}.json", Self::sanitize_key(key)));
            if file_path.exists() {
                tokio::fs::remove_file(&file_path).await?;
            }
        }

        Ok(())
    }

    /// List all session keys.
    ///
    /// Returns session keys from both memory and disk (if persistence is enabled).
    /// Duplicate keys are not included.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the storage directory fails.
    ///
    /// # Example
    /// ```
    /// use picoclaw::session::SessionManager;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let manager = SessionManager::new_memory();
    ///     manager.get_or_create("session1").await.unwrap();
    ///     manager.get_or_create("session2").await.unwrap();
    ///
    ///     let keys = manager.list().await.unwrap();
    ///     assert_eq!(keys.len(), 2);
    /// }
    /// ```
    pub async fn list(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        // Get keys from memory
        {
            let sessions = self.sessions.read().await;
            keys.extend(sessions.keys().cloned());
        }

        // Get keys from disk if persistence is enabled
        if let Some(ref storage_path) = self.storage_path {
            let mut dir_entries = tokio::fs::read_dir(storage_path).await?;
            while let Some(entry) = dir_entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Some(stem) = path.file_stem() {
                        let key = stem.to_string_lossy().to_string();
                        if !keys.contains(&key) {
                            keys.push(key);
                        }
                    }
                }
            }
        }

        keys.sort();
        Ok(keys)
    }

    /// Check if a session exists.
    ///
    /// # Arguments
    /// * `key` - Unique session identifier
    ///
    /// # Returns
    ///
    /// `true` if the session exists in memory or on disk.
    pub async fn exists(&self, key: &str) -> bool {
        // Check memory
        {
            let sessions = self.sessions.read().await;
            if sessions.contains_key(key) {
                return true;
            }
        }

        // Check disk
        if let Some(ref storage_path) = self.storage_path {
            let file_path = storage_path.join(format!("{}.json", Self::sanitize_key(key)));
            return file_path.exists();
        }

        false
    }

    /// Clear all sessions from memory (does not affect disk).
    ///
    /// Use this to free memory while keeping persisted sessions.
    pub async fn clear_cache(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.clear();
    }

    /// Get the number of sessions in memory.
    pub async fn cache_size(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    /// Sanitize a session key for use as a filename.
    ///
    /// Replaces characters that are invalid in filenames with underscores.
    fn sanitize_key(key: &str) -> String {
        key.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
    }
}

impl Clone for SessionManager {
    fn clone(&self) -> Self {
        Self {
            sessions: Arc::clone(&self.sessions),
            storage_path: self.storage_path.clone(),
        }
    }
}

impl Default for SessionManager {
    /// Creates an in-memory session manager.
    ///
    /// Use `SessionManager::new()` for file-based persistence.
    fn default() -> Self {
        Self::new_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_create_and_retrieve() {
        let manager = SessionManager::new_memory();
        let session = manager.get_or_create("test-session").await.unwrap();
        assert!(session.messages.is_empty());
        assert_eq!(session.key, "test-session");
    }

    #[tokio::test]
    async fn test_session_save_and_load() {
        let manager = SessionManager::new_memory();
        let mut session = manager.get_or_create("test-session").await.unwrap();
        session.add_message(Message::user("Hello"));
        manager.save(&session).await.unwrap();

        let loaded = manager.get_or_create("test-session").await.unwrap();
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "Hello");
    }

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = Message::assistant("Hi there");
        assert_eq!(assistant_msg.role, Role::Assistant);
        assert_eq!(assistant_msg.content, "Hi there");

        let system_msg = Message::system("You are helpful");
        assert_eq!(system_msg.role, Role::System);

        let tool_msg = Message::tool_result("call_1", "Success");
        assert_eq!(tool_msg.role, Role::Tool);
        assert_eq!(tool_msg.tool_call_id, Some("call_1".to_string()));
    }

    #[tokio::test]
    async fn test_session_delete() {
        let manager = SessionManager::new_memory();
        manager.get_or_create("test-session").await.unwrap();
        assert!(manager.exists("test-session").await);

        manager.delete("test-session").await.unwrap();
        assert!(!manager.exists("test-session").await);
    }

    #[tokio::test]
    async fn test_session_list() {
        let manager = SessionManager::new_memory();
        manager.get_or_create("session-a").await.unwrap();
        manager.get_or_create("session-b").await.unwrap();
        manager.get_or_create("session-c").await.unwrap();

        let keys = manager.list().await.unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"session-a".to_string()));
        assert!(keys.contains(&"session-b".to_string()));
        assert!(keys.contains(&"session-c".to_string()));
    }

    #[tokio::test]
    async fn test_session_get_nonexistent() {
        let manager = SessionManager::new_memory();
        let result = manager.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_clone() {
        let manager1 = SessionManager::new_memory();
        let manager2 = manager1.clone();

        // Create session with manager1
        let mut session = manager1.get_or_create("shared").await.unwrap();
        session.add_message(Message::user("Test"));
        manager1.save(&session).await.unwrap();

        // Should be visible from manager2
        let loaded = manager2.get("shared").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().messages.len(), 1);
    }

    #[tokio::test]
    async fn test_session_clear_cache() {
        let manager = SessionManager::new_memory();
        manager.get_or_create("session1").await.unwrap();
        manager.get_or_create("session2").await.unwrap();

        assert_eq!(manager.cache_size().await, 2);

        manager.clear_cache().await;
        assert_eq!(manager.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_file_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        // Create manager and save a session
        {
            let manager = SessionManager::with_path(storage_path.clone()).unwrap();
            let mut session = manager.get_or_create("persist-test").await.unwrap();
            session.add_message(Message::user("Persisted message"));
            manager.save(&session).await.unwrap();
        }

        // Create new manager instance and load the session
        {
            let manager = SessionManager::with_path(storage_path.clone()).unwrap();
            let session = manager.get_or_create("persist-test").await.unwrap();
            assert_eq!(session.messages.len(), 1);
            assert_eq!(session.messages[0].content, "Persisted message");
        }
    }

    #[tokio::test]
    async fn test_file_persistence_delete() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let manager = SessionManager::with_path(storage_path.clone()).unwrap();

        // Create and save
        let session = manager.get_or_create("delete-test").await.unwrap();
        manager.save(&session).await.unwrap();

        // Verify file exists
        let file_path = storage_path.join("delete-test.json");
        assert!(file_path.exists());

        // Delete
        manager.delete("delete-test").await.unwrap();

        // Verify file is gone
        assert!(!file_path.exists());
    }

    #[tokio::test]
    async fn test_file_persistence_list() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().to_path_buf();

        let manager = SessionManager::with_path(storage_path).unwrap();

        // Create and save multiple sessions
        for name in ["alpha", "beta", "gamma"] {
            let session = manager.get_or_create(name).await.unwrap();
            manager.save(&session).await.unwrap();
        }

        // Clear cache to force disk reads
        manager.clear_cache().await;

        let keys = manager.list().await.unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"alpha".to_string()));
        assert!(keys.contains(&"beta".to_string()));
        assert!(keys.contains(&"gamma".to_string()));
    }

    #[test]
    fn test_sanitize_key() {
        assert_eq!(SessionManager::sanitize_key("simple"), "simple");
        assert_eq!(SessionManager::sanitize_key("telegram:chat123"), "telegram_chat123");
        assert_eq!(SessionManager::sanitize_key("path/to/session"), "path_to_session");
        assert_eq!(
            SessionManager::sanitize_key("a:b/c\\d*e?f\"g<h>i|j"),
            "a_b_c_d_e_f_g_h_i_j"
        );
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let manager = Arc::new(SessionManager::new_memory());
        let mut handles = Vec::new();

        // Spawn multiple tasks accessing the same session
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = tokio::spawn(async move {
                let mut session = manager_clone.get_or_create("concurrent").await.unwrap();
                session.add_message(Message::user(&format!("Message {}", i)));
                manager_clone.save(&session).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Session should exist with some messages (exact count depends on race conditions)
        let session = manager.get("concurrent").await.unwrap().unwrap();
        assert!(!session.messages.is_empty());
    }

    #[tokio::test]
    async fn test_session_with_all_message_types() {
        let manager = SessionManager::new_memory();
        let mut session = manager.get_or_create("all-types").await.unwrap();

        // Add all message types
        session.add_message(Message::system("You are a helpful assistant"));
        session.add_message(Message::user("Search for rust programming"));
        session.add_message(Message::assistant_with_tools(
            "Let me search for that.",
            vec![ToolCall::new("call_1", "search", r#"{"q": "rust"}"#)],
        ));
        session.add_message(Message::tool_result("call_1", "Found 100 results"));
        session.add_message(Message::assistant("I found 100 results about Rust."));

        manager.save(&session).await.unwrap();

        let loaded = manager.get_or_create("all-types").await.unwrap();
        assert_eq!(loaded.messages.len(), 5);
        assert_eq!(loaded.messages[0].role, Role::System);
        assert_eq!(loaded.messages[1].role, Role::User);
        assert_eq!(loaded.messages[2].role, Role::Assistant);
        assert!(loaded.messages[2].has_tool_calls());
        assert_eq!(loaded.messages[3].role, Role::Tool);
        assert!(loaded.messages[3].is_tool_result());
        assert_eq!(loaded.messages[4].role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_session_default() {
        let manager = SessionManager::default();
        let session = manager.get_or_create("test").await.unwrap();
        assert!(session.is_empty());
    }
}
