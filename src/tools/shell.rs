//! Shell tool for PicoClaw
//!
//! This module provides a tool for executing shell commands. Commands are run
//! in a subprocess with configurable timeout and workspace directory support.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use crate::error::{PicoError, Result};

use super::{Tool, ToolContext};

/// Tool for executing shell commands.
///
/// Executes a shell command and returns the combined stdout and stderr output.
/// Commands are run using `sh -c` for shell interpretation.
///
/// # Parameters
/// - `command`: The shell command to execute (required)
/// - `timeout`: Timeout in seconds, defaults to 60 (optional)
///
/// # Security Note
/// This tool executes arbitrary shell commands. It should be used with caution
/// and appropriate safeguards in production environments.
///
/// # Example
/// ```rust
/// use picoclaw::tools::{Tool, ToolContext};
/// use picoclaw::tools::shell::ShellTool;
/// use serde_json::json;
///
/// # tokio_test::block_on(async {
/// let tool = ShellTool;
/// let ctx = ToolContext::new();
/// let result = tool.execute(json!({"command": "echo hello"}), &ctx).await;
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().trim(), "hello");
/// # });
/// ```
pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return the output"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 60)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<String> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PicoError::Tool("Missing 'command' argument".into()))?;

        let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60);

        // Build the command
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        // Set working directory if workspace is specified
        if let Some(ref workspace) = ctx.workspace {
            cmd.current_dir(workspace);
        }

        // Capture output
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Execute with timeout
        let output = tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output())
            .await
            .map_err(|_| PicoError::Tool(format!("Command timed out after {}s", timeout_secs)))?
            .map_err(|e| PicoError::Tool(format!("Failed to execute command: {}", e)))?;

        // Build result string
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();

        if !stdout.is_empty() {
            result.push_str(&stdout);
        }

        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push_str("\n--- stderr ---\n");
            }
            result.push_str(&stderr);
        }

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            result.push_str(&format!("\n[Exit code: {}]", exit_code));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_shell_echo() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool.execute(json!({"command": "echo hello"}), &ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[tokio::test]
    async fn test_shell_multiple_commands() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "echo first && echo second"}), &ctx)
            .await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("first"));
        assert!(output.contains("second"));
    }

    #[tokio::test]
    async fn test_shell_with_workspace() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("test.txt"), "workspace file").unwrap();

        let tool = ShellTool;
        let ctx = ToolContext::new().with_workspace(dir.path().to_str().unwrap());

        let result = tool.execute(json!({"command": "cat test.txt"}), &ctx).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "workspace file");
    }

    #[tokio::test]
    async fn test_shell_pwd_with_workspace() {
        let dir = tempdir().unwrap();

        let tool = ShellTool;
        let ctx = ToolContext::new().with_workspace(dir.path().to_str().unwrap());

        let result = tool.execute(json!({"command": "pwd"}), &ctx).await;
        assert!(result.is_ok());

        // The output should contain the temp directory path
        let output = result.unwrap();
        // On macOS, /tmp is symlinked to /private/tmp, so we compare canonical paths
        let expected = dir.path().canonicalize().unwrap();
        let actual_path = std::path::Path::new(output.trim());
        let actual = actual_path.canonicalize().unwrap_or_else(|_| actual_path.to_path_buf());
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_shell_stderr() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "echo error >&2"}), &ctx)
            .await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("error"));
    }

    #[tokio::test]
    async fn test_shell_combined_output() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(
                json!({"command": "echo stdout && echo stderr >&2"}),
                &ctx,
            )
            .await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("stdout"));
        assert!(output.contains("stderr"));
        assert!(output.contains("--- stderr ---"));
    }

    #[tokio::test]
    async fn test_shell_exit_code() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool.execute(json!({"command": "exit 42"}), &ctx).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("[Exit code: 42]"));
    }

    #[tokio::test]
    async fn test_shell_failed_command() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "ls /nonexistent_picoclaw_path"}), &ctx)
            .await;
        assert!(result.is_ok()); // The tool returns Ok with error in output
        let output = result.unwrap();
        assert!(output.contains("Exit code:") || output.contains("No such file"));
    }

    #[tokio::test]
    async fn test_shell_missing_command() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool.execute(json!({}), &ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'command'"));
    }

    #[tokio::test]
    async fn test_shell_timeout() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "sleep 10", "timeout": 1}), &ctx)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_shell_custom_timeout_success() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "sleep 0.1 && echo done", "timeout": 5}), &ctx)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("done"));
    }

    #[tokio::test]
    async fn test_shell_environment_variables() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "MY_VAR=hello && echo $MY_VAR"}), &ctx)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello"));
    }

    #[tokio::test]
    async fn test_shell_piped_commands() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "echo 'hello world' | tr ' ' '-'"}), &ctx)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello-world");
    }

    #[tokio::test]
    async fn test_shell_special_characters() {
        let tool = ShellTool;
        let ctx = ToolContext::new();

        let result = tool
            .execute(json!({"command": "echo \"hello 'world'\""}) , &ctx)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello 'world'"));
    }

    #[test]
    fn test_shell_tool_name() {
        assert_eq!(ShellTool.name(), "shell");
    }

    #[test]
    fn test_shell_tool_description() {
        assert!(!ShellTool.description().is_empty());
        assert!(ShellTool.description().contains("shell"));
    }

    #[test]
    fn test_shell_tool_parameters() {
        let params = ShellTool.parameters();
        assert!(params.is_object());
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["command"].is_object());
        assert!(params["properties"]["timeout"].is_object());
        assert_eq!(params["required"][0], "command");
    }
}
