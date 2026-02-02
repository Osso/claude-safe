use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Disallowed tools for safe execution (web-only mode)
const DISALLOWED_TOOLS: &[&str] = &[
    // Shell execution
    "Bash",
    // File operations
    "Read",
    "Write",
    "Edit",
    "NotebookEdit",
    "Glob",
    "Grep",
    // Code intelligence
    "LSP",
    // Agent/skill execution
    "Task",
    "Skill",
    // Process/task management
    "KillShell",
    "TaskOutput",
    "TaskStop",
    // Browser MCP (browsermcp)
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
    // Browser MCP (claude-in-chrome)
    "mcp__claude-in-chrome__javascript_tool",
    "mcp__claude-in-chrome__read_page",
    "mcp__claude-in-chrome__find",
    "mcp__claude-in-chrome__form_input",
    "mcp__claude-in-chrome__computer",
    "mcp__claude-in-chrome__navigate",
    "mcp__claude-in-chrome__resize_window",
    "mcp__claude-in-chrome__gif_creator",
    "mcp__claude-in-chrome__upload_image",
    "mcp__claude-in-chrome__get_page_text",
    "mcp__claude-in-chrome__tabs_context_mcp",
    "mcp__claude-in-chrome__tabs_create_mcp",
    "mcp__claude-in-chrome__update_plan",
    "mcp__claude-in-chrome__read_console_messages",
    "mcp__claude-in-chrome__read_network_requests",
    "mcp__claude-in-chrome__shortcuts_list",
    "mcp__claude-in-chrome__shortcuts_execute",
    // Nushell MCP
    "mcp__nushell__execute",
    // Persistent shell MCP
    "mcp__persistent-shell__list_sessions",
    "mcp__persistent-shell__execute",
    "mcp__persistent-shell__create_session",
    "mcp__persistent-shell__close_session",
    // Memory MCP
    "mcp__claude-memory__prompt_search",
    "mcp__claude-memory__answer_search",
    "mcp__claude-memory__memory_write",
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
