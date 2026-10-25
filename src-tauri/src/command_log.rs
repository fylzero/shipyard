use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub const COMMAND_LOG_EVENT: &str = "command-log";
pub const COMMAND_LOG_CLEARED_EVENT: &str = "command-log-cleared";

const MAX_ENTRIES: usize = 4000;
const MAX_OUTPUT_BYTES: usize = 12_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandLogEntry {
    pub id: String,
    pub at: u64,
    pub cwd: String,
    pub program: String,
    pub args: Vec<String>,
    pub command: String,
    pub success: bool,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
}

struct Logger {
    path: PathBuf,
    entries: Vec<CommandLogEntry>,
    app: Option<AppHandle>,
    paused: bool,
}

static LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

pub fn init(path: PathBuf, app: Option<AppHandle>) {
    let entries = load_entries(&path);
    if let Ok(mut slot) = LOGGER.lock() {
        *slot = Some(Logger {
            path,
            entries,
            app,
            paused: false,
        });
    }
}

pub fn list() -> Vec<CommandLogEntry> {
    LOGGER
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().map(|logger| logger.entries.clone()))
        .unwrap_or_default()
}

pub fn paused() -> bool {
    LOGGER
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().map(|logger| logger.paused))
        .unwrap_or(false)
}

pub fn set_paused(paused: bool) -> Result<(), String> {
    let mut slot = LOGGER
        .lock()
        .map_err(|_| "Could not lock the command history.".to_string())?;
    if let Some(logger) = slot.as_mut() {
        logger.paused = paused;
    }
    Ok(())
}

pub fn clear() -> Result<(), String> {
    let mut slot = LOGGER
        .lock()
        .map_err(|_| "Could not lock the command history.".to_string())?;
    let Some(logger) = slot.as_mut() else {
        return Ok(());
    };
    logger.entries.clear();
    fs::write(&logger.path, "").map_err(|err| format!("Could not clear command history: {err}"))?;
    if let Some(app) = &logger.app {
        let _ = app.emit(COMMAND_LOG_CLEARED_EVENT, ());
    }
    Ok(())
}

pub fn record(
    cwd: &Path,
    program: &Path,
    args: &[&str],
    success: bool,
    duration: Duration,
    stdout: &str,
    stderr: &str,
) {
    let Ok(mut slot) = LOGGER.lock() else {
        return;
    };
    let Some(logger) = slot.as_mut() else {
        return;
    };
    if logger.paused {
        return;
    }
    let entry = CommandLogEntry {
        id: Uuid::new_v4().to_string(),
        at: now_ms(),
        cwd: cwd.to_string_lossy().into_owned(),
        program: program.to_string_lossy().into_owned(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        command: format_command(program, args),
        success,
        duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        stdout: truncate_output(stdout),
        stderr: truncate_output(stderr),
    };
    logger.entries.push(entry.clone());
    if logger.entries.len() > MAX_ENTRIES {
        let drop_count = logger.entries.len() - MAX_ENTRIES;
        logger.entries.drain(0..drop_count);
        let _ = rewrite(&logger.path, &logger.entries);
    } else {
        let _ = append_line(&logger.path, &entry);
    }
    if let Some(app) = &logger.app {
        let _ = app.emit(COMMAND_LOG_EVENT, &entry);
    }
}

fn format_command(program: &Path, args: &[&str]) -> String {
    let name = program
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.to_string_lossy().into_owned());
    std::iter::once(quote(&name))
        .chain(args.iter().map(|arg| quote(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote(value: &str) -> String {
    if !value.is_empty()
        && value.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '-' | '_' | '.' | '/' | '=' | ':' | '@' | '+' | ',')
        })
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn truncate_output(text: &str) -> String {
    if text.len() <= MAX_OUTPUT_BYTES {
        return text.to_string();
    }
    let mut end = MAX_OUTPUT_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n… truncated {} bytes",
        &text[..end],
        text.len().saturating_sub(end)
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn load_entries(path: &Path) -> Vec<CommandLogEntry> {
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            continue;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<CommandLogEntry>(line) {
            entries.push(entry);
        }
    }
    if entries.len() > MAX_ENTRIES {
        let drop_count = entries.len() - MAX_ENTRIES;
        entries.drain(0..drop_count);
    }
    entries
}

fn append_line(path: &Path, entry: &CommandLogEntry) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| err.to_string())?;
    let raw = serde_json::to_string(entry).map_err(|err| err.to_string())?;
    writeln!(file, "{raw}").map_err(|err| err.to_string())
}

fn rewrite(path: &Path, entries: &[CommandLogEntry]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut raw = String::new();
    for entry in entries {
        raw.push_str(&serde_json::to_string(entry).map_err(|err| err.to_string())?);
        raw.push('\n');
    }
    fs::write(path, raw).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn temp_log() -> PathBuf {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("shipyard-history-{}-{}", std::process::id(), n));
        let _ = fs::create_dir_all(&dir);
        dir.join("command-history.jsonl")
    }

    #[test]
    fn persists_and_clears_command_history() {
        let path = temp_log();
        init(path.clone(), None);
        set_paused(false).unwrap();
        record(
            Path::new("/tmp/repo-a"),
            Path::new("/usr/bin/git"),
            &["status", "--porcelain"],
            true,
            Duration::from_millis(12),
            "ok",
            "",
        );
        record(
            Path::new("/tmp/repo-a"),
            Path::new("/usr/bin/git"),
            &["pull"],
            false,
            Duration::from_millis(40),
            "",
            "rejected",
        );

        let entries = list();
        let mine: Vec<_> = entries
            .iter()
            .filter(|entry| entry.cwd == "/tmp/repo-a")
            .collect();
        assert!(mine
            .iter()
            .any(|entry| entry.command == "git status --porcelain"));
        assert!(mine
            .iter()
            .any(|entry| entry.command == "git pull" && !entry.success));

        init(path, None);
        let reloaded = list();
        assert!(reloaded
            .iter()
            .any(|entry| entry.cwd == "/tmp/repo-a" && entry.args.contains(&"pull".into())));

        clear().unwrap();
        assert!(!list().iter().any(|entry| entry.cwd == "/tmp/repo-a"));
    }

    #[test]
    fn pause_skips_new_command_history() {
        let path = temp_log();
        init(path, None);
        set_paused(true).unwrap();
        assert!(paused());
        record(
            Path::new("/tmp/repo-paused"),
            Path::new("/usr/bin/git"),
            &["status"],
            true,
            Duration::from_millis(4),
            "",
            "",
        );
        assert!(!list().iter().any(|entry| entry.cwd == "/tmp/repo-paused"));

        set_paused(false).unwrap();
        assert!(!paused());
        record(
            Path::new("/tmp/repo-paused"),
            Path::new("/usr/bin/git"),
            &["pull"],
            true,
            Duration::from_millis(8),
            "",
            "",
        );
        assert!(list().iter().any(|entry| entry.cwd == "/tmp/repo-paused"));
    }
}
