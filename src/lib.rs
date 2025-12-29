use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Disallowed tools for safe execution (read-only mode)
const DISALLOWED_TOOLS: &[&str] = &[
    // Shell execution
    "Bash",
    // File modification
    "Write",
    "Edit",
    "NotebookEdit",
    // Agent/skill execution
    "Task",
    "Skill",
    // Process management
    "KillShell",
    // Browser MCP
    "mcp__browsermcp__browser_navigate",
    "mcp__browsermcp__browser_click",
    "mcp__browsermcp__browser_snapshot",
    "mcp__browsermcp__browser_screenshot",
    "mcp__browsermcp__browser_wait",
    "mcp__browsermcp__browser_hover",
    "mcp__browsermcp__browser_type",
    "mcp__browsermcp__browser_select_option",
    "mcp__browsermcp__browser_press_key",
    "mcp__browsermcp__browser_go_back",
    "mcp__browsermcp__browser_go_forward",
    "mcp__browsermcp__browser_get_console_logs",
    // Nushell MCP
    "mcp__nushell__execute",
    // Persistent shell MCP
    "mcp__persistent-shell__list_sessions",
    "mcp__persistent-shell__execute",
    "mcp__persistent-shell__create_session",
    "mcp__persistent-shell__close_session",
];

/// Call Claude CLI with safety restrictions (no MCP tools, no session persistence)
/// Returns the raw output from Claude
pub async fn call(prompt: &str, model: &str, output_format: &str) -> Result<String, String> {
    let disallowed = DISALLOWED_TOOLS.join(",");

    let mut child = Command::new("claude")
        .args([
            "-p", "-",
            "--output-format", output_format,
            "--model", model,
            "--disallowedTools", &disallowed,
            "--no-session-persistence",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn claude CLI: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to run claude CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("claude CLI failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Call Claude with haiku model and text output (convenience function)
pub async fn call_haiku(prompt: &str) -> Result<String, String> {
    call(prompt, "haiku", "text").await
}

/// Call Claude with haiku model and JSON output
pub async fn call_haiku_json(prompt: &str) -> Result<String, String> {
    call(prompt, "haiku", "json").await
}

/// Call Claude with sonnet model and text output
pub async fn call_sonnet(prompt: &str) -> Result<String, String> {
    call(prompt, "sonnet", "text").await
}

/// Returns the disallowed tools list for manual Command building
pub fn disallowed_tools() -> String {
    DISALLOWED_TOOLS.join(",")
}
