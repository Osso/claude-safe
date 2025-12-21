use std::env;
use std::io::{self, Read};
use std::process::{Command, Stdio};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    // Read stdin if available
    let mut stdin_content = String::new();
    if atty::isnt(atty::Stream::Stdin) {
        io::stdin().read_to_string(&mut stdin_content).ok();
    }

    // Build disallowed tools list (all MCP browser tools)
    let disallowed = [
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
    ]
    .join(",");

    let mut cmd = Command::new("claude");

    // Add safety flags
    cmd.arg("--disallowedTools").arg(&disallowed);
    cmd.arg("--no-session-persistence");

    // Pass through all user arguments
    cmd.args(&args);

    // Handle stdin
    if !stdin_content.is_empty() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::inherit());
    }

    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let mut child = cmd.spawn().expect("Failed to spawn claude");

    // Write stdin content if we have it
    if !stdin_content.is_empty() {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(stdin_content.as_bytes()).ok();
        }
    }

    let status = child.wait().expect("Failed to wait on claude");
    std::process::exit(status.code().unwrap_or(1));
}
