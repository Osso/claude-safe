use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct FakeClaude {
    dir: PathBuf,
    args_path: PathBuf,
    stdin_path: PathBuf,
}

impl FakeClaude {
    fn new(name: &str) -> Self {
        let dir = env::temp_dir().join(format!("claude-safe-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create fake claude dir");

        let executable = dir.join("claude");
        fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CLAUDE_SAFE_ARGS_OUT\"\ncat > \"$CLAUDE_SAFE_STDIN_OUT\"\nexit \"${CLAUDE_SAFE_EXIT:-0}\"\n",
        )
        .expect("write fake claude");

        let mut permissions = fs::metadata(&executable)
            .expect("read fake claude metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make fake claude executable");

        Self {
            args_path: dir.join("args"),
            stdin_path: dir.join("stdin"),
            dir,
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_claude-safe"));
        let path = env::var("PATH").expect("PATH is set");

        command
            .env("PATH", format!("{}:{path}", self.dir.display()))
            .env("CLAUDE_SAFE_ARGS_OUT", &self.args_path)
            .env("CLAUDE_SAFE_STDIN_OUT", &self.stdin_path);

        command
    }

    fn args(&self) -> Vec<String> {
        fs::read_to_string(&self.args_path)
            .expect("read captured args")
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn stdin(&self) -> String {
        fs::read_to_string(&self.stdin_path).expect("read captured stdin")
    }
}

impl Drop for FakeClaude {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn print_mode_forwards_stdin_and_blocks_sessions() {
    let fake = FakeClaude::new("print");
    let mut child = fake
        .command()
        .arg("--print")
        .arg("summarize")
        .stdin(Stdio::piped())
        .spawn()
        .expect("spawn claude-safe");

    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(b"hello from stdin")
        .expect("write stdin");

    let status = child.wait().expect("wait for claude-safe");

    assert!(status.success());
    assert!(
        fake.args()
            .contains(&"--no-session-persistence".to_string())
    );
    assert!(fake.args().contains(&"--disallowedTools".to_string()));
    assert!(
        fake.args()
            .ends_with(&["--print".to_string(), "summarize".to_string()])
    );
    assert_eq!(fake.stdin(), "hello from stdin");
}

#[test]
fn non_print_mode_keeps_session_persistence_available() {
    let fake = FakeClaude::new("interactive");
    let status = fake
        .command()
        .arg("continue")
        .stdin(Stdio::null())
        .status()
        .expect("run claude-safe");

    assert!(status.success());
    assert!(
        !fake
            .args()
            .contains(&"--no-session-persistence".to_string())
    );
    assert!(fake.args().starts_with(&["--disallowedTools".to_string()]));
    assert!(fake.args().ends_with(&["continue".to_string()]));
}

#[test]
fn exits_with_claude_status_code() {
    let fake = FakeClaude::new("exit");
    let status = fake
        .command()
        .env("CLAUDE_SAFE_EXIT", "7")
        .stdin(Stdio::null())
        .status()
        .expect("run claude-safe");

    assert_eq!(status.code(), Some(7));
}
