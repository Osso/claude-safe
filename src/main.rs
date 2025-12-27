use std::env;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    // Read stdin if available
    let mut stdin_content = String::new();
    let has_stdin = !atty::is(atty::Stream::Stdin);
    if has_stdin {
        io::stdin().read_to_string(&mut stdin_content).ok();
    }

    let disallowed = claude_safe::disallowed_tools();
    let is_print_mode = args.iter().any(|a| a == "-p" || a == "--print");

    let mut cmd = Command::new("claude");
    cmd.arg("--disallowedTools").arg(&disallowed);
    if is_print_mode {
        cmd.arg("--no-session-persistence");
    }
    cmd.args(&args);

    if !stdin_content.is_empty() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::inherit());
    }

    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let mut child = cmd.spawn().expect("Failed to spawn claude");

    if !stdin_content.is_empty() {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(stdin_content.as_bytes()).ok();
        }
    }

    let status = child.wait().expect("Failed to wait on claude");
    std::process::exit(status.code().unwrap_or(1));
}
