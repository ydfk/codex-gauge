use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use serde_json::Value;
use thiserror::Error;

use super::protocol;

#[derive(Debug, Error)]
pub enum AppServerError {
    #[error("codex_not_found")]
    CommandNotFound,
    #[error("app_server_error")]
    AppServer,
}

pub struct CodexAppServer {
    child: Child,
    stdin: ChildStdin,
    responses: Receiver<Value>,
    next_id: u64,
}

impl CodexAppServer {
    pub fn start(command: &str) -> Result<Self, AppServerError> {
        let mut last_error = None;
        let mut child = None;

        for candidate in codex_command_candidates(command) {
            match spawn_app_server(&candidate) {
                Ok(process) => {
                    child = Some(process);
                    break;
                }
                Err(err) => last_error = Some(err),
            }
        }

        let mut child = child.ok_or_else(|| match last_error {
            Some(err) => err,
            None => AppServerError::CommandNotFound,
        })?;

        let stdin = child.stdin.take().ok_or(AppServerError::AppServer)?;
        let stdout = child.stdout.take().ok_or(AppServerError::AppServer)?;
        let (sender, responses) = mpsc::channel();

        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                let Ok(size) = reader.read_line(&mut line) else {
                    break;
                };
                if size == 0 {
                    break;
                }
                if let Ok(value) = serde_json::from_str::<Value>(&line) {
                    let _ = sender.send(value);
                }
            }
        });

        Ok(Self {
            child,
            stdin,
            responses,
            next_id: 1,
        })
    }

    pub fn initialize(&mut self) -> Result<(), AppServerError> {
        let id = self.next_id();
        self.send(&protocol::initialize_request(id))?;
        self.read_response(id)?;
        self.send(&protocol::initialized_notification())
    }

    pub fn request(&mut self, method: &str) -> Result<Value, AppServerError> {
        let id = self.next_id();
        self.send(&protocol::method_request(id, method))?;
        self.read_response(id)
    }

    fn send(&mut self, value: &Value) -> Result<(), AppServerError> {
        let text = serde_json::to_string(value).map_err(|_| AppServerError::AppServer)?;
        self.stdin
            .write_all(text.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|_| AppServerError::AppServer)
    }

    fn read_response(&mut self, id: u64) -> Result<Value, AppServerError> {
        loop {
            let value = self
                .responses
                .recv_timeout(Duration::from_secs(8))
                .map_err(|_| AppServerError::AppServer)?;

            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if value.get("error").is_some() {
                return Err(AppServerError::AppServer);
            }

            return Ok(value);
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

fn spawn_app_server(command: &str) -> Result<Child, AppServerError> {
    let mut command = Command::new(command);
    command
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hide_windows_console(&mut command);

    command.spawn().map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied => {
            AppServerError::CommandNotFound
        }
        _ => AppServerError::AppServer,
    })
}

#[cfg(windows)]
fn hide_windows_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_windows_console(_command: &mut Command) {}

fn codex_command_candidates(command: &str) -> Vec<String> {
    if command != "codex" && command != "codex.exe" {
        return vec![command.to_string()];
    }

    let mut candidates = Vec::new();
    for path in bundled_codex_candidates() {
        if path.exists() {
            candidates.push(path.to_string_lossy().to_string());
        }
    }
    candidates.push(command.to_string());
    candidates
}

fn bundled_codex_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        let root = PathBuf::from(local_app_data);
        candidates.push(
            root.join("OpenAI")
                .join("Codex")
                .join("bin")
                .join("codex.exe"),
        );
        candidates.push(
            root.join("Packages")
                .join("OpenAI.Codex_2p2nqsd0c76g0")
                .join("LocalCache")
                .join("Local")
                .join("OpenAI")
                .join("Codex")
                .join("bin")
                .join("codex.exe"),
        );
    }

    candidates
}

impl Drop for CodexAppServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
