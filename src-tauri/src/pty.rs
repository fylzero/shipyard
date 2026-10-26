use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

pub const TERMINAL_DATA_EVENT: &str = "terminal-data";
pub const TERMINAL_EXIT_EVENT: &str = "terminal-exit";

#[derive(Clone, Serialize)]
pub struct TerminalChunk {
    pub id: String,
    pub data: String,
}

#[derive(Clone, Serialize)]
pub struct TerminalExit {
    pub id: String,
}

struct TerminalSession {
    writer: Mutex<Box<dyn Write + Send>>,
    master: Box<dyn MasterPty + Send>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

#[derive(Clone, Default)]
pub struct TerminalState {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
}

fn default_shell() -> String {
    if let Ok(shell) = std::env::var("SHELL") {
        if !shell.trim().is_empty() {
            return shell;
        }
    }
    #[cfg(windows)]
    {
        return std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
    }
    #[cfg(target_os = "macos")]
    {
        "/bin/zsh".into()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "/bin/sh".into()
    }
}

fn lock_sessions(
    state: &TerminalState,
) -> Result<std::sync::MutexGuard<'_, HashMap<String, TerminalSession>>, String> {
    state
        .sessions
        .lock()
        .map_err(|_| "Could not lock terminal sessions.".to_string())
}

#[tauri::command]
pub fn open_terminal(
    app: AppHandle,
    state: State<TerminalState>,
    path: String,
    cols: u16,
    rows: u16,
) -> Result<String, String> {
    let cwd = Path::new(path.trim());
    if !cwd.is_dir() {
        return Err("Repository path is not a directory.".into());
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("Could not open a terminal: {err}"))?;

    let shell = default_shell();
    let mut cmd = CommandBuilder::new(&shell);
    #[cfg(unix)]
    {
        cmd.arg("-l");
    }
    cmd.cwd(cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|err| format!("Could not start {shell}: {err}"))?;
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| format!("Could not read from the terminal: {err}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|err| format!("Could not write to the terminal: {err}"))?;

    let id = Uuid::new_v4().to_string();
    let session = TerminalSession {
        writer: Mutex::new(writer),
        master: pair.master,
        child: Mutex::new(child),
    };

    lock_sessions(&state)?.insert(id.clone(), session);

    let emit_id = id.clone();
    let sessions = state.sessions.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                    let _ = app.emit(
                        TERMINAL_DATA_EVENT,
                        TerminalChunk {
                            id: emit_id.clone(),
                            data,
                        },
                    );
                }
                Err(_) => break,
            }
        }
        if let Ok(mut map) = sessions.lock() {
            map.remove(&emit_id);
        }
        let _ = app.emit(TERMINAL_EXIT_EVENT, TerminalExit { id: emit_id });
    });

    Ok(id)
}

#[tauri::command]
pub fn write_terminal(state: State<TerminalState>, id: String, data: String) -> Result<(), String> {
    let sessions = lock_sessions(&state)?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| "Terminal is not open.".to_string())?;
    let mut writer = session
        .writer
        .lock()
        .map_err(|_| "Could not lock the terminal writer.".to_string())?;
    writer
        .write_all(data.as_bytes())
        .map_err(|err| format!("Could not write to the terminal: {err}"))?;
    writer
        .flush()
        .map_err(|err| format!("Could not write to the terminal: {err}"))
}

#[tauri::command]
pub fn resize_terminal(
    state: State<TerminalState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = lock_sessions(&state)?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| "Terminal is not open.".to_string())?;
    session
        .master
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(20),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| format!("Could not resize the terminal: {err}"))
}

#[tauri::command]
pub fn close_terminal(state: State<TerminalState>, id: String) -> Result<(), String> {
    lock_sessions(&state)?.remove(&id);
    Ok(())
}
