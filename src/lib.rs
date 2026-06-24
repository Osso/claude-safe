use std::{env, process::Stdio};
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

fn joined_disallowed_tools() -> String {
    DISALLOWED_TOOLS.join(",")
}

pub fn claude_print_args(model: &str, output_format: &str) -> Vec<String> {
    vec![
        "-p".to_string(),
        "-".to_string(),
        "--output-format".to_string(),
        output_format.to_string(),
        "--model".to_string(),
        model.to_string(),
        "--disallowedTools".to_string(),
        joined_disallowed_tools(),
        "--no-session-persistence".to_string(),
    ]
}

pub fn safe_cli_args(args: &[String]) -> Vec<String> {
    let mut safe_args = vec!["--disallowedTools".to_string(), joined_disallowed_tools()];

    if args.iter().any(|arg| arg == "-p" || arg == "--print") {
        safe_args.push("--no-session-persistence".to_string());
    }

    safe_args.extend(args.iter().cloned());
    safe_args
}

/// Call Claude CLI with safety restrictions (no MCP tools, no session persistence)
/// Returns the raw output from Claude
pub async fn call(prompt: &str, model: &str, output_format: &str) -> Result<String, String> {
    let executable = env::var("CLAUDE_SAFE_CLAUDE").unwrap_or_else(|_| "claude".to_string());
    call_executable(&executable, prompt, model, output_format).await
}

async fn call_executable(
    executable: &str,
    prompt: &str,
    model: &str,
    output_format: &str,
) -> Result<String, String> {
    let mut child = Command::new(executable)
        .args(claude_print_args(model, output_format))
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
    joined_disallowed_tools()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::future::Future;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct FakeClaude {
        dir: PathBuf,
        executable: PathBuf,
        stdin_path: PathBuf,
    }

    impl FakeClaude {
        fn new(name: &str, script: &str) -> Self {
            let dir = env::temp_dir().join(format!(
                "claude-safe-lib-test-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("create fake claude dir");

            let executable = dir.join("claude");
            let stdin_path = dir.join("stdin");
            let script = script.replace("{stdin_path}", &stdin_path.display().to_string());
            fs::write(&executable, script).expect("write fake claude");

            let mut permissions = fs::metadata(&executable)
                .expect("read fake claude metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&executable, permissions).expect("make fake claude executable");
            std::thread::sleep(std::time::Duration::from_millis(10));

            Self {
                dir,
                executable,
                stdin_path,
            }
        }

        fn path(&self) -> &Path {
            &self.executable
        }

        fn stdin_path(&self) -> &Path {
            &self.stdin_path
        }
    }

    impl Drop for FakeClaude {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn block_on<T>(future: impl Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .expect("build tokio runtime")
            .block_on(future)
    }

    fn split_disallowed_tools() -> Vec<String> {
        disallowed_tools().split(',').map(str::to_string).collect()
    }

    #[test]
    fn disallowed_tools_include_external_surfaces() {
        let tools = split_disallowed_tools();

        assert!(tools.contains(&"Bash".to_string()));
        assert!(tools.contains(&"Read".to_string()));
        assert!(tools.contains(&"Task".to_string()));
        assert!(tools.contains(&"mcp__browsermcp__browser_navigate".to_string()));
        assert!(tools.contains(&"mcp__claude-in-chrome__javascript_tool".to_string()));
        assert!(tools.contains(&"mcp__persistent-shell__execute".to_string()));
        assert!(tools.contains(&"mcp__claude-memory__memory_write".to_string()));
    }

    #[test]
    fn disallowed_tools_are_comma_separated_without_empty_entries() {
        let tools = split_disallowed_tools();

        assert_eq!(tools.len(), DISALLOWED_TOOLS.len());
        assert!(tools.iter().all(|tool| !tool.is_empty()));
        assert!(!disallowed_tools().contains(",,"));
    }

    #[test]
    fn claude_print_args_force_safe_print_mode() {
        assert_eq!(
            claude_print_args("haiku", "json"),
            vec![
                "-p",
                "-",
                "--output-format",
                "json",
                "--model",
                "haiku",
                "--disallowedTools",
                &disallowed_tools(),
                "--no-session-persistence",
            ]
        );
    }

    #[test]
    fn safe_cli_args_add_no_session_persistence_only_for_print_mode() {
        let args = vec!["--print".to_string(), "hello".to_string()];
        let safe_args = safe_cli_args(&args);

        assert_eq!(safe_args[0], "--disallowedTools");
        assert_eq!(safe_args[1], disallowed_tools());
        assert!(safe_args.contains(&"--no-session-persistence".to_string()));
        assert!(safe_args.ends_with(&args));
    }

    #[test]
    fn safe_cli_args_preserve_interactive_session_for_non_print_mode() {
        let args = vec!["chat".to_string()];
        let safe_args = safe_cli_args(&args);

        assert_eq!(safe_args[0], "--disallowedTools");
        assert_eq!(safe_args[1], disallowed_tools());
        assert!(!safe_args.contains(&"--no-session-persistence".to_string()));
        assert!(safe_args.ends_with(&args));
    }

    #[test]
    fn call_executable_writes_prompt_and_trims_stdout() {
        let fake = FakeClaude::new(
            "success",
            "#!/bin/sh\ncat > \"{stdin_path}\"\nprintf '  answer  \\n'\n",
        );

        let result = block_on(call_executable(
            fake.path().to_str().expect("fake path is utf-8"),
            "prompt body",
            "sonnet",
            "text",
        ));

        assert_eq!(result, Ok("answer".to_string()));
        assert_eq!(
            fs::read_to_string(fake.stdin_path()).expect("read captured stdin"),
            "prompt body"
        );
    }

    #[test]
    fn call_executable_reports_non_zero_exit() {
        let fake = FakeClaude::new("failure", "#!/bin/sh\nprintf 'bad request' >&2\nexit 4\n");

        let error = block_on(call_executable(
            fake.path().to_str().expect("fake path is utf-8"),
            "prompt",
            "haiku",
            "json",
        ))
        .expect_err("fake claude should fail");

        assert_eq!(error, "claude CLI failed: bad request");
    }

    #[test]
    fn call_executable_reports_spawn_failure() {
        let error = block_on(call_executable(
            "/definitely/missing/claude-safe-test/claude",
            "prompt",
            "haiku",
            "text",
        ))
        .expect_err("missing executable should fail");

        assert!(error.starts_with("Failed to spawn claude CLI: "));
    }

    #[test]
    fn public_call_helpers_use_configured_claude_executable() {
        let _guard = ENV_LOCK.lock().expect("lock test env");
        let fake = FakeClaude::new("public", "#!/bin/sh\nprintf '%s\\n' \"$*\"\n");

        unsafe {
            env::set_var("CLAUDE_SAFE_CLAUDE", fake.path());
        }

        let custom = block_on(call("prompt", "opus", "stream-json")).expect("custom call");
        let haiku = block_on(call_haiku("prompt")).expect("haiku call");
        let haiku_json = block_on(call_haiku_json("prompt")).expect("haiku json call");
        let sonnet = block_on(call_sonnet("prompt")).expect("sonnet call");

        unsafe {
            env::remove_var("CLAUDE_SAFE_CLAUDE");
        }

        assert!(custom.contains("--model opus"));
        assert!(custom.contains("--output-format stream-json"));
        assert!(haiku.contains("--model haiku"));
        assert!(haiku.contains("--output-format text"));
        assert!(haiku_json.contains("--model haiku"));
        assert!(haiku_json.contains("--output-format json"));
        assert!(sonnet.contains("--model sonnet"));
        assert!(sonnet.contains("--output-format text"));
    }
}
