use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use crate::command_log;
use crate::models::{
    BlameLine, BranchOverview, BranchTracking, CommitFile, CommitNode, DeleteMergedResult,
    FileBlame, GitConfig, LastCommit, LocalBranch, RemoteBranch, RemoteEntry, RemoteOverview,
    RepoFile, StashEntry, TagEntry, WorkingTreeFile,
};

pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub fn resolve_git_binary() -> Option<PathBuf> {
    if let Some(path) = look_on_path("git") {
        return Some(path);
    }

    for candidate in [
        "/opt/homebrew/bin/git",
        "/usr/local/bin/git",
        "/usr/bin/git",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(output) = Command::new("/bin/zsh")
        .args(["-l", "-c", "command -v git"])
        .output()
    {
        if output.status.success() {
            let resolved = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !resolved.is_empty() {
                let path = PathBuf::from(resolved);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }

    None
}

fn look_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}

pub fn run_git(git: &Path, repo: &Path, args: &[&str]) -> Result<GitOutput, String> {
    run_git_inner(git, repo, args, true)
}

fn run_git_quiet(git: &Path, repo: &Path, args: &[&str]) -> Result<GitOutput, String> {
    run_git_inner(git, repo, args, false)
}

fn run_git_stdin(git: &Path, cwd: &Path, args: &[&str], input: &str) -> Result<GitOutput, String> {
    let mut command = Command::new(git);
    command
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to run git: {err}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|err| format!("Failed to run git: {err}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("Failed to run git: {err}"))?;
    Ok(GitOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    })
}

fn run_git_inner(git: &Path, repo: &Path, args: &[&str], log: bool) -> Result<GitOutput, String> {
    run_git_command(git, repo, args, &[], log)
}

fn run_git_command(
    git: &Path,
    cwd: &Path,
    args: &[&str],
    extra_env: &[(&str, &Path)],
    log: bool,
) -> Result<GitOutput, String> {
    let started = Instant::now();
    let mut command = Command::new(git);
    // Status and diff otherwise refresh the index stat cache and rewrite
    // `.git/index`. The file watcher then runs them again. Optional locks
    // are only that cache update; add, commit, and restore still lock.
    command
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let result = command
        .output()
        .map_err(|err| format!("Failed to run git: {err}"));

    if log {
        match &result {
            Ok(output) => {
                command_log::record(
                    cwd,
                    git,
                    args,
                    output.status.success(),
                    started.elapsed(),
                    &String::from_utf8_lossy(&output.stdout),
                    &String::from_utf8_lossy(&output.stderr),
                );
            }
            Err(err) => {
                command_log::record(cwd, git, args, false, started.elapsed(), "", err);
            }
        }
    }

    let output = result?;
    Ok(GitOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    })
}

pub fn combined_message(output: &GitOutput) -> String {
    let stdout = output.stdout.trim();
    let stderr = output.stderr.trim();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout.to_string(),
        (true, false) => stderr.to_string(),
        (false, false) => format!("{stdout}\n{stderr}"),
    }
}

pub fn validate_ref(name: &str) -> Result<(), String> {
    validate_named_ref(name, "branch")
}

fn validate_tag(name: &str) -> Result<(), String> {
    validate_named_ref(name, "tag")
}

fn validate_named_ref(name: &str, kind: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 255 {
        return Err(format!("Invalid {kind} name"));
    }
    if name.starts_with('/')
        || name.ends_with('/')
        || name.starts_with('.')
        || name.contains("..")
        || name.contains('\\')
    {
        return Err(format!("Invalid {kind} name"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | '+'))
    {
        return Err(format!("Invalid {kind} name: {name}"));
    }
    Ok(())
}

const GIT_CONFIG_MAX_BYTES: usize = 1_000_000;

fn config_cwd() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}

fn path_to_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| "Git config path is not valid UTF-8".into())
}

pub fn git_config_path() -> PathBuf {
    global_write_path(None)
}

fn global_write_path(file: Option<&Path>) -> PathBuf {
    if let Some(file) = file {
        return file.to_path_buf();
    }
    if let Ok(path) = std::env::var("GIT_CONFIG_GLOBAL") {
        let path = path.trim();
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(path) = home.as_ref().map(|dir| dir.join(".gitconfig")) {
        if path.is_file() {
            return path;
        }
    }
    let xdg = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|dir| dir.join(".config")));
    if let Some(path) = xdg.map(|dir| dir.join("git").join("config")) {
        if path.is_file() {
            return path;
        }
    }
    home.map(|dir| dir.join(".gitconfig"))
        .unwrap_or_else(|| PathBuf::from(".gitconfig"))
}

fn run_git_config(git: &Path, args: &[&str], file: Option<&Path>) -> Result<GitOutput, String> {
    let cwd = config_cwd();
    let extra = file.map(|path| vec![("GIT_CONFIG_GLOBAL", path), ("GIT_CONFIG_NOSYSTEM", Path::new("1"))]);
    run_git_command(git, &cwd, args, extra.as_deref().unwrap_or(&[]), true)
}

fn get_global_value(git: &Path, key: &str, file: Option<&Path>) -> Result<String, String> {
    let output = run_git_config(git, &["config", "--global", "--get", key], file)?;
    if !output.success {
        return Ok(String::new());
    }
    Ok(output.stdout.trim().to_string())
}

fn normalize_pull_rebase(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => "true".into(),
        "false" | "no" | "off" | "0" => "false".into(),
        _ => String::new(),
    }
}

fn sanitize_config_value(key: &str, value: &str) -> Result<String, String> {
    if value.contains('\n') || value.contains('\r') || value.contains('\0') {
        return Err("Git config values cannot contain line breaks.".into());
    }
    if value.len() > 1024 {
        return Err("That git config value is too long.".into());
    }
    match key {
        "user.name" | "user.email" => Ok(value.trim().to_string()),
        "init.defaultBranch" => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(String::new());
            }
            validate_ref(value)?;
            Ok(value.to_string())
        }
        "checkout.defaultRemote" => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(String::new());
            }
            if value.len() > 255
                || value.starts_with('/')
                || value.ends_with('/')
                || value.starts_with('.')
                || value.contains("..")
                || value.contains('\\')
                || !value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | '+'))
            {
                return Err(format!("Invalid remote name: {value}"));
            }
            Ok(value.to_string())
        }
        "pull.rebase" => match value.trim().to_ascii_lowercase().as_str() {
            "" => Ok(String::new()),
            "true" | "yes" | "on" | "1" => Ok("true".into()),
            "false" | "no" | "off" | "0" => Ok("false".into()),
            other => Err(format!("Unsupported pull.rebase value: {other}")),
        },
        _ => Err(format!("Cannot edit {key} from this form.")),
    }
}

fn set_global_value(git: &Path, key: &str, value: &str, file: Option<&Path>) -> Result<(), String> {
    if value.is_empty() {
        let _ = run_git_config(git, &["config", "--global", "--unset-all", key], file)?;
        return Ok(());
    }
    let output = run_git_config(
        git,
        &["config", "--global", "--replace-all", key, value],
        file,
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not update git config.",
        ));
    }
    Ok(())
}

fn read_config_contents(path: &Path) -> Result<String, String> {
    if !path.is_file() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|err| format!("Could not read git config: {err}"))
}

fn read_git_config_at(git: &Path, file: Option<&Path>) -> Result<GitConfig, String> {
    let path = global_write_path(file);
    Ok(GitConfig {
        path: path_to_string(&path)?,
        contents: read_config_contents(&path)?,
        user_name: get_global_value(git, "user.name", file)?,
        user_email: get_global_value(git, "user.email", file)?,
        default_branch: get_global_value(git, "init.defaultBranch", file)?,
        pull_rebase: normalize_pull_rebase(&get_global_value(git, "pull.rebase", file)?),
        default_remote: get_global_value(git, "checkout.defaultRemote", file)?,
    })
}

pub fn read_git_config(git: &Path) -> Result<GitConfig, String> {
    read_git_config_at(git, None)
}

fn update_git_config_value_at(
    git: &Path,
    key: &str,
    value: &str,
    file: Option<&Path>,
) -> Result<GitConfig, String> {
    let value = sanitize_config_value(key, value)?;
    set_global_value(git, key, &value, file)?;
    read_git_config_at(git, file)
}

pub fn update_git_config_value(git: &Path, key: &str, value: &str) -> Result<GitConfig, String> {
    update_git_config_value_at(git, key, value, None)
}

fn write_git_config_at(git: &Path, contents: &str, file: Option<&Path>) -> Result<GitConfig, String> {
    if contents.len() > GIT_CONFIG_MAX_BYTES {
        return Err("Git config file is too large.".into());
    }
    if contents.contains('\0') {
        return Err("Git config cannot contain null bytes.".into());
    }
    let path = global_write_path(file);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Could not create the git config folder: {err}"))?;
        }
    }
    let previous = if path.is_file() {
        Some(read_config_contents(&path)?)
    } else {
        None
    };
    let mut body = contents.to_string();
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    fs::write(&path, &body).map_err(|err| format!("Could not write git config: {err}"))?;
    let check = run_git_config(git, &["config", "--global", "--list"], file)?;
    if !check.success {
        match previous {
            Some(old) => {
                let _ = fs::write(&path, old);
            }
            None => {
                let _ = fs::remove_file(&path);
            }
        }
        return Err(or_fallback(
            &combined_message(&check),
            "That git config file is not valid.",
        ));
    }
    read_git_config_at(git, file)
}

pub fn write_git_config(git: &Path, contents: &str) -> Result<GitConfig, String> {
    write_git_config_at(git, contents, None)
}

pub fn repo_root(git: &Path, path: &Path) -> Result<String, String> {
    let output = run_git(git, path, &["rev-parse", "--show-toplevel"])?;
    if !output.success {
        return Err("That folder is not a git repository.".into());
    }
    let root = output.stdout.trim();
    if root.is_empty() {
        return Err("That folder is not a git repository.".into());
    }
    Ok(root.to_string())
}

pub fn clone_repo(git: &Path, url: &str, parent: &Path, name: &str) -> Result<PathBuf, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("Enter a repository URL.".into());
    }
    if url.starts_with('-') || url.chars().any(char::is_control) {
        return Err("Invalid repository URL.".into());
    }
    let name = name.trim();
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err("Invalid folder name.".into());
    }
    if !parent.is_dir() {
        return Err("The destination folder does not exist.".into());
    }
    let dest = parent.join(name);
    if dest.exists() {
        let empty_dir = fs::read_dir(&dest)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !empty_dir {
            return Err(format!("{} already exists.", dest.display()));
        }
    }
    let dest_arg = dest
        .to_str()
        .ok_or_else(|| "Destination path is not valid UTF-8".to_string())?;
    let output = run_git(git, parent, &["clone", "--", url, dest_arg])?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), "Clone failed."));
    }
    Ok(dest)
}

pub struct LiveStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
    pub insertions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub conflicted_files: u32,
    pub operation: String,
}

pub fn fetch_remote(git: &Path, repo: &Path) {
    let _ = run_network_git(
        git,
        repo,
        &["fetch", "--all", "--prune", "--no-tags"],
        Duration::from_secs(30),
    );
}

/// Never prompts: credential helpers and ssh run in batch mode, and the process is
/// killed at `timeout` so an unreachable remote cannot hang the caller.
fn run_network_git(
    git: &Path,
    repo: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<GitOutput, String> {
    let started = Instant::now();
    let mut child = match Command::new(git)
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GCM_INTERACTIVE", "Never")
        .env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes -o ConnectTimeout=8")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            let message = format!("Failed to run git: {err}");
            command_log::record(repo, git, args, false, started.elapsed(), "", &message);
            return Err(message);
        }
    };

    let stderr_reader = child.stderr.take().map(|mut stderr| {
        thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut stderr, &mut text);
            text
        })
    });
    let collect_stderr =
        |reader: Option<thread::JoinHandle<String>>| reader.and_then(|handle| handle.join().ok()).unwrap_or_default();

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stderr = collect_stderr(stderr_reader);
                command_log::record(repo, git, args, status.success(), started.elapsed(), "", &stderr);
                return Ok(GitOutput {
                    stdout: String::new(),
                    stderr,
                    success: status.success(),
                });
            }
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = collect_stderr(stderr_reader);
                let message = format!("git {} timed out after {}s", args[0], timeout.as_secs());
                command_log::record(repo, git, args, false, started.elapsed(), "", &message);
                return Err(message);
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                let message = err.to_string();
                command_log::record(repo, git, args, false, started.elapsed(), "", &message);
                return Err(message);
            }
        }
    }
}

pub fn live_status(git: &Path, repo: &Path) -> Result<LiveStatus, String> {
    let output = run_git(
        git,
        repo,
        &["status", "--porcelain=v2", "--branch", "--untracked-files=all"],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read repository status.",
        ));
    }

    let mut branch = String::from("HEAD");
    let mut ahead = 0;
    let mut behind = 0;
    let mut dirty = false;
    let mut changed_files = 0;
    let mut conflicted_files = 0;
    let mut saw_ab = false;

    for line in output.stdout.lines() {
        if let Some(name) = line.strip_prefix("# branch.head ") {
            branch = name.trim().to_string();
            continue;
        }
        if let Some(counts) = line.strip_prefix("# branch.ab ") {
            saw_ab = true;
            let mut parts = counts.split_whitespace();
            ahead = parse_count(parts.next(), '+');
            behind = parse_count(parts.next(), '-');
            continue;
        }
        if !line.is_empty() && !line.starts_with('#') {
            dirty = true;
            changed_files += 1;
            if line.starts_with("u ") {
                conflicted_files += 1;
            }
        }
    }

    if branch == "HEAD" {
        if let Ok(short) = run_git(git, repo, &["rev-parse", "--short", "HEAD"]) {
            if short.success {
                branch = format!("detached {}", short.stdout.trim());
            } else {
                branch = "detached HEAD".into();
            }
        }
    } else if !saw_ab {
        (ahead, behind) = ahead_behind_for_ref(git, repo, &format!("origin/{branch}"));
    }

    let (insertions, deletions) = if dirty {
        working_tree_line_counts(git, repo)
    } else {
        (0, 0)
    };

    Ok(LiveStatus {
        branch,
        ahead,
        behind,
        dirty,
        insertions,
        deletions,
        changed_files,
        conflicted_files,
        operation: current_operation(repo).unwrap_or_default().to_string(),
    })
}

fn working_tree_line_counts(git: &Path, repo: &Path) -> (u32, u32) {
    let mut insertions = 0;
    let mut deletions = 0;

    if let Ok(output) = run_git(git, repo, &["diff", "--numstat", "HEAD"]) {
        if output.success {
            add_numstat(&output.stdout, &mut insertions, &mut deletions);
        }
    }

    if let Ok(output) = run_git(git, repo, &["ls-files", "--others", "--exclude-standard", "-z"]) {
        if output.success {
            for rel in output.stdout.split('\0').filter(|path| !path.is_empty()) {
                insertions += count_text_lines(&repo.join(rel));
            }
        }
    }

    (insertions, deletions)
}

fn add_numstat(stdout: &str, insertions: &mut u32, deletions: &mut u32) {
    for line in stdout.lines() {
        let mut parts = line.split('\t');
        if let Some(added) = parts.next().and_then(|value| value.parse::<u32>().ok()) {
            *insertions += added;
        }
        if let Some(removed) = parts.next().and_then(|value| value.parse::<u32>().ok()) {
            *deletions += removed;
        }
    }
}

fn count_text_lines(path: &Path) -> u32 {
    match fs::read(path) {
        Ok(bytes) if !bytes.contains(&0) => String::from_utf8_lossy(&bytes).lines().count() as u32,
        _ => 0,
    }
}

fn parse_count(value: Option<&str>, prefix: char) -> u32 {
    value
        .unwrap_or("0")
        .trim_start_matches(prefix)
        .parse()
        .unwrap_or(0)
}

fn ahead_behind_for_ref(git: &Path, repo: &Path, other: &str) -> (u32, u32) {
    ahead_behind_between(git, repo, "HEAD", other, true)
}

fn ahead_behind_between(
    git: &Path,
    repo: &Path,
    left: &str,
    right: &str,
    log: bool,
) -> (u32, u32) {
    let spec = format!("{left}...{right}");
    let output = if log {
        run_git(git, repo, &["rev-list", "--left-right", "--count", &spec])
    } else {
        run_git_quiet(git, repo, &["rev-list", "--left-right", "--count", &spec])
    };
    let output = match output {
        Ok(output) if output.success => output,
        _ => return (0, 0),
    };
    let mut parts = output.stdout.split_whitespace();
    let ahead = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
    let behind = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
    (ahead, behind)
}

fn head_branch_name(git: &Path, repo: &Path) -> Option<String> {
    let output = run_git(git, repo, &["symbolic-ref", "--quiet", "--short", "HEAD"]).ok()?;
    if !output.success {
        return None;
    }
    let name = output.stdout.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub fn current_branch(git: &Path, repo: &Path) -> Result<String, String> {
    let abbrev = run_git(git, repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if !abbrev.success {
        return Err(or_fallback(
            &combined_message(&abbrev),
            "Could not read the current branch.",
        ));
    }
    let name = abbrev.stdout.trim().to_string();
    if name != "HEAD" {
        return Ok(name);
    }

    let short = run_git(git, repo, &["rev-parse", "--short", "HEAD"])?;
    if short.success {
        Ok(format!("detached {}", short.stdout.trim()))
    } else {
        Ok("detached HEAD".into())
    }
}

pub fn folder_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string())
}

fn repo_git_dir(repo: &Path) -> Option<PathBuf> {
    let dot_git = repo.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    if !dot_git.is_file() {
        return None;
    }
    let contents = fs::read_to_string(&dot_git).ok()?;
    for line in contents.lines() {
        let Some(value) = line.strip_prefix("gitdir:") else {
            continue;
        };
        let path = PathBuf::from(value.trim());
        return Some(if path.is_absolute() {
            path
        } else {
            repo.join(path)
        });
    }
    None
}

pub fn current_operation(repo: &Path) -> Option<&'static str> {
    let dir = repo_git_dir(repo)?;
    if dir.join("rebase-merge").exists() || dir.join("rebase-apply").exists() {
        Some("rebase")
    } else if dir.join("MERGE_HEAD").exists() {
        Some("merge")
    } else if dir.join("CHERRY_PICK_HEAD").exists() {
        Some("cherry-pick")
    } else if dir.join("REVERT_HEAD").exists() {
        Some("revert")
    } else {
        None
    }
}

fn unmerged_letters(index: char, worktree: char) -> bool {
    matches!(
        (index, worktree),
        ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')
    )
}

fn remaining_conflicts(git: &Path, repo: &Path) -> Result<Vec<String>, String> {
    let output = run_git(git, repo, &["diff", "--name-only", "--diff-filter=U"])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read remaining conflicts.",
        ));
    }
    Ok(output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn abort_operation(git: &Path, repo: &Path) -> Result<String, String> {
    let operation = current_operation(repo).ok_or_else(|| {
        "No merge or rebase is in progress.".to_string()
    })?;
    let (args, fallback, success): (&[&str], &str, &str) = match operation {
        "rebase" => (
            &["rebase", "--abort"],
            "Failed to abort the rebase.",
            "Aborted rebase",
        ),
        "cherry-pick" => (
            &["cherry-pick", "--abort"],
            "Failed to abort the cherry-pick.",
            "Aborted cherry-pick",
        ),
        "revert" => (
            &["revert", "--abort"],
            "Failed to abort the revert.",
            "Aborted revert",
        ),
        _ => (
            &["merge", "--abort"],
            "Failed to abort the merge.",
            "Aborted merge",
        ),
    };
    let output = run_git(git, repo, args)?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), fallback));
    }
    Ok(or_fallback(&combined_message(&output), success))
}

pub fn continue_operation(git: &Path, repo: &Path) -> Result<String, String> {
    let operation = current_operation(repo).ok_or_else(|| {
        "No merge or rebase is in progress.".to_string()
    })?;
    let leftover = remaining_conflicts(git, repo)?;
    if !leftover.is_empty() {
        return Err("Resolve remaining conflicts and mark them resolved first.".into());
    }
    let (args, fallback, success): (&[&str], &str, &str) = match operation {
        "rebase" => (
            &["-c", "core.editor=true", "rebase", "--continue"],
            "Failed to continue the rebase.",
            "Continued rebase",
        ),
        "cherry-pick" => (
            &["-c", "core.editor=true", "cherry-pick", "--continue"],
            "Failed to continue the cherry-pick.",
            "Continued cherry-pick",
        ),
        "revert" => (
            &["-c", "core.editor=true", "revert", "--continue"],
            "Failed to continue the revert.",
            "Continued revert",
        ),
        _ => (
            &["-c", "core.editor=true", "merge", "--continue"],
            "Failed to continue the merge.",
            "Continued merge",
        ),
    };
    let output = run_git(git, repo, args)?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), fallback));
    }
    Ok(or_fallback(&combined_message(&output), success))
}

pub fn editor_app_name(editor: &str) -> Option<String> {
    match editor.trim() {
        "" | "system" => None,
        "cursor" => Some("Cursor".into()),
        "vscode" => Some("Visual Studio Code".into()),
        "phpstorm" => Some("PhpStorm".into()),
        "webstorm" => Some("WebStorm".into()),
        "intellij" => Some("IntelliJ IDEA".into()),
        "sublime" => Some("Sublime Text".into()),
        "nova" => Some("Nova".into()),
        "zed" => Some("Zed".into()),
        "textedit" => Some("TextEdit".into()),
        other => Some(other.to_string()),
    }
}

pub fn open_in_editor(repo: &Path, file: &str, editor: &str) -> Result<(), String> {
    require_file_path(file)?;
    let path = repo.join(file);
    if !path.exists() {
        return Err(
            "That file is not on disk. It may have been deleted in this conflict.".into(),
        );
    }
    let mut command = Command::new("open");
    if let Some(app) = editor_app_name(editor) {
        command.arg("-a").arg(&app).arg("--").arg(&path);
    } else {
        command.arg(&path);
    }
    let status = command
        .status()
        .map_err(|err| format!("Could not open the file: {err}"))?;
    if status.success() {
        Ok(())
    } else if let Some(app) = editor_app_name(editor) {
        Err(format!(
            "Could not open the file in {app}. Pick another editor in General settings."
        ))
    } else {
        Err("Could not open the file in an editor.".into())
    }
}

pub fn open_repo_in_finder(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        if path.exists() {
            return Err("That path is not a folder.".into());
        }
        return Err("That repository folder is missing.".into());
    }
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|err| format!("Could not open the repository in Finder: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not open the repository in Finder.".into())
    }
}

pub fn reveal_file_in_finder(repo: &Path, file: &str) -> Result<(), String> {
    let path = repo_file_path(repo, file)?;
    if !path.exists() {
        return Err("That file is not on disk.".into());
    }
    let status = Command::new("open")
        .arg("-R")
        .arg(&path)
        .status()
        .map_err(|err| format!("Could not show the file in Finder: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not show the file in Finder.".into())
    }
}

pub fn repo_remote_browse_url(git: &Path, repo: &Path) -> Result<String, String> {
    let output = run_git_quiet(git, repo, &["remote", "get-url", "origin"])?;
    if !output.success {
        return Err("This repository has no origin remote.".into());
    }
    let remote = output.stdout.trim();
    if remote.is_empty() {
        return Err("This repository has no origin remote.".into());
    }
    remote_browse_url(remote)
}

pub fn remote_browse_url(remote: &str) -> Result<String, String> {
    let remote = remote.trim();
    if remote.is_empty() {
        return Err("This repository has no origin remote.".into());
    }
    if is_local_remote(remote) {
        return Err("This remote is a local path, not a web URL.".into());
    }
    if let Some(url) = browse_url_from_http(remote)
        .or_else(|| browse_url_from_ssh_or_git(remote))
        .or_else(|| browse_url_from_scp(remote))
    {
        return Ok(url);
    }
    Err("Could not turn the origin remote into a web URL.".into())
}

fn is_local_remote(remote: &str) -> bool {
    let lower = remote.to_ascii_lowercase();
    if lower.starts_with("file://") {
        return true;
    }
    if remote.starts_with('/') || remote.starts_with('~') || remote.starts_with('.') {
        return true;
    }
    let bytes = remote.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn strip_scheme_ignore_ascii_case<'a>(value: &'a str, scheme: &str) -> Option<&'a str> {
    if value.len() >= scheme.len() && value[..scheme.len()].eq_ignore_ascii_case(scheme) {
        Some(&value[scheme.len()..])
    } else {
        None
    }
}

fn strip_git_suffix(value: &str) -> &str {
    let value = value.trim_end_matches('/');
    value
        .strip_suffix(".git")
        .or_else(|| value.strip_suffix(".GIT"))
        .unwrap_or(value)
        .trim_end_matches('/')
}

fn browse_url_from_http(remote: &str) -> Option<String> {
    let (scheme, rest) = if let Some(rest) = strip_scheme_ignore_ascii_case(remote, "https://") {
        ("https", rest)
    } else if let Some(rest) = strip_scheme_ignore_ascii_case(remote, "http://") {
        ("http", rest)
    } else {
        return None;
    };
    let rest = strip_url_userinfo(rest);
    if rest.is_empty() || rest.starts_with('/') {
        return None;
    }
    Some(format!("{scheme}://{}", strip_git_suffix(&rest)))
}

fn strip_url_userinfo(rest: &str) -> String {
    let Some(slash) = rest.find('/') else {
        return rest
            .rsplit_once('@')
            .map(|(_, host)| host.to_string())
            .unwrap_or_else(|| rest.to_string());
    };
    let head = &rest[..slash];
    let tail = &rest[slash..];
    if let Some((_, host)) = head.rsplit_once('@') {
        format!("{host}{tail}")
    } else {
        rest.to_string()
    }
}

fn browse_url_from_ssh_or_git(remote: &str) -> Option<String> {
    let rest = strip_scheme_ignore_ascii_case(remote, "ssh://")
        .or_else(|| strip_scheme_ignore_ascii_case(remote, "git://"))?;
    let rest = rest
        .rsplit_once('@')
        .map(|(_, host_and_path)| host_and_path)
        .unwrap_or(rest);
    let slash = rest.find('/')?;
    let hostport = &rest[..slash];
    let path = &rest[slash + 1..];
    let host = hostport.split(':').next().unwrap_or_default();
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("https://{host}/{}", strip_git_suffix(path)))
}

fn browse_url_from_scp(remote: &str) -> Option<String> {
    if remote.contains("://") {
        return None;
    }
    let (user_host, path) = remote.split_once(':')?;
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        return None;
    }
    let host = user_host
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(user_host);
    if host.is_empty() || host.contains('/') {
        return None;
    }
    Some(format!("https://{host}/{}", strip_git_suffix(path)))
}

pub fn ref_exists(git: &Path, repo: &Path, git_ref: &str) -> bool {
    run_git(git, repo, &["show-ref", "--verify", "--quiet", git_ref])
        .map(|output| output.success)
        .unwrap_or(false)
}

pub fn local_branches(git: &Path, repo: &Path) -> Result<Vec<String>, String> {
    let output = run_git(
        git,
        repo,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not list local branches.",
        ));
    }
    let mut branches: Vec<String> = output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    branches.sort();
    Ok(branches)
}

pub fn local_branch_tracking(git: &Path, repo: &Path) -> Result<Vec<BranchTracking>, String> {
    let output = run_git(
        git,
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(upstream)%00%(upstream:track)",
            "refs/heads",
            "refs/remotes/origin",
        ],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not list branch remotes.",
        ));
    }

    let mut locals = Vec::new();
    let mut origin_names = HashSet::new();
    for line in output.stdout.lines() {
        let mut parts = line.split('\0');
        let refname = parts.next().unwrap_or("");
        let short = parts.next().unwrap_or("").trim();
        let upstream = parts.next().unwrap_or("").trim();
        let track = parts.next().unwrap_or("").trim();
        if short.is_empty() {
            continue;
        }
        if refname.starts_with("refs/heads/") {
            locals.push((short.to_string(), upstream.to_string(), track.to_string()));
            continue;
        }
        if let Some(name) = short.strip_prefix("origin/") {
            if name != "HEAD" {
                origin_names.insert(name.to_string());
            }
        }
    }
    locals.sort_by(|left, right| left.0.cmp(&right.0));

    Ok(locals
        .into_iter()
        .map(|(name, upstream, track)| {
            branch_tracking_for(git, repo, &name, &upstream, &track, &origin_names)
        })
        .collect())
}

fn branch_tracking_for(
    git: &Path,
    repo: &Path,
    name: &str,
    upstream: &str,
    track: &str,
    origin_names: &HashSet<String>,
) -> BranchTracking {
    if let Some(remote) = remote_tracking_name(upstream) {
        if let Some((ahead, behind)) = parse_upstream_track(track) {
            return BranchTracking {
                name: name.to_string(),
                local_only: false,
                ahead,
                behind,
                upstream: Some(remote),
            };
        }
    }

    let origin = format!("origin/{name}");
    if origin_names.contains(name) {
        let (ahead, behind) = ahead_behind_between(git, repo, name, &origin, false);
        return BranchTracking {
            name: name.to_string(),
            local_only: false,
            ahead,
            behind,
            upstream: Some(origin),
        };
    }

    BranchTracking {
        name: name.to_string(),
        local_only: true,
        ahead: 0,
        behind: 0,
        upstream: None,
    }
}

fn remote_tracking_name(upstream: &str) -> Option<String> {
    let rest = upstream.strip_prefix("refs/remotes/")?.trim();
    if rest.is_empty() {
        return None;
    }
    let short = rest.rsplit_once('/').map(|(_, name)| name).unwrap_or(rest);
    if short == "HEAD" {
        return None;
    }
    Some(rest.to_string())
}

fn parse_upstream_track(track: &str) -> Option<(u32, u32)> {
    let track = track.trim();
    if track.is_empty() {
        return Some((0, 0));
    }
    let inner = track.trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() {
        return Some((0, 0));
    }
    if inner.to_ascii_lowercase().contains("gone") {
        return None;
    }
    let mut ahead = 0;
    let mut behind = 0;
    for part in inner.split(',') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("ahead ") {
            ahead = value.trim().parse().unwrap_or(0);
        } else if let Some(value) = part.strip_prefix("behind ") {
            behind = value.trim().parse().unwrap_or(0);
        }
    }
    Some((ahead, behind))
}

fn short_branch_name(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn is_protected_branch(name: &str) -> bool {
    matches!(name, "develop" | "main" | "master")
}

fn candidate_git_ref(name: &str) -> String {
    if let Some(remote) = name.strip_prefix("origin/") {
        format!("refs/remotes/origin/{remote}")
    } else {
        format!("refs/heads/{name}")
    }
}

fn merge_target_candidates(preferred: Option<&str>) -> Vec<String> {
    let mut candidates = Vec::new();
    if let Some(name) = preferred.map(str::trim).filter(|value| !value.is_empty()) {
        candidates.push(format!("origin/{name}"));
        candidates.push(name.to_string());
    }
    for name in ["develop", "main", "master"] {
        candidates.push(format!("origin/{name}"));
        candidates.push(name.to_string());
    }
    let mut seen = HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn resolve_merge_target_from(preferred: Option<&str>, known: &HashSet<String>) -> Option<String> {
    merge_target_candidates(preferred)
        .into_iter()
        .find(|candidate| known.contains(candidate))
}

struct BranchRefIndex {
    local: Vec<String>,
    current: String,
    shorts: HashSet<String>,
    oids: HashMap<String, String>,
}

fn branch_ref_index(git: &Path, repo: &Path) -> Result<BranchRefIndex, String> {
    let output = run_git(
        git,
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(objectname)",
            "refs/heads",
            "refs/remotes/origin",
        ],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not list local branches.",
        ));
    }

    let mut local = Vec::new();
    let mut current = String::new();
    let mut shorts = HashSet::new();
    let mut oids = HashMap::new();
    for line in output.stdout.lines() {
        let mut parts = line.split('\0');
        let refname = parts.next().unwrap_or("");
        let short = parts.next().unwrap_or("").trim();
        let head = parts.next().unwrap_or("");
        let oid = parts.next().unwrap_or("").trim();
        if short.is_empty() {
            continue;
        }
        shorts.insert(short.to_string());
        if !oid.is_empty() {
            oids.insert(short.to_string(), oid.to_string());
        }
        if refname.starts_with("refs/heads/") {
            local.push(short.to_string());
            if head == "*" {
                current = short.to_string();
            }
        }
    }
    if current.is_empty() {
        current = current_branch(git, repo).unwrap_or_default();
    }
    Ok(BranchRefIndex {
        local,
        current,
        shorts,
        oids,
    })
}

fn ancestor_merged_names(git: &Path, repo: &Path, target: &str) -> HashSet<String> {
    let spec = format!("--merged={target}");
    match run_git(
        git,
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:short)",
            &spec,
            "refs/heads",
        ],
    ) {
        Ok(output) if output.success => output
            .stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect(),
        _ => HashSet::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CherryContainment {
    None,
    Partial,
    Full,
}

fn cherry_workers(jobs: usize) -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get().clamp(2, 8))
        .unwrap_or(4)
        .min(jobs.max(1))
}

fn branch_cherry_containment(
    git: &Path,
    repo: &Path,
    branch_ref: &str,
    target: &str,
) -> CherryContainment {
    let output = match run_git_quiet(git, repo, &["cherry", target, branch_ref]) {
        Ok(output) if output.success => output,
        _ => return CherryContainment::None,
    };
    let mut in_target = 0u32;
    let mut unique = 0u32;
    for line in output.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('-') {
            in_target += 1;
        } else if line.starts_with('+') {
            unique += 1;
        }
    }
    match (in_target, unique) {
        (0, _) => CherryContainment::None,
        (_, 0) => CherryContainment::Full,
        _ => CherryContainment::Partial,
    }
}

fn cherry_map(
    git: &Path,
    repo: &Path,
    target: &str,
    names: &[String],
) -> HashMap<String, CherryContainment> {
    if names.is_empty() {
        return HashMap::new();
    }
    if names.len() == 1 {
        let name = &names[0];
        let mut map = HashMap::new();
        map.insert(
            name.clone(),
            branch_cherry_containment(git, repo, &format!("refs/heads/{name}"), target),
        );
        return map;
    }

    let next = Mutex::new(0usize);
    let results = Mutex::new(HashMap::new());
    let workers = cherry_workers(names.len());
    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let index = {
                    let Ok(mut next) = next.lock() else {
                        return;
                    };
                    let index = *next;
                    if index >= names.len() {
                        return;
                    }
                    *next += 1;
                    index
                };
                let name = &names[index];
                let containment = branch_cherry_containment(
                    git,
                    repo,
                    &format!("refs/heads/{name}"),
                    target,
                );
                if let Ok(mut results) = results.lock() {
                    results.insert(name.clone(), containment);
                }
            });
        }
    });
    results.into_inner().unwrap_or_default()
}

fn is_not_fully_merged(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("not fully merged") || lower.contains("not yet merged to")
}

fn tidy_git_message(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("hint:")
                && !line.contains("advice.forceDeleteBranch")
        })
        .map(|line| {
            line.strip_prefix("error: ")
                .or_else(|| line.strip_prefix("warning: "))
                .unwrap_or(line)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn join_branch_names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [name] => name.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let (last, rest) = names.split_last().unwrap();
            format!("{} and {last}", rest.join(", "))
        }
    }
}

fn format_delete_merged_message(
    deleted: &[String],
    refused: &[String],
    errors: &[String],
) -> String {
    let mut parts = Vec::new();
    if !deleted.is_empty() {
        parts.push(format!(
            "Deleted {} merged {}",
            deleted.len(),
            if deleted.len() == 1 {
                "branch"
            } else {
                "branches"
            }
        ));
    }
    if !refused.is_empty() {
        parts.push(format!(
            "Git would not safely delete {}",
            join_branch_names(refused)
        ));
    }
    if !errors.is_empty() {
        parts.push(errors.join(" "));
    }
    if parts.is_empty() {
        "No merged local branches to delete.".into()
    } else {
        parts.join(". ")
    }
}

fn sort_local_branches(branches: &mut [LocalBranch]) {
    branches.sort_by(|left, right| {
        right
            .current
            .cmp(&left.current)
            .then(left.name.cmp(&right.name))
    });
}

pub fn branch_overview_with(
    git: &Path,
    repo: &Path,
    preferred: Option<&str>,
    classify: bool,
) -> Result<BranchOverview, String> {
    let index = branch_ref_index(git, repo)?;
    let merge_target = resolve_merge_target_from(preferred, &index.shorts);
    let target_ref = merge_target.as_deref().map(candidate_git_ref);
    let target_short = merge_target
        .as_deref()
        .map(short_branch_name)
        .unwrap_or_default();
    let ancestor_merged = target_ref
        .as_deref()
        .map(|target| ancestor_merged_names(git, repo, target))
        .unwrap_or_default();
    let target_oid = merge_target
        .as_deref()
        .and_then(|target| index.oids.get(target).cloned());

    let leftover: Vec<String> = index
        .local
        .iter()
        .filter(|name| {
            let protected_branch = is_protected_branch(name) || *name == target_short;
            !protected_branch && !ancestor_merged.contains(*name)
        })
        .cloned()
        .collect();
    let cherries = if classify {
        target_ref
            .as_deref()
            .map(|target| cherry_map(git, repo, target, &leftover))
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let mut branches: Vec<LocalBranch> = index
        .local
        .into_iter()
        .map(|name| {
            let protected_branch = is_protected_branch(&name) || name == target_short;
            let ancestor = ancestor_merged.contains(&name);
            let same_tip = target_oid
                .as_deref()
                .and_then(|target| index.oids.get(&name).map(|oid| oid.as_str() == target))
                .unwrap_or(false);
            let needs_cherry = !ancestor && !protected_branch && target_ref.is_some();
            let cherry = cherries.get(&name).copied();
            let pending = needs_cherry && (!classify || cherry.is_none());
            /*
             * Same tip as the integration branch is a new pointer, not leftover
             * work. Delete merged uses this same leftover list.
             */
            let merged = (ancestor && !same_tip) || cherry == Some(CherryContainment::Full);
            let partial = !merged && cherry == Some(CherryContainment::Partial);
            LocalBranch {
                current: name == index.current,
                merged,
                partial,
                protected_branch,
                pending,
                name,
            }
        })
        .collect();
    sort_local_branches(&mut branches);
    Ok(BranchOverview {
        merge_target,
        branches,
    })
}

fn quoted_branch_name(line: &str) -> Option<String> {
    let start = line.find('\'')?;
    let rest = &line[start + 1..];
    let end = rest.find('\'')?;
    let name = &rest[..end];
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn deleted_branch_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("Deleted branch ")?;
    let name = rest.split(" (was").next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn classify_branch_delete_output(
    names: &[String],
    output: &GitOutput,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let text = combined_message(output);
    let mut deleted = Vec::new();
    let mut refused = Vec::new();
    let mut seen = HashSet::new();

    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = deleted_branch_name(line) {
            if seen.insert(name.clone()) {
                deleted.push(name);
            }
            continue;
        }
        if is_not_fully_merged(line) {
            if let Some(name) = quoted_branch_name(line) {
                if seen.insert(name.clone()) {
                    refused.push(name);
                }
            }
        }
    }

    let mut errors = Vec::new();
    let leftover_error = tidy_git_message(&text);
    for name in names {
        if seen.contains(name) {
            continue;
        }
        if output.success {
            deleted.push(name.clone());
            continue;
        }
        if leftover_error.is_empty() {
            errors.push(format!("Failed to delete {name}"));
        } else {
            errors.push(leftover_error.clone());
        }
    }
    errors.sort();
    errors.dedup();
    (deleted, refused, errors)
}

fn delete_named_branches(
    git: &Path,
    repo: &Path,
    names: &[String],
    force: bool,
) -> Result<DeleteMergedResult, String> {
    let mut valid = Vec::new();
    let mut errors = Vec::new();
    for name in names {
        match validate_ref(name) {
            Ok(()) => valid.push(name.clone()),
            Err(err) => errors.push(err),
        }
    }
    if valid.is_empty() {
        return Ok(DeleteMergedResult {
            message: format_delete_merged_message(&[], &[], &errors),
            deleted: Vec::new(),
            refused: Vec::new(),
            errors,
        });
    }

    let flag = if force { "-D" } else { "-d" };
    let mut args = vec!["branch", flag];
    args.extend(valid.iter().map(String::as_str));
    let output = run_git(git, repo, &args)?;
    let (deleted, refused, mut parse_errors) = classify_branch_delete_output(&valid, &output);
    errors.append(&mut parse_errors);
    Ok(DeleteMergedResult {
        message: format_delete_merged_message(&deleted, &refused, &errors),
        deleted,
        refused,
        errors,
    })
}

pub fn delete_local_branch(git: &Path, repo: &Path, branch: &str, force: bool) -> Result<String, String> {
    validate_ref(branch)?;
    let current = current_branch(git, repo)?;
    if current == branch {
        return Err("Cannot delete the branch that is currently checked out.".into());
    }
    if !ref_exists(git, repo, &format!("refs/heads/{branch}")) {
        return Err(format!("Local branch {branch} does not exist."));
    }
    let result = delete_named_branches(git, repo, &[branch.to_string()], force)?;
    if !result.deleted.is_empty() {
        return Ok(format!("Deleted {branch}"));
    }
    if !result.refused.is_empty() {
        return Err(format!("Branch {branch} is not fully merged."));
    }
    Err(or_fallback(
        &result.errors.join(" "),
        &format!("Failed to delete {branch}"),
    ))
}

pub fn delete_merged_branches(
    git: &Path,
    repo: &Path,
    preferred: Option<&str>,
    force: bool,
    names: Option<&[String]>,
) -> Result<DeleteMergedResult, String> {
    let current = current_branch(git, repo).unwrap_or_default();
    let victims: Vec<String> = if let Some(names) = names {
        names
            .iter()
            .filter(|name| *name != &current && !is_protected_branch(name))
            .cloned()
            .collect()
    } else {
        let overview = branch_overview_with(git, repo, preferred, true)?;
        overview
            .branches
            .iter()
            .filter(|branch| branch.merged && !branch.current && !branch.protected_branch)
            .map(|branch| branch.name.clone())
            .collect()
    };
    if victims.is_empty() {
        return Ok(DeleteMergedResult {
            deleted: Vec::new(),
            refused: Vec::new(),
            errors: Vec::new(),
            message: "No merged local branches to delete.".into(),
        });
    }
    delete_named_branches(git, repo, &victims, force)
}

pub fn checkout_local_branch(git: &Path, repo: &Path, branch: &str) -> Result<String, String> {
    validate_ref(branch)?;
    let current = current_branch(git, repo)?;
    if current == branch {
        return Ok(format!("Already on {branch}"));
    }
    if !ref_exists(git, repo, &format!("refs/heads/{branch}")) {
        return Err(format!("Local branch {branch} does not exist."));
    }
    let output = run_git(git, repo, &["checkout", branch])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to check out {branch}"),
        ));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Checked out {branch}"),
    ))
}

pub fn create_and_checkout_branch(
    git: &Path,
    repo: &Path,
    branch: &str,
    start: &str,
) -> Result<String, String> {
    validate_ref(branch)?;
    if ref_exists(git, repo, &format!("refs/heads/{branch}")) {
        return Err(format!("Branch {branch} already exists."));
    }
    let start = start.trim();
    let output = if start.is_empty() || head_branch_name(git, repo).as_deref() == Some(start) {
        run_git(git, repo, &["checkout", "-b", branch])?
    } else if ref_exists(git, repo, &format!("refs/heads/{start}")) {
        run_git(git, repo, &["checkout", "-b", branch, start])?
    } else if validate_commit_hash(start).is_ok() {
        let hash = require_commit(git, repo, start)?;
        run_git(git, repo, &["checkout", "-b", branch, &hash])?
    } else {
        return Err(format!("Local branch {start} does not exist."));
    };
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to create branch {branch}"),
        ));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Created and checked out {branch}"),
    ))
}

fn require_no_operation(repo: &Path) -> Result<(), String> {
    if let Some(operation) = current_operation(repo) {
        return Err(format!("Finish or abort the {operation} first."));
    }
    Ok(())
}

fn is_merge_commit(git: &Path, repo: &Path, hash: &str) -> bool {
    let spec = format!("{hash}^2");
    run_git_quiet(git, repo, &["rev-parse", "--verify", "--quiet", &spec])
        .map(|output| output.success && !output.stdout.trim().is_empty())
        .unwrap_or(false)
}

fn head_commit(git: &Path, repo: &Path) -> Result<String, String> {
    let output = run_git_quiet(git, repo, &["rev-parse", "--verify", "HEAD"])?;
    let hash = output.stdout.trim();
    if !output.success || hash.is_empty() {
        return Err("Could not read HEAD.".into());
    }
    Ok(hash.to_string())
}

pub fn checkout_commit(git: &Path, repo: &Path, hash: &str) -> Result<String, String> {
    require_no_operation(repo)?;
    let hash = require_commit(git, repo, hash)?;
    if head_commit(git, repo)? == hash {
        return Ok("Already on this commit.".into());
    }
    let output = run_git(git, repo, &["checkout", "--detach", &hash])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to check out that commit.",
        ));
    }
    let short = hash.get(..7).unwrap_or(&hash);
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Checked out {short} (detached HEAD)"),
    ))
}

fn resolve_commit_list(git: &Path, repo: &Path, hashes: &[String]) -> Result<Vec<String>, String> {
    if hashes.is_empty() {
        return Err("Select at least one commit.".into());
    }
    let mut resolved = Vec::with_capacity(hashes.len());
    for hash in hashes {
        resolved.push(require_commit(git, repo, hash)?);
    }
    Ok(resolved)
}

pub fn cherry_pick_commits(git: &Path, repo: &Path, hashes: &[String]) -> Result<String, String> {
    require_no_operation(repo)?;
    let resolved = resolve_commit_list(git, repo, hashes)?;
    if resolved.iter().any(|hash| is_merge_commit(git, repo, hash)) {
        return Err("Cherry-pick cannot apply a merge commit.".into());
    }
    let mut args = vec!["cherry-pick".to_string()];
    args.extend(resolved);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_git(git, repo, &arg_refs)?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Cherry-pick failed.",
        ));
    }
    let label = if hashes.len() == 1 {
        "Cherry-picked 1 commit".to_string()
    } else {
        format!("Cherry-picked {} commits", hashes.len())
    };
    Ok(or_fallback(&combined_message(&output), &label))
}

pub fn revert_commits(git: &Path, repo: &Path, hashes: &[String]) -> Result<String, String> {
    require_no_operation(repo)?;
    let resolved = resolve_commit_list(git, repo, hashes)?;
    if resolved.iter().any(|hash| is_merge_commit(git, repo, hash)) {
        return Err("Revert cannot undo a merge commit.".into());
    }
    let mut args = vec!["revert".to_string(), "--no-edit".to_string()];
    args.extend(resolved);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_git(git, repo, &arg_refs)?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), "Revert failed."));
    }
    let label = if hashes.len() == 1 {
        "Reverted 1 commit".to_string()
    } else {
        format!("Reverted {} commits", hashes.len())
    };
    Ok(or_fallback(&combined_message(&output), &label))
}

fn host_from_browse_url(url: &str) -> String {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    rest.split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn commit_browse_url(browse: &str, hash: &str) -> Result<String, String> {
    validate_commit_hash(hash)?;
    let browse = browse.trim().trim_end_matches('/');
    if browse.is_empty() {
        return Err("This repository has no origin remote.".into());
    }
    let host = host_from_browse_url(browse);
    let path = if host.contains("gitlab.") || host.ends_with("gitlab.com") {
        format!("/-/commit/{hash}")
    } else if host.contains("bitbucket.") || host.ends_with("bitbucket.org") {
        format!("/commits/{hash}")
    } else {
        format!("/commit/{hash}")
    };
    Ok(format!("{browse}{path}"))
}

pub fn commit_remote_url(git: &Path, repo: &Path, hash: &str) -> Result<String, String> {
    let hash = require_commit(git, repo, hash)?;
    let browse = repo_remote_browse_url(git, repo)?;
    commit_browse_url(&browse, &hash)
}

pub fn merge_local_branch(
    git: &Path,
    repo: &Path,
    source: &str,
    target: &str,
) -> Result<String, String> {
    validate_ref(source)?;
    validate_ref(target)?;
    require_no_operation(repo)?;
    if source == target {
        return Err("Choose a different branch to merge into.".into());
    }
    if !ref_exists(git, repo, &format!("refs/heads/{source}")) {
        return Err(format!("Local branch {source} does not exist."));
    }
    if !ref_exists(git, repo, &format!("refs/heads/{target}")) {
        return Err(format!("Local branch {target} does not exist."));
    }

    let current = current_branch(git, repo)?;
    if current != target {
        checkout_local_branch(git, repo, target)?;
    }

    let output = run_git(git, repo, &["merge", "--no-edit", source])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to merge {source} into {target}."),
        ));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Merged {source} into {target}"),
    ))
}

pub fn rename_local_branch(
    git: &Path,
    repo: &Path,
    branch: &str,
    new_name: &str,
) -> Result<String, String> {
    validate_ref(branch)?;
    validate_ref(new_name)?;
    if branch == new_name {
        return Ok(format!("Already named {new_name}"));
    }
    if !ref_exists(git, repo, &format!("refs/heads/{branch}")) {
        return Err(format!("Local branch {branch} does not exist."));
    }
    if ref_exists(git, repo, &format!("refs/heads/{new_name}")) {
        return Err(format!("Branch {new_name} already exists."));
    }
    let output = run_git(git, repo, &["branch", "-m", branch, new_name])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to rename {branch}"),
        ));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Renamed {branch} to {new_name}"),
    ))
}

pub const PUSH_REJECTED_PREFIX: &str = "Push rejected:";

fn has_upstream(git: &Path, repo: &Path) -> bool {
    run_git_quiet(git, repo, &["rev-parse", "--abbrev-ref", "@{upstream}"])
        .map(|output| output.success && !output.stdout.trim().is_empty())
        .unwrap_or(false)
}

fn has_origin_remote(git: &Path, repo: &Path) -> bool {
    run_git_quiet(git, repo, &["remote", "get-url", "origin"])
        .map(|output| output.success)
        .unwrap_or(false)
}

/// Branches created or pushed without `-u` have no upstream, so plain `git pull` refuses to run
/// even though `origin/<branch>` exists. Link them so pull and push target the same branch.
fn link_missing_upstream(git: &Path, repo: &Path) {
    if has_upstream(git, repo) {
        return;
    }
    let Some(branch) = head_branch_name(git, repo) else {
        return;
    };
    if !ref_exists(git, repo, &format!("refs/remotes/origin/{branch}")) {
        return;
    }
    let upstream = format!("--set-upstream-to=origin/{branch}");
    let _ = run_git(git, repo, &["branch", &upstream, &branch]);
}

fn has_config(git: &Path, repo: &Path, key: &str) -> bool {
    run_git_quiet(git, repo, &["config", "--get", key])
        .map(|output| output.success && !output.stdout.trim().is_empty())
        .unwrap_or(false)
}

/// Standard `git pull` only. Never add `--force` or other overwrite flags.
/// With no `remote_branch`, pulls the current branch's upstream. Without `pull.rebase` or
/// `pull.ff` configured, git refuses to reconcile diverged branches, so fall back to a merge.
pub fn run_pull(git: &Path, repo: &Path, remote_branch: Option<&str>) -> Result<GitOutput, String> {
    let mut args = vec!["pull"];
    if !has_config(git, repo, "pull.rebase") && !has_config(git, repo, "pull.ff") {
        args.push("--no-rebase");
    }
    match remote_branch {
        Some(branch) => args.extend(["origin", branch]),
        None => link_missing_upstream(git, repo),
    }
    run_git(git, repo, &args)
}

pub fn pull(git: &Path, repo: &Path) -> Result<String, String> {
    let output = run_pull(git, repo, None)?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), "Pull failed."));
    }
    Ok(or_fallback(
        &combined_message(&output),
        "Pulled current branch",
    ))
}

fn is_non_fast_forward(message: &str) -> bool {
    message.contains("[rejected]")
        && (message.contains("non-fast-forward") || message.contains("fetch first"))
}

/// Standard `git push` only. Never add `--force`, `--force-with-lease`, or `+` refspecs.
pub fn push(git: &Path, repo: &Path) -> Result<String, String> {
    link_missing_upstream(git, repo);
    let new_branch = if has_upstream(git, repo) || !has_origin_remote(git, repo) {
        None
    } else {
        head_branch_name(git, repo)
    };
    let output = match &new_branch {
        Some(branch) => run_git(git, repo, &["push", "-u", "origin", branch])?,
        None => run_git(git, repo, &["push"])?,
    };
    if !output.success {
        let message = combined_message(&output);
        if is_non_fast_forward(&message) {
            return Err(format!(
                "{PUSH_REJECTED_PREFIX} the remote branch has commits you don't have yet. Pull them in, then push again.\n\n{message}"
            ));
        }
        return Err(or_fallback(&message, "Push failed."));
    }
    Ok(or_fallback(
        &combined_message(&output),
        "Pushed current branch",
    ))
}

/// Soft-reset HEAD to the published tip so unpushed commits become staged changes.
pub fn reset_unpushed_commits(git: &Path, repo: &Path) -> Result<String, String> {
    if current_operation(repo).is_some() {
        return Err(
            "Cannot undo unpushed commits while a merge or rebase is in progress. Abort it instead."
                .into(),
        );
    }

    let target = published_tip(git, repo)?;
    let (ahead, _) = ahead_behind_for_ref(git, repo, &target);
    if ahead == 0 {
        return Err("No unpushed commits to undo.".into());
    }

    let output = run_git(git, repo, &["reset", "--soft", &target])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to undo unpushed commits.",
        ));
    }

    let label = if ahead == 1 {
        "1 unpushed commit".to_string()
    } else {
        format!("{ahead} unpushed commits")
    };
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Undid {label}. Changes are still staged."),
    ))
}

fn published_tip(git: &Path, repo: &Path) -> Result<String, String> {
    if let Ok(output) = run_git_quiet(git, repo, &["rev-parse", "--abbrev-ref", "@{upstream}"]) {
        if output.success {
            let name = output.stdout.trim();
            if !name.is_empty() {
                return Ok(name.to_string());
            }
        }
    }

    let branch = current_branch(git, repo)?;
    if branch == "HEAD" || branch.starts_with("detached ") {
        return Err("Cannot undo unpushed commits while HEAD is detached.".into());
    }

    let remote = format!("origin/{branch}");
    if ref_exists(git, repo, &format!("refs/remotes/{remote}")) {
        return Ok(remote);
    }

    Err("This branch has no remote to reset to.".into())
}

pub fn checkout_with_fallbacks(
    git: &Path,
    repo: &Path,
    branches: &[String],
) -> Result<String, String> {
    let current = current_branch(git, repo)?;
    if branches.first().is_some_and(|target| target == &current) {
        return Ok(format!("Already on {current}"));
    }

    let fetch = run_git(git, repo, &["fetch", "--all", "--prune"])?;
    let fetch_note = if fetch.success {
        String::new()
    } else {
        format!("Fetch warning: {}\n", combined_message(&fetch))
    };

    for branch in branches {
        validate_ref(branch)?;
        if branch == &current {
            return Ok(format!("{fetch_note}Already on {branch}"));
        }
        let local = format!("refs/heads/{branch}");
        if ref_exists(git, repo, &local) {
            let output = run_git(git, repo, &["checkout", branch])?;
            if output.success {
                return Ok(format!("{fetch_note}Checked out {branch}"));
            }
            return Err(format!(
                "{fetch_note}{}",
                or_fallback(&combined_message(&output), &format!("Failed to check out {branch}"))
            ));
        }

        let remote = format!("refs/remotes/origin/{branch}");
        if ref_exists(git, repo, &remote) {
            let remote_ref = format!("origin/{branch}");
            let output = run_git(git, repo, &["checkout", "-B", branch, &remote_ref])?;
            if output.success {
                return Ok(format!(
                    "{fetch_note}Checked out {branch} from origin/{branch}"
                ));
            }
            return Err(format!(
                "{fetch_note}{}",
                or_fallback(
                    &combined_message(&output),
                    &format!("Failed to check out origin/{branch}")
                )
            ));
        }
    }

    Err(format!(
        "{fetch_note}None of these branches exist locally or on origin: {}",
        branches.join(", ")
    ))
}

pub const NOT_FAST_FORWARD_PREFIX: &str = "Not a fast-forward:";
pub const LOCAL_BRANCH_EXISTS_PREFIX: &str = "Local branch exists:";

const REMOTE_FETCH_TIMEOUT: Duration = Duration::from_secs(60);

pub fn validate_remote_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("Remote names must be 1 to 64 characters.".into());
    }
    if name.starts_with('-') || name.starts_with('.') || name.ends_with(".lock") {
        return Err(format!("Invalid remote name: {name}"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(format!(
            "Invalid remote name: {name}. Use letters, numbers, dots, dashes, or underscores."
        ));
    }
    Ok(())
}

fn validate_remote_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("Enter a remote URL.".into());
    }
    if url.len() > 2048 || url.starts_with('-') || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("That doesn't look like a valid remote URL.".into());
    }
    Ok(())
}

fn remote_exists(git: &Path, repo: &Path, name: &str) -> bool {
    run_git_quiet(git, repo, &["remote"])
        .map(|output| output.success && output.stdout.lines().any(|line| line.trim() == name))
        .unwrap_or(false)
}

fn require_remote(git: &Path, repo: &Path, name: &str) -> Result<(), String> {
    validate_remote_name(name)?;
    if !remote_exists(git, repo, name) {
        return Err(format!("There is no remote named {name}."));
    }
    Ok(())
}

fn remote_sort_rank(name: &str) -> u8 {
    match name {
        "origin" => 0,
        "upstream" => 1,
        _ => 2,
    }
}

pub fn list_remotes(git: &Path, repo: &Path) -> Result<Vec<RemoteEntry>, String> {
    let output = run_git(git, repo, &["remote", "-v"])?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), "Could not list remotes."));
    }
    let mut remotes: Vec<RemoteEntry> = Vec::new();
    for line in output.stdout.lines() {
        let mut parts = line.split_whitespace();
        let (Some(name), Some(url), Some(kind)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let index = match remotes.iter().position(|remote| remote.name == name) {
            Some(index) => index,
            None => {
                remotes.push(RemoteEntry {
                    name: name.to_string(),
                    fetch_url: String::new(),
                    push_url: String::new(),
                    browse_url: None,
                    branch_count: 0,
                });
                remotes.len() - 1
            }
        };
        let entry = &mut remotes[index];
        if kind == "(push)" {
            entry.push_url = url.to_string();
        } else {
            entry.fetch_url = url.to_string();
        }
    }

    let refs = run_git_quiet(
        git,
        repo,
        &["for-each-ref", "--format=%(refname)%00%(symref)", "refs/remotes"],
    )?;
    for entry in &mut remotes {
        if entry.push_url.is_empty() {
            entry.push_url = entry.fetch_url.clone();
        }
        entry.browse_url = remote_browse_url(&entry.fetch_url).ok();
        let prefix = format!("refs/remotes/{}/", entry.name);
        entry.branch_count = refs
            .stdout
            .lines()
            .filter(|line| {
                let mut parts = line.split('\0');
                let refname = parts.next().unwrap_or("");
                let symref = parts.next().unwrap_or("");
                refname.starts_with(&prefix) && symref.is_empty()
            })
            .count() as u32;
    }
    remotes.sort_by(|left, right| {
        remote_sort_rank(&left.name)
            .cmp(&remote_sort_rank(&right.name))
            .then(left.name.cmp(&right.name))
    });
    Ok(remotes)
}

pub fn fetch_named_remote(git: &Path, repo: &Path, name: &str) -> Result<String, String> {
    require_remote(git, repo, name)?;
    let output = run_network_git(
        git,
        repo,
        &["fetch", "--prune", "--no-tags", name],
        REMOTE_FETCH_TIMEOUT,
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Could not fetch {name}."),
        ));
    }
    Ok(format!("Fetched {name}"))
}

pub fn add_remote(git: &Path, repo: &Path, name: &str, url: &str) -> Result<String, String> {
    let url = url.trim();
    validate_remote_name(name)?;
    validate_remote_url(url)?;
    if remote_exists(git, repo, name) {
        return Err(format!("A remote named {name} already exists."));
    }
    let output = run_git(git, repo, &["remote", "add", name, url])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to add remote {name}."),
        ));
    }
    match fetch_named_remote(git, repo, name) {
        Ok(_) => Ok(format!("Added {name} and fetched its branches")),
        Err(err) => Ok(format!(
            "Added {name}, but fetching it failed. Check the URL and your access, then fetch again.\n\n{err}"
        )),
    }
}

pub fn update_remote(
    git: &Path,
    repo: &Path,
    name: &str,
    new_name: &str,
    url: &str,
) -> Result<String, String> {
    let url = url.trim();
    require_remote(git, repo, name)?;
    validate_remote_name(new_name)?;
    validate_remote_url(url)?;
    let mut changes = Vec::new();
    if new_name != name {
        if remote_exists(git, repo, new_name) {
            return Err(format!("A remote named {new_name} already exists."));
        }
        let output = run_git(git, repo, &["remote", "rename", name, new_name])?;
        if !output.success {
            return Err(or_fallback(
                &combined_message(&output),
                &format!("Failed to rename remote {name}."),
            ));
        }
        changes.push(format!("Renamed {name} to {new_name}"));
    }
    let current_url = run_git_quiet(git, repo, &["remote", "get-url", new_name])
        .map(|output| output.stdout.trim().to_string())
        .unwrap_or_default();
    if current_url != url {
        let output = run_git(git, repo, &["remote", "set-url", new_name, url])?;
        if !output.success {
            return Err(or_fallback(
                &combined_message(&output),
                &format!("Failed to change the URL for {new_name}."),
            ));
        }
        changes.push(format!("Updated the URL for {new_name}"));
    }
    if changes.is_empty() {
        return Ok(format!("{name} is unchanged"));
    }
    Ok(changes.join(". "))
}

fn local_upstreams(git: &Path, repo: &Path) -> Vec<(String, String)> {
    let output = match run_git_quiet(
        git,
        repo,
        &["for-each-ref", "--format=%(refname:short)%00%(upstream)", "refs/heads"],
    ) {
        Ok(output) if output.success => output,
        _ => return Vec::new(),
    };
    output
        .stdout
        .lines()
        .filter_map(|line| {
            let (name, upstream) = line.split_once('\0')?;
            let name = name.trim();
            (!name.is_empty()).then(|| (name.to_string(), upstream.trim().to_string()))
        })
        .collect()
}

pub fn remove_remote(git: &Path, repo: &Path, name: &str) -> Result<String, String> {
    require_remote(git, repo, name)?;
    let prefix = format!("refs/remotes/{name}/");
    let orphaned: Vec<String> = local_upstreams(git, repo)
        .into_iter()
        .filter(|(_, upstream)| upstream.starts_with(&prefix))
        .map(|(local, _)| local)
        .collect();
    let output = run_git(git, repo, &["remote", "remove", name])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to remove remote {name}."),
        ));
    }
    if orphaned.is_empty() {
        return Ok(format!("Removed remote {name}"));
    }
    Ok(format!(
        "Removed remote {name}. These local branches no longer track anything: {}",
        join_branch_names(&orphaned)
    ))
}

fn remote_default_branch(git: &Path, repo: &Path, remote: &str) -> Option<String> {
    let head = format!("refs/remotes/{remote}/HEAD");
    let output = run_git_quiet(git, repo, &["symbolic-ref", "--quiet", &head]).ok()?;
    if !output.success {
        return None;
    }
    output
        .stdout
        .trim()
        .strip_prefix(&format!("refs/remotes/{remote}/"))
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
}

pub fn remote_branches(git: &Path, repo: &Path, remote: &str) -> Result<RemoteOverview, String> {
    require_remote(git, repo, remote)?;
    let prefix = format!("refs/remotes/{remote}/");
    let pattern = format!("refs/remotes/{remote}");
    let output = run_git(
        git,
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(symref)%00%(objectname:short)%00%(committerdate:iso-strict)%00%(subject)",
            &pattern,
        ],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Could not list branches on {remote}."),
        ));
    }

    let locals = local_upstreams(git, repo);
    let local_names: HashSet<&str> = locals.iter().map(|(name, _)| name.as_str()).collect();
    let current = head_branch_name(git, repo).unwrap_or_default();
    let default_branch = remote_default_branch(git, repo, remote);

    let mut branches = Vec::new();
    for line in output.stdout.lines() {
        let mut parts = line.split('\0');
        let refname = parts.next().unwrap_or("");
        let symref = parts.next().unwrap_or("");
        let hash = parts.next().unwrap_or("").trim();
        let date = parts.next().unwrap_or("").trim();
        let subject = parts.next().unwrap_or("").trim();
        let Some(name) = refname.strip_prefix(&prefix) else {
            continue;
        };
        if name.is_empty() || !symref.is_empty() {
            continue;
        }
        let tracker = locals
            .iter()
            .find(|(_, upstream)| upstream == refname)
            .map(|(local, _)| local.clone());
        let tracked = tracker.is_some();
        let local = tracker.or_else(|| local_names.contains(name).then(|| name.to_string()));
        let (ahead, behind) = local
            .as_deref()
            .map(|local| {
                ahead_behind_between(git, repo, &format!("refs/heads/{local}"), refname, false)
            })
            .unwrap_or((0, 0));
        branches.push(RemoteBranch {
            name: name.to_string(),
            remote: remote.to_string(),
            hash: hash.to_string(),
            date: date.to_string(),
            subject: subject.to_string(),
            is_default: default_branch.as_deref() == Some(name),
            current: local.as_deref().is_some_and(|local| local == current),
            local,
            tracked,
            ahead,
            behind,
        });
    }
    branches.sort_by(|left, right| {
        right
            .is_default
            .cmp(&left.is_default)
            .then(left.name.cmp(&right.name))
    });
    Ok(RemoteOverview {
        remote: remote.to_string(),
        default_branch,
        branches,
    })
}

fn require_remote_branch(git: &Path, repo: &Path, remote: &str, branch: &str) -> Result<String, String> {
    require_remote(git, repo, remote)?;
    validate_ref(branch)?;
    let full = format!("refs/remotes/{remote}/{branch}");
    if !ref_exists(git, repo, &full) {
        return Err(format!(
            "{remote}/{branch} does not exist. Fetch {remote} and try again."
        ));
    }
    Ok(full)
}

fn is_ancestor(git: &Path, repo: &Path, ancestor: &str, descendant: &str) -> bool {
    run_git_quiet(git, repo, &["merge-base", "--is-ancestor", ancestor, descendant])
        .map(|output| output.success)
        .unwrap_or(false)
}

pub fn checkout_remote_branch(
    git: &Path,
    repo: &Path,
    remote: &str,
    branch: &str,
    local_name: Option<&str>,
) -> Result<String, String> {
    let full = require_remote_branch(git, repo, remote, branch)?;
    require_no_operation(repo)?;
    let local_name = local_name.map(str::trim).filter(|name| !name.is_empty());
    if local_name.is_none() {
        if let Some((tracker, _)) = local_upstreams(git, repo)
            .into_iter()
            .find(|(_, upstream)| *upstream == full)
        {
            return checkout_local_branch(git, repo, &tracker);
        }
    }
    let name = local_name.unwrap_or(branch);
    validate_ref(name)?;
    if ref_exists(git, repo, &format!("refs/heads/{name}")) {
        return Err(format!(
            "{LOCAL_BRANCH_EXISTS_PREFIX} a local branch named {name} already exists and doesn't track {remote}/{branch}. Pick another name."
        ));
    }
    let source = format!("{remote}/{branch}");
    let output = run_git(git, repo, &["checkout", "-b", name, "--track", &source])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to check out {source}"),
        ));
    }
    Ok(format!("Checked out {name} tracking {source}"))
}

/// Brings a remote branch into a local branch. Fast-forwards when possible; a merge commit
/// is only made when `allow_merge_commit` is set. Never rewrites the target's history.
pub fn merge_remote_branch(
    git: &Path,
    repo: &Path,
    remote: &str,
    branch: &str,
    target: &str,
    allow_merge_commit: bool,
) -> Result<String, String> {
    let full = require_remote_branch(git, repo, remote, branch)?;
    validate_ref(target)?;
    require_no_operation(repo)?;
    let target_ref = format!("refs/heads/{target}");
    if !ref_exists(git, repo, &target_ref) {
        return Err(format!("Local branch {target} does not exist."));
    }
    let source = format!("{remote}/{branch}");
    if is_ancestor(git, repo, &full, &target_ref) {
        return Ok(format!("{target} already has everything in {source}"));
    }
    let fast_forward = is_ancestor(git, repo, &target_ref, &full);
    if !fast_forward && !allow_merge_commit {
        return Err(format!(
            "{NOT_FAST_FORWARD_PREFIX} {target} has commits that aren't in {source}, so it can't fast-forward. Merging makes a merge commit on {target}."
        ));
    }

    let on_target = head_branch_name(git, repo).as_deref() == Some(target);
    if fast_forward && !on_target {
        // Updates the branch without checking it out; git refuses if another worktree has it.
        let refspec = format!("{full}:{target_ref}");
        let output = run_git(git, repo, &["fetch", "--no-tags", ".", &refspec])?;
        if !output.success {
            return Err(or_fallback(
                &combined_message(&output),
                &format!("Failed to fast-forward {target} to {source}."),
            ));
        }
        return Ok(format!("Fast-forwarded {target} to {source}"));
    }

    if !on_target {
        checkout_local_branch(git, repo, target)?;
    }
    let args: Vec<&str> = if fast_forward {
        vec!["merge", "--ff-only", &source]
    } else {
        vec!["merge", "--no-edit", &source]
    };
    let output = run_git(git, repo, &args)?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to merge {source} into {target}."),
        ));
    }
    Ok(if fast_forward {
        format!("Fast-forwarded {target} to {source}")
    } else {
        format!("Merged {source} into {target}")
    })
}

/// Standard push of one local branch to the remote it tracks, or to origin with `-u` when it
/// tracks nothing. Never adds `--force` or `+` refspecs.
pub fn push_local_branch(git: &Path, repo: &Path, branch: &str) -> Result<String, String> {
    validate_ref(branch)?;
    if !ref_exists(git, repo, &format!("refs/heads/{branch}")) {
        return Err(format!("Local branch {branch} does not exist."));
    }
    let config = |key: String| {
        run_git_quiet(git, repo, &["config", "--get", &key])
            .ok()
            .filter(|output| output.success)
            .map(|output| output.stdout.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    let upstream_remote = config(format!("branch.{branch}.remote")).filter(|remote| remote != ".");
    let upstream_merge = config(format!("branch.{branch}.merge"));
    let (remote, output) = match (upstream_remote, upstream_merge) {
        (Some(remote), Some(merge)) => {
            let refspec = format!("refs/heads/{branch}:{merge}");
            let output = run_git(git, repo, &["push", &remote, &refspec])?;
            (remote, output)
        }
        _ => {
            if !has_origin_remote(git, repo) {
                return Err(format!("{branch} doesn't track a remote and there is no origin to push to."));
            }
            let output = run_git(git, repo, &["push", "-u", "origin", branch])?;
            ("origin".to_string(), output)
        }
    };
    if !output.success {
        let message = combined_message(&output);
        if is_non_fast_forward(&message) {
            return Err(format!(
                "{PUSH_REJECTED_PREFIX} {remote} has commits on {branch} you don't have yet. Pull them in, then push again.\n\n{message}"
            ));
        }
        return Err(or_fallback(&message, &format!("Failed to push {branch}.")));
    }
    Ok(format!("Pushed {branch} to {remote}"))
}

/// Deletes the branch on the remote server. The remote's default branch is refused.
pub fn delete_remote_branch(git: &Path, repo: &Path, remote: &str, branch: &str) -> Result<String, String> {
    require_remote_branch(git, repo, remote, branch)?;
    if remote_default_branch(git, repo, remote).as_deref() == Some(branch) {
        return Err(format!(
            "{branch} is the default branch on {remote}. Change the default on the server before deleting it."
        ));
    }
    let target = format!("refs/heads/{branch}");
    let output = run_network_git(
        git,
        repo,
        &["push", remote, "--delete", &target],
        REMOTE_FETCH_TIMEOUT,
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to delete {remote}/{branch}."),
        ));
    }
    let _ = run_git_quiet(
        git,
        repo,
        &["update-ref", "-d", &format!("refs/remotes/{remote}/{branch}")],
    );
    Ok(format!("Deleted {branch} on {remote}"))
}

pub fn log_graph(git: &Path, repo: &Path) -> Result<Vec<CommitNode>, String> {
    let output = run_git(
        git,
        repo,
        &[
            "log",
            "--all",
            "--max-count=400",
            "--pretty=format:%H%x1f%P%x1f%s%x1f%an%x1f%aI%x1f%D",
        ],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the commit log.",
        ));
    }

    Ok(parse_log_commits(&output.stdout))
}

fn parse_log_commits(stdout: &str) -> Vec<CommitNode> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_log_commit_line)
        .collect()
}

fn parse_log_commit_line(line: &str) -> Option<CommitNode> {
    let mut parts = line.split('\u{1f}');
    let hash = parts.next()?.to_string();
    if hash.is_empty() {
        return None;
    }
    let parents = parts
        .next()?
        .split_whitespace()
        .filter(|parent| !parent.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    Some(CommitNode {
        hash,
        parents,
        subject: parts.next().unwrap_or_default().to_string(),
        author: parts.next().unwrap_or_default().to_string(),
        date: parts.next().unwrap_or_default().to_string(),
        refs: parts.next().unwrap_or_default().to_string(),
        path: None,
        old_path: None,
        status: None,
    })
}

fn parse_file_log_commits(stdout: &str) -> Vec<CommitNode> {
    let mut commits = Vec::new();
    let mut current: Option<CommitNode> = None;
    for line in stdout.lines() {
        if line.contains('\u{1f}') {
            if let Some(commit) = current.take() {
                commits.push(commit);
            }
            current = parse_log_commit_line(line);
            continue;
        }
        let Some(file) = parse_name_status_line(line) else {
            continue;
        };
        if let Some(commit) = current.as_mut() {
            if commit.path.is_none() {
                commit.path = Some(file.path);
                commit.old_path = file.old_path;
                commit.status = Some(file.status);
            }
        }
    }
    if let Some(commit) = current {
        commits.push(commit);
    }
    commits
}

pub fn file_log(git: &Path, repo: &Path, file: &str) -> Result<Vec<CommitNode>, String> {
    require_file_path(file)?;
    let output = run_git(
        git,
        repo,
        &[
            "log",
            "--follow",
            "--find-renames",
            "--find-copies",
            "--max-count=400",
            "--pretty=format:%H%x1f%P%x1f%s%x1f%an%x1f%aI%x1f%D",
            "--name-status",
            "--",
            file,
        ],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the file history.",
        ));
    }
    Ok(parse_file_log_commits(&output.stdout))
}

pub fn discard_all_changes(git: &Path, repo: &Path) -> Result<(), String> {
    if current_operation(repo).is_some() {
        return Err(
            "Cannot discard all changes while a merge or rebase is in progress. Abort it instead."
                .into(),
        );
    }
    let reset = run_git(git, repo, &["reset", "--hard", "HEAD"])?;
    if !reset.success {
        return Err(or_fallback(
            &combined_message(&reset),
            "Failed to discard tracked changes.",
        ));
    }
    let clean = run_git(git, repo, &["clean", "-fd"])?;
    if !clean.success {
        return Err(or_fallback(
            &combined_message(&clean),
            "Failed to remove untracked files.",
        ));
    }
    Ok(())
}

pub fn discard_file_changes(
    git: &Path,
    repo: &Path,
    file: &str,
    staged: bool,
) -> Result<(), String> {
    require_file_path(file)?;
    if is_git_internal(file) {
        return Err("Cannot discard git internals.".into());
    }
    if current_operation(repo).is_some() {
        return Err(
            "Cannot discard changes while a merge or rebase is in progress. Abort it instead."
                .into(),
        );
    }

    let files = working_tree(git, repo)?;
    let entry = files
        .iter()
        .find(|entry| entry.path == file && entry.staged == staged)
        .ok_or_else(|| "Nothing to discard for that file.".to_string())?;

    if !staged {
        if entry.untracked {
            return remove_worktree_file(repo, file);
        }
        return restore_worktree(git, repo, file);
    }

    let has_unstaged = files
        .iter()
        .any(|entry| entry.path == file && !entry.staged);
    if !committed_in_head(git, repo, file) {
        let source = staged_rename_source(git, repo, file)?;
        return discard_staged_addition(git, repo, file, source, has_unstaged);
    }

    discard_staged_tracked(git, repo, file, has_unstaged)
}

fn remove_worktree_file(repo: &Path, file: &str) -> Result<(), String> {
    let path = repo_file_path(repo, file)?;
    if path.is_dir() {
        return Err("That path is a folder.".into());
    }
    if !path.exists() {
        return Err("That file is not on disk.".into());
    }
    fs::remove_file(&path).map_err(|err| format!("Could not discard the file: {err}"))
}

fn restore_worktree(git: &Path, repo: &Path, file: &str) -> Result<(), String> {
    let output = run_git(git, repo, &["restore", "--worktree", "--", file])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to discard changes.",
        ));
    }
    Ok(())
}

fn restore_from_head(git: &Path, repo: &Path, file: &str, worktree: bool) -> Result<(), String> {
    let output = if worktree {
        run_git(
            git,
            repo,
            &["restore", "--source=HEAD", "--staged", "--worktree", "--", file],
        )?
    } else {
        run_git(
            git,
            repo,
            &["restore", "--source=HEAD", "--staged", "--", file],
        )?
    };
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to discard changes.",
        ));
    }
    Ok(())
}

fn discard_staged_addition(
    git: &Path,
    repo: &Path,
    file: &str,
    rename_from: Option<String>,
    has_unstaged: bool,
) -> Result<(), String> {
    let path = repo_file_path(repo, file)?;
    if has_unstaged && path.is_file() {
        return Err(unstaged_edits_error());
    }
    if let Some(old) = &rename_from {
        ensure_rename_source_restorable(git, repo, old)?;
    }

    let snapshot = if path.is_file() {
        Some(fs::read(&path).map_err(discard_io_error)?)
    } else {
        None
    };

    if has_unstaged {
        remove_index_entry(git, repo, file)?;
    } else {
        delete_staged_new_file(git, repo, file)?;
    }

    if let Some(old) = &rename_from {
        if let Err(err) = restore_from_head(git, repo, old, true) {
            rollback_staged_new_file(git, repo, file, snapshot.as_deref());
            return Err(err);
        }
    }
    Ok(())
}

fn delete_staged_new_file(git: &Path, repo: &Path, file: &str) -> Result<(), String> {
    let removed = run_git(git, repo, &["rm", "-f", "--", file])?;
    if removed.success {
        return Ok(());
    }
    let cached = run_git(git, repo, &["rm", "-f", "--cached", "--", file])?;
    if !cached.success {
        return Err(or_fallback(
            &combined_message(&removed),
            "Failed to discard changes.",
        ));
    }
    let path = repo_file_path(repo, file)?;
    if path.is_file() {
        fs::remove_file(&path).map_err(|err| format!("Could not discard the file: {err}"))?;
    }
    Ok(())
}

fn remove_index_entry(git: &Path, repo: &Path, file: &str) -> Result<(), String> {
    let output = run_git(git, repo, &["rm", "--cached", "-f", "--", file])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to discard changes.",
        ));
    }
    Ok(())
}

fn ensure_rename_source_restorable(git: &Path, repo: &Path, old: &str) -> Result<(), String> {
    let path = repo_file_path(repo, old)?;
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(discard_io_error(err)),
    };
    let spec = format!("HEAD:{old}");
    if !meta.is_file()
        || git_object_id(git, repo, &["hash-object", "--", old])?
            != git_object_id(git, repo, &["rev-parse", "--verify", &spec])?
    {
        return Err(format!(
            "Could not discard the rename because {old} has changes that would be overwritten."
        ));
    }
    Ok(())
}

fn git_object_id(git: &Path, repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = run_git(git, repo, args)?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not discard changes.",
        ));
    }
    Ok(output.stdout.trim().to_string())
}

fn unstaged_overlap_error() -> String {
    "Could not discard staged changes without losing nearby unstaged edits. Stage or discard those edits first.".into()
}

fn unstaged_edits_error() -> String {
    "This file has unstaged edits. Stage or discard them before discarding the staged file.".into()
}

fn discard_io_error(err: std::io::Error) -> String {
    format!("Could not discard changes: {err}")
}

fn rollback_staged_new_file(git: &Path, repo: &Path, file: &str, snapshot: Option<&[u8]>) {
    let Some(bytes) = snapshot else {
        return;
    };
    let Ok(path) = repo_file_path(repo, file) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if fs::write(&path, bytes).is_ok() {
        let _ = run_git(git, repo, &["add", "--", file]);
    }
}

fn discard_staged_tracked(
    git: &Path,
    repo: &Path,
    file: &str,
    has_unstaged: bool,
) -> Result<(), String> {
    if !has_unstaged {
        return restore_from_head(git, repo, file, true);
    }
    let path = repo_file_path(repo, file)?;
    if !path.is_file() || is_staged_deletion(git, repo, file)? {
        return restore_from_head(git, repo, file, false);
    }

    let snapshot = fs::read(&path).map_err(discard_io_error)?;
    let merged = merge_out_staged_changes(git, repo, file, &snapshot)?;
    fs::write(&path, &merged).map_err(discard_io_error)?;
    if let Err(err) = restore_from_head(git, repo, file, false) {
        let _ = fs::write(&path, snapshot);
        return Err(err);
    }
    Ok(())
}

fn is_staged_deletion(git: &Path, repo: &Path, file: &str) -> Result<bool, String> {
    let output = run_git(
        git,
        repo,
        &[
            "diff",
            "--cached",
            "--name-only",
            "--diff-filter=D",
            "-z",
            "--",
            file,
        ],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not discard changes.",
        ));
    }
    Ok(output.stdout.split('\0').any(|path| path == file))
}

fn staged_rename_source(git: &Path, repo: &Path, file: &str) -> Result<Option<String>, String> {
    let output = run_git(
        git,
        repo,
        &["diff", "--cached", "--name-status", "-M", "-z"],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not discard changes.",
        ));
    }
    let parts: Vec<&str> = output.stdout.split('\0').collect();
    let mut index = 0;
    while index < parts.len() {
        let status = parts[index];
        if status.is_empty() {
            break;
        }
        let renamed = status.starts_with('R') || status.starts_with('C');
        if renamed {
            let old = parts.get(index + 1).copied().unwrap_or("");
            let new = parts.get(index + 2).copied().unwrap_or("");
            if status.starts_with('R') && new == file {
                return Ok(Some(old.to_string()));
            }
            index += 3;
        } else {
            index += 2;
        }
    }
    Ok(None)
}

fn merge_out_staged_changes(
    git: &Path,
    repo: &Path,
    file: &str,
    worktree: &[u8],
) -> Result<Vec<u8>, String> {
    let dir = std::env::temp_dir().join(format!(
        "shipyard-discard-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&dir).map_err(discard_io_error)?;
    let result = merge_out_staged_changes_in(git, repo, file, worktree, &dir);
    let _ = fs::remove_dir_all(&dir);
    result
}

fn merge_out_staged_changes_in(
    git: &Path,
    repo: &Path,
    file: &str,
    worktree: &[u8],
    dir: &Path,
) -> Result<Vec<u8>, String> {
    let current = dir.join("worktree");
    let base = dir.join("index");
    let other = dir.join("head");
    fs::write(&current, worktree).map_err(discard_io_error)?;
    let index_spec = format!(":{file}");
    let head_spec = format!("HEAD:{file}");
    run_git_to_file(git, repo, &["cat-file", "--filters", &index_spec], &base)?;
    run_git_to_file(git, repo, &["cat-file", "--filters", &head_spec], &other)?;

    let [current_arg, base_arg, other_arg] = [&current, &base, &other].map(|path| path.to_str());
    let (Some(current_arg), Some(base_arg), Some(other_arg)) = (current_arg, base_arg, other_arg)
    else {
        return Err("Could not discard changes.".into());
    };
    // Replays the index -> HEAD change onto the worktree copy; conflicts and binary files exit non-zero.
    let merged = run_git(
        git,
        repo,
        &["merge-file", "-q", current_arg, base_arg, other_arg],
    )?;
    if !merged.success {
        return Err(unstaged_overlap_error());
    }
    fs::read(&current).map_err(discard_io_error)
}

fn run_git_to_file(git: &Path, repo: &Path, args: &[&str], dest: &Path) -> Result<(), String> {
    let started = Instant::now();
    let file = fs::File::create(dest).map_err(discard_io_error)?;
    let result = Command::new(git)
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdout(Stdio::from(file))
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("Failed to run git: {err}"));
    match &result {
        Ok(output) => command_log::record(
            repo,
            git,
            args,
            output.status.success(),
            started.elapsed(),
            "",
            &String::from_utf8_lossy(&output.stderr),
        ),
        Err(err) => command_log::record(repo, git, args, false, started.elapsed(), "", err),
    }
    let output = result?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(or_fallback(stderr.trim(), "Could not discard changes."));
    }
    Ok(())
}

pub fn last_commit(git: &Path, repo: &Path) -> Result<LastCommit, String> {
    let output = run_git(git, repo, &["log", "-1", "--pretty=format:%s%x1f%b"])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the last commit.",
        ));
    }
    if output.stdout.is_empty() {
        return Err("No commits yet.".into());
    }

    let mut parts = output.stdout.splitn(2, '\u{1f}');
    let title = parts.next().unwrap_or_default().to_string();
    let description = parts.next().unwrap_or_default().trim().to_string();
    Ok(LastCommit {
        title,
        description,
        published: last_commit_is_published(git, repo),
    })
}

fn last_commit_is_published(git: &Path, repo: &Path) -> bool {
    let upstream = match run_git_quiet(git, repo, &["rev-parse", "--abbrev-ref", "@{upstream}"]) {
        Ok(output) if output.success => output.stdout.trim().to_string(),
        _ => return false,
    };
    if upstream.is_empty() {
        return false;
    }
    let spec = format!("HEAD...{upstream}");
    let output = match run_git_quiet(git, repo, &["rev-list", "--left-right", "--count", &spec]) {
        Ok(output) if output.success => output,
        _ => return false,
    };
    output
        .stdout
        .split_whitespace()
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0)
        == 0
}

pub fn commit(
    git: &Path,
    repo: &Path,
    title: &str,
    description: &str,
    amend: bool,
) -> Result<String, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Enter a commit title.".into());
    }
    if title.chars().count() > 72 {
        return Err("Commit title must be 72 characters or fewer.".into());
    }
    if title.contains('\0') || description.contains('\0') {
        return Err("Invalid commit message.".into());
    }

    if amend {
        let head = run_git_quiet(git, repo, &["rev-parse", "--verify", "HEAD"])?;
        if !head.success {
            return Err("Nothing to amend.".into());
        }
    }

    let staged = run_git(git, repo, &["diff", "--cached", "--quiet"])?;
    if staged.success {
        return Err(if amend {
            "Nothing is staged to amend.".into()
        } else {
            "Nothing is staged to commit.".into()
        });
    }

    let description = description.trim();
    let output = match (amend, description.is_empty()) {
        (false, true) => run_git(git, repo, &["commit", "-m", title])?,
        (false, false) => run_git(git, repo, &["commit", "-m", title, "-m", description])?,
        (true, true) => run_git(git, repo, &["commit", "--amend", "-m", title])?,
        (true, false) => {
            run_git(git, repo, &["commit", "--amend", "-m", title, "-m", description])?
        }
    };
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            if amend {
                "Amend failed."
            } else {
                "Commit failed."
            },
        ));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("{} {title}", if amend { "Amended" } else { "Committed" }),
    ))
}

fn porcelain_path(rest: &str) -> Option<String> {
    let path = rest
        .trim()
        .split(" -> ")
        .last()
        .unwrap_or(rest)
        .trim_matches('"')
        .to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn describe_letter(letter: char) -> String {
    match letter {
        'M' => "Modified".into(),
        'A' => "Added".into(),
        'D' => "Deleted".into(),
        'R' => "Renamed".into(),
        'C' => "Copied".into(),
        'T' => "Type changed".into(),
        'U' => "Conflicted".into(),
        '?' => "Untracked".into(),
        other => other.to_string(),
    }
}

fn validate_commit_hash(hash: &str) -> Result<(), String> {
    if hash.len() < 7 || hash.len() > 64 {
        return Err("Invalid commit.".into());
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid commit.".into());
    }
    Ok(())
}

fn require_commit(git: &Path, repo: &Path, hash: &str) -> Result<String, String> {
    validate_commit_hash(hash)?;
    let spec = format!("{hash}^{{commit}}");
    let output = run_git(git, repo, &["rev-parse", "--verify", "--quiet", &spec])?;
    let resolved = output.stdout.trim();
    if !output.success || resolved.is_empty() {
        return Err("Commit not found.".into());
    }
    Ok(resolved.to_string())
}

fn first_parent(git: &Path, repo: &Path, hash: &str) -> Result<Option<String>, String> {
    let spec = format!("{hash}^");
    let output = run_git(git, repo, &["rev-parse", "--verify", "--quiet", &spec])?;
    let resolved = output.stdout.trim();
    if !output.success || resolved.is_empty() {
        return Ok(None);
    }
    Ok(Some(resolved.to_string()))
}

fn parse_name_status_line(line: &str) -> Option<CommitFile> {
    let mut parts = line.split('\t');
    let status = parts.next()?.trim();
    let letter = status.chars().find(|c| c.is_ascii_alphabetic())?;
    let first_path = name_status_path(parts.next()?);
    if first_path.is_empty() {
        return None;
    }
    let second_path = parts.next().map(name_status_path).filter(|path| !path.is_empty());
    let (path, old_path) = match second_path {
        Some(new_path) => (new_path, Some(first_path)),
        None => (first_path, None),
    };
    Some(CommitFile {
        path,
        old_path,
        status: describe_letter(letter),
    })
}

fn name_status_path(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

pub fn commit_files(git: &Path, repo: &Path, hash: &str) -> Result<Vec<CommitFile>, String> {
    let hash = require_commit(git, repo, hash)?;
    let parent = first_parent(git, repo, &hash)?;
    let output = if let Some(parent) = parent.as_deref() {
        run_git(
            git,
            repo,
            &[
                "diff-tree",
                "--no-commit-id",
                "--name-status",
                "-r",
                "--find-renames",
                parent,
                &hash,
            ],
        )?
    } else {
        run_git(
            git,
            repo,
            &[
                "diff-tree",
                "--no-commit-id",
                "--name-status",
                "-r",
                "--find-renames",
                "--root",
                &hash,
            ],
        )?
    };
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the commit files.",
        ));
    }

    Ok(output
        .stdout
        .lines()
        .filter_map(parse_name_status_line)
        .collect())
}

fn stash_ref(index: u32) -> String {
    format!("stash@{{{index}}}")
}

fn parse_stash_index(selector: &str) -> Option<u32> {
    let start = selector.rfind('{')?;
    let end = selector.rfind('}')?;
    if end <= start + 1 {
        return None;
    }
    selector[start + 1..end].parse().ok()
}

fn parse_stash_line(line: &str) -> Option<StashEntry> {
    let mut parts = line.split('\u{1f}');
    let selector = parts.next()?.trim();
    let index = parse_stash_index(selector)?;
    Some(StashEntry {
        index,
        message: parts.next().unwrap_or_default().to_string(),
        date: parts.next().unwrap_or_default().to_string(),
    })
}

fn missing_stash_ref(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("refs/stash") && (lower.contains("unknown revision") || lower.contains("bad revision") || lower.contains("does not exist") || lower.contains("needed a single revision"))
}

pub fn stash_list(git: &Path, repo: &Path) -> Result<Vec<StashEntry>, String> {
    let output = run_git(
        git,
        repo,
        &["stash", "list", "--pretty=format:%gd%x1f%gs%x1f%aI"],
    )?;
    if !output.success {
        if missing_stash_ref(&combined_message(&output)) {
            return Ok(Vec::new());
        }
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the stash list.",
        ));
    }
    Ok(output
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_stash_line)
        .collect())
}

fn stash_action(
    git: &Path,
    repo: &Path,
    action: &str,
    index: u32,
    fallback: &str,
    success: &str,
) -> Result<String, String> {
    let spec = stash_ref(index);
    let output = run_git(git, repo, &["stash", action, &spec])?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), fallback));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("{success} {spec}"),
    ))
}

pub fn stash_apply(git: &Path, repo: &Path, index: u32) -> Result<String, String> {
    stash_action(
        git,
        repo,
        "apply",
        index,
        "Failed to apply the stash.",
        "Applied",
    )
}

pub fn stash_pop(git: &Path, repo: &Path, index: u32) -> Result<String, String> {
    stash_action(
        git,
        repo,
        "pop",
        index,
        "Failed to pop the stash.",
        "Popped",
    )
}

pub fn stash_drop(git: &Path, repo: &Path, index: u32) -> Result<String, String> {
    stash_action(
        git,
        repo,
        "drop",
        index,
        "Failed to drop the stash.",
        "Dropped",
    )
}

pub fn stash_push(git: &Path, repo: &Path, message: &str) -> Result<String, String> {
    if message.contains('\0') {
        return Err("Invalid stash message.".into());
    }
    let files = working_tree(git, repo)?;
    if files.is_empty() {
        return Err("Nothing to stash.".into());
    }
    let message = message.trim();
    let output = if message.is_empty() {
        run_git(git, repo, &["stash", "push", "--include-untracked"])?
    } else {
        run_git(
            git,
            repo,
            &["stash", "push", "--include-untracked", "-m", message],
        )?
    };
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to stash changes.",
        ));
    }
    let combined = combined_message(&output);
    if combined.to_ascii_lowercase().contains("no local changes") {
        return Err("Nothing to stash.".into());
    }
    Ok(or_fallback(&combined, "Stashed changes"))
}

fn parse_tag_line(line: &str) -> Option<TagEntry> {
    let mut parts = line.split('\0');
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    let object = parts.next().unwrap_or("").trim();
    let peeled = parts.next().unwrap_or("").trim();
    let date = parts.next().unwrap_or("").trim();
    let message = parts.next().unwrap_or("").trim();
    let annotated = !peeled.is_empty();
    let hash = if annotated { peeled } else { object };
    Some(TagEntry {
        name: name.to_string(),
        hash: hash.to_string(),
        date: date.to_string(),
        message: message.to_string(),
        annotated,
    })
}

pub fn tag_list(git: &Path, repo: &Path) -> Result<Vec<TagEntry>, String> {
    let output = run_git(
        git,
        repo,
        &[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)%00%(objectname:short)%00%(*objectname:short)%00%(creatordate:iso-strict)%00%(subject)",
            "refs/tags",
        ],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the tag list.",
        ));
    }
    Ok(output
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(parse_tag_line)
        .collect())
}

fn tag_target_ok(target: &str) -> Result<(), String> {
    if target.is_empty() {
        return Ok(());
    }
    if target.len() > 255 || target.contains('\0') || target.chars().any(|c| c.is_control()) {
        return Err("Invalid tag target.".into());
    }
    Ok(())
}

pub fn create_tag(
    git: &Path,
    repo: &Path,
    name: &str,
    message: &str,
    target: &str,
) -> Result<String, String> {
    validate_tag(name)?;
    if message.contains('\0') {
        return Err("Invalid tag message.".into());
    }
    let target = target.trim();
    tag_target_ok(target)?;
    if ref_exists(git, repo, &format!("refs/tags/{name}")) {
        return Err(format!("Tag {name} already exists."));
    }
    let message = message.trim();
    let output = match (message.is_empty(), target.is_empty()) {
        (true, true) => run_git(git, repo, &["tag", name])?,
        (true, false) => run_git(git, repo, &["tag", name, target])?,
        (false, true) => run_git(git, repo, &["tag", "-a", name, "-m", message])?,
        (false, false) => run_git(git, repo, &["tag", "-a", name, "-m", message, target])?,
    };
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to create tag {name}"),
        ));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Created tag {name}"),
    ))
}

pub fn delete_tag(git: &Path, repo: &Path, name: &str) -> Result<String, String> {
    validate_tag(name)?;
    if !ref_exists(git, repo, &format!("refs/tags/{name}")) {
        return Err(format!("Tag {name} does not exist."));
    }
    let output = run_git(git, repo, &["tag", "-d", name])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            &format!("Failed to delete tag {name}"),
        ));
    }
    Ok(or_fallback(
        &combined_message(&output),
        &format!("Deleted tag {name}"),
    ))
}

fn file_exists_at(git: &Path, repo: &Path, rev: &str, file: &str) -> bool {
    let spec = format!("{rev}:{file}");
    run_git_quiet(git, repo, &["cat-file", "-e", &spec])
        .map(|output| output.success)
        .unwrap_or(false)
}

fn followed_path_at_commit(
    git: &Path,
    repo: &Path,
    hash: &str,
    file: &str,
) -> Result<Option<String>, String> {
    let output = run_git_quiet(
        git,
        repo,
        &[
            "log",
            "--follow",
            "--find-renames",
            "--max-count=400",
            "--pretty=format:%H",
            "--name-status",
            "--",
            file,
        ],
    )?;
    if !output.success {
        return Ok(None);
    }

    let mut current_hash: Option<&str> = None;
    for line in output.stdout.lines() {
        if line.contains('\t') {
            let Some(current_hash) = current_hash else {
                continue;
            };
            if current_hash != hash {
                continue;
            }
            if let Some(file) = parse_name_status_line(line) {
                return Ok(Some(file.path));
            }
            continue;
        }
        if line.len() >= 7 && line.chars().all(|c| c.is_ascii_hexdigit()) {
            current_hash = Some(line);
        }
    }
    Ok(None)
}

fn resolve_commit_file_path(
    git: &Path,
    repo: &Path,
    hash: &str,
    file: &str,
) -> Result<String, String> {
    if file_exists_at(git, repo, hash, file) {
        return Ok(file.to_string());
    }
    if let Some(parent) = first_parent(git, repo, hash)? {
        if file_exists_at(git, repo, &parent, file) {
            return Ok(file.to_string());
        }
    }
    if let Some(followed) = followed_path_at_commit(git, repo, hash, file)? {
        return Ok(followed);
    }
    Ok(file.to_string())
}

pub fn commit_file_diff(git: &Path, repo: &Path, hash: &str, file: &str) -> Result<String, String> {
    require_file_path(file)?;
    let hash = require_commit(git, repo, hash)?;
    let file = resolve_commit_file_path(git, repo, &hash, file)?;
    let parent = first_parent(git, repo, &hash)?;
    let output = if let Some(parent) = parent.as_deref() {
        run_git(
            git,
            repo,
            &["diff", "--find-renames", parent, &hash, "--", &file],
        )?
    } else {
        run_git(
            git,
            repo,
            &[
                "show",
                "--pretty=format:",
                "--find-renames",
                &hash,
                "--",
                &file,
            ],
        )?
    };
    if !output.success && output.stdout.trim().is_empty() {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the commit diff.",
        ));
    }
    if output.stdout.trim().is_empty() {
        return Ok("No changes.".into());
    }
    Ok(output.stdout)
}

#[derive(Clone, Default)]
struct BlameMeta {
    author: String,
    email: String,
    timestamp: u64,
    summary: String,
}

fn uncommitted_blame(contents: &str) -> Vec<BlameLine> {
    if contents.is_empty() {
        return Vec::new();
    }
    contents
        .lines()
        .enumerate()
        .map(|(index, _)| BlameLine {
            line: (index + 1) as u32,
            hash: "0".repeat(40),
            author: "Not Committed Yet".into(),
            email: String::new(),
            timestamp: 0,
            summary: "Uncommitted changes".into(),
        })
        .collect()
}

fn is_uncommitted_line(line: &BlameLine) -> bool {
    line.hash.chars().all(|c| c == '0')
        || line.author == "Not Committed Yet"
        || line.author == "External file (--contents)"
}

fn normalize_blame_line(mut line: BlameLine) -> BlameLine {
    if is_uncommitted_line(&line) {
        if line.author == "External file (--contents)" || line.author.is_empty() {
            line.author = "Not Committed Yet".into();
        }
        if line.summary.is_empty() || line.summary == "External file (--contents)" {
            line.summary = "Not Committed Yet".into();
        }
    }
    line
}

fn config_value(git: &Path, repo: &Path, key: &str) -> String {
    run_git_quiet(git, repo, &["config", "--get", key])
        .ok()
        .filter(|output| output.success)
        .map(|output| output.stdout.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
}

fn current_author(git: &Path, repo: &Path) -> (String, String) {
    (
        config_value(git, repo, "user.name"),
        config_value(git, repo, "user.email"),
    )
}

fn attribute_uncommitted(lines: Vec<BlameLine>, name: &str, email: &str) -> Vec<BlameLine> {
    lines
        .into_iter()
        .map(|mut line| {
            if !is_uncommitted_line(&line) {
                return line;
            }
            if !name.is_empty() {
                line.author = name.to_string();
            } else if line.author == "Not Committed Yet" || line.author.is_empty() {
                line.author = "You".into();
            }
            if !email.is_empty() {
                line.email = email.to_string();
            }
            line.summary = "Not Committed Yet".into();
            line
        })
        .collect()
}

fn with_current_author(git: &Path, repo: &Path, blame: FileBlame) -> FileBlame {
    let (name, email) = current_author(git, repo);
    FileBlame {
        current: attribute_uncommitted(blame.current, &name, &email),
        previous: attribute_uncommitted(blame.previous, &name, &email),
    }
}

fn parse_blame_porcelain(stdout: &str) -> Vec<BlameLine> {
    let mut lines = Vec::new();
    let mut commits: HashMap<String, BlameMeta> = HashMap::new();
    let mut current_hash = String::new();
    let mut current_final = 0u32;
    let mut pending = BlameMeta::default();

    for raw in stdout.lines() {
        if raw.starts_with('\t') {
            commits
                .entry(current_hash.clone())
                .or_insert_with(|| pending.clone());
            let meta = commits.get(&current_hash).cloned().unwrap_or_default();
            lines.push(normalize_blame_line(BlameLine {
                line: current_final,
                hash: current_hash.clone(),
                author: meta.author,
                email: meta.email,
                timestamp: meta.timestamp,
                summary: meta.summary,
            }));
            continue;
        }

        if raw.len() >= 41
            && raw.as_bytes()[40] == b' '
            && raw.as_bytes()[..40]
                .iter()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            let mut parts = raw.split_whitespace();
            current_hash = parts.next().unwrap_or_default().to_string();
            let _orig = parts.next();
            current_final = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
            pending = commits.get(&current_hash).cloned().unwrap_or_default();
            continue;
        }

        if let Some(value) = raw.strip_prefix("author ") {
            pending.author = value.to_string();
        } else if let Some(value) = raw.strip_prefix("author-mail ") {
            pending.email = value
                .trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string();
        } else if let Some(value) = raw.strip_prefix("author-time ") {
            pending.timestamp = value.parse().unwrap_or(0);
        } else if let Some(value) = raw.strip_prefix("summary ") {
            pending.summary = value.to_string();
        }
    }

    lines
}

fn blame_rev(git: &Path, repo: &Path, rev: &str, file: &str) -> Vec<BlameLine> {
    let output = run_git_quiet(git, repo, &["blame", "--porcelain", rev, "--", file]);
    match output {
        Ok(output) if output.success => parse_blame_porcelain(&output.stdout),
        _ => Vec::new(),
    }
}

fn blame_contents(
    git: &Path,
    repo: &Path,
    file: &str,
    contents: &str,
    rev: Option<&str>,
) -> Vec<BlameLine> {
    let mut args = vec!["blame", "--porcelain", "--contents", "-"];
    if let Some(rev) = rev {
        args.push(rev);
    }
    args.extend(["--", file]);
    match run_git_stdin(git, repo, &args, contents) {
        Ok(output) if output.success => parse_blame_porcelain(&output.stdout),
        _ if rev.is_some() => blame_contents(git, repo, file, contents, None),
        _ => uncommitted_blame(contents),
    }
}

fn blame_worktree(git: &Path, repo: &Path, file: &str) -> Vec<BlameLine> {
    let output = run_git_quiet(git, repo, &["blame", "--porcelain", "--", file]);
    match output {
        Ok(output) if output.success => parse_blame_porcelain(&output.stdout),
        _ => match fs::read_to_string(repo.join(file)) {
            Ok(contents) => uncommitted_blame(&contents),
            Err(_) => Vec::new(),
        },
    }
}

fn blame_index(git: &Path, repo: &Path, file: &str) -> Vec<BlameLine> {
    let spec = format!(":{file}");
    let output = match run_git_quiet(git, repo, &["show", &spec]) {
        Ok(output) if output.success => output,
        _ => return Vec::new(),
    };
    blame_contents(git, repo, file, &output.stdout, Some("HEAD"))
}

fn blame_head(git: &Path, repo: &Path, file: &str) -> Vec<BlameLine> {
    if file_exists_at(git, repo, "HEAD", file) {
        blame_rev(git, repo, "HEAD", file)
    } else {
        Vec::new()
    }
}

fn blame_commit(
    git: &Path,
    repo: &Path,
    file: &str,
    rev: &str,
    old_path: Option<&str>,
) -> Result<FileBlame, String> {
    let hash = require_commit(git, repo, rev)?;
    let current_path = resolve_commit_file_path(git, repo, &hash, file)?;
    let current = blame_rev(git, repo, &hash, &current_path);
    let previous = match first_parent(git, repo, &hash)? {
        Some(parent) => {
            let prev_file = match old_path.filter(|path| !path.is_empty()) {
                Some(old) if file_exists_at(git, repo, &parent, old) => old.to_string(),
                _ => resolve_commit_file_path(git, repo, &parent, file)?,
            };
            blame_rev(git, repo, &parent, &prev_file)
        }
        None => Vec::new(),
    };
    Ok(FileBlame { current, previous })
}

pub fn file_blame(
    git: &Path,
    repo: &Path,
    file: &str,
    rev: Option<&str>,
    staged: bool,
    old_path: Option<&str>,
) -> Result<FileBlame, String> {
    require_file_path(file)?;
    if let Some(old) = old_path.filter(|path| !path.is_empty()) {
        require_file_path(old)?;
    }

    let blame = if let Some(rev) = rev.filter(|rev| !rev.is_empty()) {
        blame_commit(git, repo, file, rev, old_path)?
    } else if staged {
        FileBlame {
            current: blame_index(git, repo, file),
            previous: blame_head(git, repo, file),
        }
    } else {
        FileBlame {
            current: blame_worktree(git, repo, file),
            previous: blame_index(git, repo, file),
        }
    };
    Ok(with_current_author(git, repo, blame))
}

fn ls_files_z(git: &Path, repo: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let output = run_git(git, repo, args)?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not list repository files.",
        ));
    }
    Ok(output
        .stdout
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

pub fn repo_files(git: &Path, repo: &Path) -> Result<Vec<RepoFile>, String> {
    let visible = ls_files_z(
        git,
        repo,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let tracked_ignored = ls_files_z(
        git,
        repo,
        &["ls-files", "--cached", "--ignored", "--exclude-standard", "-z"],
    )?;
    let deleted: HashSet<String> = ls_files_z(git, repo, &["ls-files", "--deleted", "-z"])?
        .into_iter()
        .collect();
    let other_ignored = ls_files_z(
        git,
        repo,
        &[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
            "-z",
        ],
    )?;

    let tracked_ignored: HashSet<String> = tracked_ignored.into_iter().collect();
    let mut by_path = HashMap::new();

    for path in visible {
        if deleted.contains(&path) {
            continue;
        }
        let ignored = tracked_ignored.contains(&path);
        by_path.insert(
            path.clone(),
            RepoFile {
                path,
                ignored,
                directory: false,
            },
        );
    }

    for raw in other_ignored {
        let directory = raw.ends_with('/');
        let path = raw.trim_end_matches('/').to_string();
        if path.is_empty() {
            continue;
        }
        by_path.entry(path.clone()).or_insert(RepoFile {
            path,
            ignored: true,
            directory,
        });
    }

    let mut files: Vec<RepoFile> = by_path.into_values().collect();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub fn working_tree(git: &Path, repo: &Path) -> Result<Vec<WorkingTreeFile>, String> {
    let output = run_git(git, repo, &["status", "--porcelain=v1", "-uall"])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the working tree.",
        ));
    }

    let mut files = Vec::new();
    for line in output.stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let index = line.as_bytes()[0] as char;
        let worktree = line.as_bytes()[1] as char;
        let Some(path) = porcelain_path(&line[3..]) else {
            continue;
        };
        if index == '?' && worktree == '?' {
            files.push(WorkingTreeFile {
                path,
                status: "Untracked".into(),
                untracked: true,
                staged: false,
            });
            continue;
        }
        if unmerged_letters(index, worktree) {
            files.push(WorkingTreeFile {
                path,
                status: "Conflicted".into(),
                untracked: false,
                staged: false,
            });
            continue;
        }
        if index != ' ' && index != '?' {
            files.push(WorkingTreeFile {
                path: path.clone(),
                status: describe_letter(index),
                untracked: false,
                staged: true,
            });
        }
        if worktree != ' ' && worktree != '?' {
            files.push(WorkingTreeFile {
                path,
                status: describe_letter(worktree),
                untracked: false,
                staged: false,
            });
        }
    }

    Ok(files)
}

fn require_file_path(file: &str) -> Result<(), String> {
    if file.is_empty() || file.contains('\0') {
        return Err("Invalid file path".into());
    }
    let path = Path::new(file);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::Prefix(_)
            )
        })
    {
        return Err("Invalid file path".into());
    }
    Ok(())
}

fn repo_file_path(repo: &Path, file: &str) -> Result<PathBuf, String> {
    require_file_path(file)?;
    Ok(repo.join(file))
}

fn is_git_internal(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    normalized == ".git" || normalized.starts_with(".git/")
}

fn file_name(file: &str) -> String {
    Path::new(file)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(file)
        .to_string()
}

fn parent_folder_name(file: &str) -> String {
    Path::new(file)
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
}

fn file_extension(file: &str) -> String {
    let name = file_name(file);
    let rest = name.strip_prefix('.').unwrap_or(&name);
    match rest.rfind('.') {
        Some(index) if index > 0 && index + 1 < rest.len() => {
            format!(".{}", &rest[index + 1..])
        }
        Some(index) if name.starts_with('.') && index + 1 < rest.len() => {
            format!(".{}", &rest[index + 1..])
        }
        _ => String::new(),
    }
}

fn escape_gitignore(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        if matches!(ch, '#' | '!' | '?' | '*' | '[' | '\\' | ' ') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn ignore_pattern(file: &str, kind: &str) -> Result<String, String> {
    match kind {
        "file" => {
            let name = file_name(file);
            if name.is_empty() || name == "." || name == ".." {
                return Err("Invalid file path".into());
            }
            Ok(escape_gitignore(&name))
        }
        "extension" => {
            let ext = file_extension(file);
            if ext.is_empty() {
                return Err("That file has no extension.".into());
            }
            Ok(format!("*{ext}"))
        }
        "folder" => {
            let folder = parent_folder_name(file);
            if folder.is_empty() {
                return Err("That file is not in a folder.".into());
            }
            Ok(format!("{}/", escape_gitignore(&folder)))
        }
        _ => Err("Unknown ignore option.".into()),
    }
}

fn ignore_kind_matches(source: &str, kind: &str, path: &str) -> bool {
    match kind {
        "file" => path == source,
        "extension" => {
            let ext = file_extension(source);
            !ext.is_empty() && file_extension(path) == ext
        }
        "folder" => {
            let folder = parent_folder_name(source);
            !folder.is_empty() && parent_folder_name(path) == folder
        }
        _ => false,
    }
}

fn append_gitignore(repo: &Path, pattern: &str) -> Result<(), String> {
    let path = repo.join(".gitignore");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if existing.lines().any(|line| line.trim() == pattern) {
        return Ok(());
    }
    let mut contents = existing;
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(pattern);
    contents.push('\n');
    fs::write(&path, contents).map_err(|err| format!("Could not update .gitignore: {err}"))?;
    Ok(())
}

fn is_tracked(git: &Path, repo: &Path, file: &str) -> bool {
    run_git_quiet(git, repo, &["ls-files", "--error-unmatch", "--", file])
        .map(|output| output.success)
        .unwrap_or(false)
}

fn committed_in_head(git: &Path, repo: &Path, file: &str) -> bool {
    let spec = format!("HEAD:{file}");
    run_git_quiet(git, repo, &["cat-file", "-e", &spec])
        .map(|output| output.success)
        .unwrap_or(false)
}

fn untrack_file(git: &Path, repo: &Path, file: &str) -> Result<(), String> {
    if !is_tracked(git, repo, file) {
        return Ok(());
    }
    let output = run_git(git, repo, &["rm", "--cached", "-q", "--", file])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to stop tracking the file.",
        ));
    }
    Ok(())
}

pub fn ignore_working_tree_path(
    git: &Path,
    repo: &Path,
    file: &str,
    kind: &str,
) -> Result<(), String> {
    require_file_path(file)?;
    if is_git_internal(file) {
        return Err("Cannot ignore git internals.".into());
    }
    let pattern = ignore_pattern(file, kind)?;
    append_gitignore(repo, &pattern)?;
    let files = working_tree(git, repo)?;
    for entry in files {
        if entry.untracked || !ignore_kind_matches(file, kind, &entry.path) {
            continue;
        }
        untrack_file(git, repo, &entry.path)?;
    }
    Ok(())
}

pub fn stash_file(git: &Path, repo: &Path, file: &str) -> Result<String, String> {
    require_file_path(file)?;
    let files = working_tree(git, repo)?;
    if !files.iter().any(|entry| entry.path == file) {
        return Err("Nothing to stash for that file.".into());
    }
    let output = run_git(
        git,
        repo,
        &["stash", "push", "--include-untracked", "--", file],
    )?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to stash the file.",
        ));
    }
    let combined = combined_message(&output);
    if combined.to_ascii_lowercase().contains("no local changes") {
        return Err("Nothing to stash for that file.".into());
    }
    Ok(or_fallback(&combined, "Stashed file"))
}

pub fn delete_working_tree_file(git: &Path, repo: &Path, file: &str) -> Result<(), String> {
    let path = repo_file_path(repo, file)?;
    if is_git_internal(file) {
        return Err("Cannot delete git internals.".into());
    }
    if path.is_dir() {
        return Err("That path is a folder.".into());
    }
    let tracked = is_tracked(git, repo, file);
    if !path.exists() {
        if tracked && !committed_in_head(git, repo, file) {
            let output = run_git(git, repo, &["rm", "--cached", "-f", "--", file])?;
            if !output.success {
                return Err(or_fallback(
                    &combined_message(&output),
                    "Failed to delete the file.",
                ));
            }
            return Ok(());
        }
        return Err("That file is not on disk.".into());
    }
    if tracked && !committed_in_head(git, repo, file) {
        let output = run_git(git, repo, &["rm", "-f", "--", file])?;
        if !output.success {
            return Err(or_fallback(
                &combined_message(&output),
                "Failed to delete the file.",
            ));
        }
        return Ok(());
    }
    fs::remove_file(&path).map_err(|err| format!("Could not delete the file: {err}"))?;
    Ok(())
}

pub fn stage_file(git: &Path, repo: &Path, file: &str) -> Result<(), String> {
    require_file_path(file)?;
    let output = run_git(git, repo, &["add", "--", file])?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), "Failed to stage file."));
    }
    Ok(())
}

pub fn stage_all(git: &Path, repo: &Path) -> Result<(), String> {
    let output = run_git(git, repo, &["add", "-A"])?;
    if !output.success {
        return Err(or_fallback(&combined_message(&output), "Failed to stage all files."));
    }
    Ok(())
}

pub fn unstage_file(git: &Path, repo: &Path, file: &str) -> Result<(), String> {
    require_file_path(file)?;
    let output = run_git(git, repo, &["restore", "--staged", "--", file])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to unstage file.",
        ));
    }
    Ok(())
}

pub fn unstage_all(git: &Path, repo: &Path) -> Result<(), String> {
    let output = run_git(git, repo, &["restore", "--staged", "."])?;
    if !output.success {
        return Err(or_fallback(
            &combined_message(&output),
            "Failed to unstage all files.",
        ));
    }
    Ok(())
}

pub fn file_diff(git: &Path, repo: &Path, file: &str, staged: bool) -> Result<String, String> {
    require_file_path(file)?;

    if !staged {
        let status = run_git(git, repo, &["status", "--porcelain=v1", "--", file])?;
        let untracked = status.stdout.lines().any(|line| line.starts_with("??"));
        if untracked {
            return untracked_diff(repo, file);
        }
        let conflicted = status.stdout.lines().any(|line| {
            line.len() >= 2
                && unmerged_letters(line.as_bytes()[0] as char, line.as_bytes()[1] as char)
        });
        if conflicted {
            let combined = run_git(git, repo, &["diff", "--cc", "--", file])?;
            if combined.success && !combined.stdout.trim().is_empty() {
                return Ok(combined.stdout);
            }
        }
    }

    let output = if staged {
        run_git(git, repo, &["diff", "--cached", "--", file])?
    } else {
        run_git(git, repo, &["diff", "--", file])?
    };
    if !output.success && output.stdout.trim().is_empty() {
        return Err(or_fallback(
            &combined_message(&output),
            "Could not read the file diff.",
        ));
    }

    if output.stdout.trim().is_empty() {
        return Ok(if staged {
            "No staged changes.".into()
        } else {
            "No unstaged changes.".into()
        });
    }

    Ok(output.stdout)
}

fn untracked_diff(repo: &Path, file: &str) -> Result<String, String> {
    let absolute = repo.join(file);
    let contents = std::fs::read_to_string(&absolute).map_err(|err| {
        format!("Could not read untracked file {file}: {err}")
    })?;
    let lines: Vec<&str> = contents.lines().collect();
    let count = lines.len().max(1);
    let mut diff = format!(
        "diff --git a/{file} b/{file}\nnew file mode 100644\n--- /dev/null\n+++ b/{file}\n@@ -0,0 +1,{count} @@\n"
    );
    if contents.is_empty() {
        diff.push_str("+\n");
    } else {
        for line in lines {
            diff.push('+');
            diff.push_str(line);
            diff.push('\n');
        }
    }
    Ok(diff)
}

fn or_fallback(message: &str, fallback: &str) -> String {
    if message.trim().is_empty() {
        fallback.to_string()
    } else {
        message.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn git_bin() -> PathBuf {
        resolve_git_binary().expect("git should be installed for tests")
    }

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "shipyard-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn git(repo: &Path, args: &[&str]) -> GitOutput {
        let output = run_git(&git_bin(), repo, args).unwrap();
        assert!(
            output.success,
            "git {} failed: {}",
            args.join(" "),
            combined_message(&output)
        );
        output
    }

    fn init_repo() -> PathBuf {
        let repo = temp_dir();
        git(&repo, &["init", "-b", "develop"]);
        git(&repo, &["config", "user.name", "Shipyard Test"]);
        git(&repo, &["config", "user.email", "test@shipyard.local"]);
        fs::write(repo.join("README.md"), "hello\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "initial"]);
        repo
    }

    #[test]
    fn reads_branch_and_dirty_status() {
        let repo = init_repo();
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "develop");
        assert!(!live_status(&git_bin(), &repo).unwrap().dirty);
        fs::write(repo.join("README.md"), "changed\n").unwrap();
        let dirty = live_status(&git_bin(), &repo).unwrap();
        assert!(dirty.dirty);
        assert_eq!((dirty.insertions, dirty.deletions, dirty.changed_files), (1, 1, 1));
        fs::write(repo.join("new.txt"), "one\ntwo\n").unwrap();
        let with_untracked = live_status(&git_bin(), &repo).unwrap();
        assert_eq!(
            (
                with_untracked.insertions,
                with_untracked.deletions,
                with_untracked.changed_files
            ),
            (3, 1, 2)
        );
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "README.md" && !file.staged));
        assert!(!files.iter().any(|file| file.staged));
    }

    #[test]
    fn clones_into_named_folder() {
        let source = init_repo();
        let parent = temp_dir();
        let url = source.to_str().unwrap();
        let dest = clone_repo(&git_bin(), url, &parent, "copy").unwrap();
        assert_eq!(dest, parent.join("copy"));
        assert_eq!(fs::read_to_string(dest.join("README.md")).unwrap(), "hello\n");
        assert_eq!(current_branch(&git_bin(), &dest).unwrap(), "develop");

        let err = clone_repo(&git_bin(), url, &parent, "copy").unwrap_err();
        assert!(err.contains("already exists"), "{err}");

        fs::create_dir(parent.join("empty")).unwrap();
        assert!(clone_repo(&git_bin(), url, &parent, "empty").is_ok());
    }

    #[test]
    fn clone_rejects_bad_input() {
        let parent = temp_dir();
        let git = git_bin();
        assert!(clone_repo(&git, "  ", &parent, "repo").is_err());
        assert!(clone_repo(&git, "--upload-pack=touch /tmp/x", &parent, "repo").is_err());
        for name in ["", ".", "..", "a/b", "a\\b"] {
            assert!(clone_repo(&git, "https://example.com/r.git", &parent, name).is_err());
        }
        assert!(clone_repo(&git, "https://example.com/r.git", &parent.join("missing"), "repo").is_err());
        let err = clone_repo(&git, parent.join("nope").to_str().unwrap(), &parent, "repo").unwrap_err();
        assert!(!err.is_empty());
        assert!(!parent.join("repo").exists());
    }

    #[test]
    fn lists_tracked_and_untracked_repo_files() {
        let repo = init_repo();
        fs::create_dir_all(repo.join("src")).unwrap();
        fs::write(repo.join("src/app.ts"), "export {}\n").unwrap();
        git(&repo, &["add", "src/app.ts"]);
        git(&repo, &["commit", "-m", "add app"]);
        fs::write(repo.join("src/new.ts"), "export {}\n").unwrap();
        fs::write(repo.join("tracked.ignore"), "tracked then ignored\n").unwrap();
        git(&repo, &["add", "tracked.ignore"]);
        git(&repo, &["commit", "-m", "track ignored later"]);
        fs::write(repo.join("ignored.txt"), "nope\n").unwrap();
        fs::create_dir_all(repo.join("build")).unwrap();
        fs::write(repo.join("build/out.js"), "ignored dir\n").unwrap();
        fs::write(repo.join(".gitignore"), "ignored.txt\nbuild/\ntracked.ignore\n").unwrap();
        let files = repo_files(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "README.md" && !file.ignored));
        assert!(files.iter().any(|file| file.path == "src/app.ts" && !file.ignored));
        assert!(files.iter().any(|file| file.path == "src/new.ts" && !file.ignored));
        assert!(files.iter().any(|file| file.path == ".gitignore" && !file.ignored));
        assert!(files.iter().any(|file| file.path == "ignored.txt" && file.ignored && !file.directory));
        assert!(files.iter().any(|file| file.path == "build" && file.ignored && file.directory));
        assert!(files.iter().any(|file| file.path == "tracked.ignore" && file.ignored));
        assert!(!files.iter().any(|file| file.path == "build/out.js"));
    }

    #[test]
    fn repo_files_drops_renamed_tracked_paths() {
        let repo = init_repo();
        fs::write(repo.join("icon.jpg"), "old\n").unwrap();
        git(&repo, &["add", "icon.jpg"]);
        git(&repo, &["commit", "-m", "add icon"]);
        fs::rename(repo.join("icon.jpg"), repo.join("icon.jpg123")).unwrap();
        let files = repo_files(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "icon.jpg123" && !file.ignored));
        assert!(!files.iter().any(|file| file.path == "icon.jpg"));
    }

    #[test]
    fn lists_commits_for_a_single_file() {
        let repo = init_repo();
        fs::write(repo.join("README.md"), "hello\nagain\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "update readme"]);
        let commits = file_log(&git_bin(), &repo, "README.md").unwrap();
        assert!(commits.iter().any(|commit| commit.subject == "initial"));
        assert!(commits.iter().any(|commit| commit.subject == "update readme"));
        assert!(commits.iter().all(|commit| commit.path.as_deref() == Some("README.md")));
        assert!(file_log(&git_bin(), &repo, "missing.txt").unwrap().is_empty());
    }

    #[test]
    fn file_history_follows_renames_and_shows_old_diffs() {
        let repo = init_repo();
        fs::create_dir_all(repo.join("old")).unwrap();
        fs::write(repo.join("old/.oxfmtrc.json"), "one\n").unwrap();
        git(&repo, &["add", "old/.oxfmtrc.json"]);
        git(&repo, &["commit", "-m", "add oxfmt"]);
        fs::write(repo.join("old/.oxfmtrc.json"), "two\n").unwrap();
        git(&repo, &["add", "old/.oxfmtrc.json"]);
        git(&repo, &["commit", "-m", "update oxfmt"]);
        git(&repo, &["mv", "old/.oxfmtrc.json", ".oxfmtrc.json"]);
        git(&repo, &["commit", "-m", "move oxfmt"]);
        fs::copy(repo.join(".oxfmtrc.json"), repo.join("extra.oxfmtrc.json")).unwrap();
        git(&repo, &["add", "extra.oxfmtrc.json"]);
        git(&repo, &["commit", "-m", "copy oxfmt"]);

        let copied = file_log(&git_bin(), &repo, "extra.oxfmtrc.json")
            .unwrap()
            .into_iter()
            .find(|commit| commit.subject == "copy oxfmt")
            .expect("copy commit");
        assert_eq!(copied.path.as_deref(), Some("extra.oxfmtrc.json"));
        assert_eq!(copied.old_path.as_deref(), Some(".oxfmtrc.json"));
        assert_eq!(copied.status.as_deref(), Some("Copied"));

        let commits = file_log(&git_bin(), &repo, ".oxfmtrc.json").unwrap();
        let added = commits
            .iter()
            .find(|commit| commit.subject == "add oxfmt")
            .expect("add commit");
        let updated = commits
            .iter()
            .find(|commit| commit.subject == "update oxfmt")
            .expect("update commit");
        let moved = commits
            .iter()
            .find(|commit| commit.subject == "move oxfmt")
            .expect("move commit");
        assert_eq!(added.path.as_deref(), Some("old/.oxfmtrc.json"));
        assert_eq!(added.status.as_deref(), Some("Added"));
        assert_eq!(updated.path.as_deref(), Some("old/.oxfmtrc.json"));
        assert_eq!(updated.status.as_deref(), Some("Modified"));
        assert_eq!(moved.path.as_deref(), Some(".oxfmtrc.json"));
        assert_eq!(moved.old_path.as_deref(), Some("old/.oxfmtrc.json"));
        assert_eq!(moved.status.as_deref(), Some("Renamed"));

        let added_diff = commit_file_diff(&git_bin(), &repo, &added.hash, ".oxfmtrc.json").unwrap();
        assert!(
            added_diff.contains("+one"),
            "expected add diff, got {added_diff}"
        );
        let updated_diff =
            commit_file_diff(&git_bin(), &repo, &updated.hash, ".oxfmtrc.json").unwrap();
        assert!(
            updated_diff.contains("-one") && updated_diff.contains("+two"),
            "expected update diff, got {updated_diff}"
        );
        assert_ne!(added_diff.trim(), "No changes.");
        assert_ne!(updated_diff.trim(), "No changes.");
    }

    #[test]
    fn parse_blame_porcelain_reuses_commit_metadata() {
        let stdout = "\
c0ffeec0ffeec0ffeec0ffeec0ffeec0ffeec0ff 1 1 2
author Shipyard Test
author-mail <test@shipyard.local>
author-time 1710000000
author-tz +0000
summary initial
filename README.md
\thello
c0ffeec0ffeec0ffeec0ffeec0ffeec0ffeec0ff 2 2
\tworld
0000000000000000000000000000000000000000 3 3 1
author Not Committed Yet
author-mail <not.committed@yet>
author-time 0
author-tz +0000
summary Uncommitted changes
filename README.md
\tdraft
";
        let lines = parse_blame_porcelain(stdout);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line, 1);
        assert_eq!(lines[0].author, "Shipyard Test");
        assert_eq!(lines[0].email, "test@shipyard.local");
        assert_eq!(lines[0].timestamp, 1_710_000_000);
        assert_eq!(lines[0].summary, "initial");
        assert_eq!(lines[1].line, 2);
        assert_eq!(lines[1].author, "Shipyard Test");
        assert_eq!(lines[1].summary, "initial");
        assert_eq!(lines[2].line, 3);
        assert_eq!(lines[2].author, "Not Committed Yet");
        assert!(lines[2].hash.chars().all(|c| c == '0'));
    }

    #[test]
    fn blames_working_tree_and_commit_history() {
        let repo = init_repo();
        fs::write(repo.join("README.md"), "hello\nagain\n").unwrap();
        let worktree = file_blame(&git_bin(), &repo, "README.md", None, false, None).unwrap();
        assert_eq!(worktree.current.len(), 2);
        assert_eq!(worktree.current[0].author, "Shipyard Test");
        assert_eq!(worktree.current[0].summary, "initial");
        assert_eq!(worktree.current[1].author, "Shipyard Test");
        assert_eq!(worktree.current[1].email, "test@shipyard.local");
        assert_eq!(worktree.current[1].summary, "Not Committed Yet");
        assert_eq!(worktree.previous.len(), 1);
        assert_eq!(worktree.previous[0].summary, "initial");

        git(&repo, &["add", "README.md"]);
        let staged = file_blame(&git_bin(), &repo, "README.md", None, true, None).unwrap();
        assert_eq!(staged.current.len(), 2);
        assert_eq!(staged.current[1].author, "Shipyard Test");
        assert_eq!(staged.current[1].summary, "Not Committed Yet");
        assert_eq!(staged.previous.len(), 1);
        assert_eq!(staged.previous[0].summary, "initial");

        git(&repo, &["commit", "-m", "update readme"]);
        let commits = file_log(&git_bin(), &repo, "README.md").unwrap();
        let first = commits
            .iter()
            .find(|commit| commit.subject == "initial")
            .expect("initial commit");
        let historical = file_blame(
            &git_bin(),
            &repo,
            "README.md",
            Some(&first.hash),
            false,
            None,
        )
        .unwrap();
        assert_eq!(historical.current.len(), 1);
        assert_eq!(historical.current[0].summary, "initial");
        assert!(historical.previous.is_empty());
    }

    #[test]
    fn checkout_skips_when_already_on_target() {
        let repo = init_repo();
        let message = checkout_with_fallbacks(&git_bin(), &repo, &["develop".into()]).unwrap();
        assert!(message.contains("Already on develop"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "develop");
    }

    #[test]
    fn checkout_skips_fallback_when_already_on_it() {
        let repo = init_repo();
        let message = checkout_with_fallbacks(
            &git_bin(),
            &repo,
            &["missing-feature".into(), "develop".into()],
        )
        .unwrap();
        assert!(message.contains("Already on develop"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "develop");
    }

    #[test]
    fn checkout_falls_back_when_target_is_missing() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "MERP-234"]);
        git(&repo, &["checkout", "develop"]);
        let message = checkout_with_fallbacks(
            &git_bin(),
            &repo,
            &["MERP-123".into(), "MERP-234".into(), "develop".into()],
        )
        .unwrap();
        assert!(message.contains("MERP-234"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "MERP-234");
    }

    #[test]
    fn diff_includes_untracked_files() {
        let repo = init_repo();
        fs::write(repo.join("new.txt"), "fresh\n").unwrap();
        let diff = file_diff(&git_bin(), &repo, "new.txt", false).unwrap();
        assert!(diff.contains("+fresh"));
        let commits = log_graph(&git_bin(), &repo).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].subject, "initial");
    }

    #[test]
    fn ignore_helpers_read_name_extension_and_folder() {
        assert_eq!(file_name("scripts/find-email-inserts.sh"), "find-email-inserts.sh");
        assert_eq!(file_extension("scripts/find-email-inserts.sh"), ".sh");
        assert_eq!(parent_folder_name("scripts/find-email-inserts.sh"), "scripts");
        assert_eq!(file_extension(".env"), "");
        assert_eq!(file_extension(".env.local"), ".local");
        assert_eq!(file_extension("Makefile"), "");
        assert_eq!(parent_folder_name("README.md"), "");
        assert_eq!(ignore_pattern("src/app.ts", "file").unwrap(), "app.ts");
        assert_eq!(ignore_pattern("src/app.ts", "extension").unwrap(), "*.ts");
        assert_eq!(ignore_pattern("src/app.ts", "folder").unwrap(), "src/");
        assert!(ignore_pattern("README.md", "folder").is_err());
        assert!(ignore_pattern("Makefile", "extension").is_err());
        assert!(require_file_path("../secret").is_err());
        assert!(require_file_path("/etc/passwd").is_err());
    }

    #[test]
    fn ignores_working_tree_files_by_name_extension_and_folder() {
        let repo = init_repo();
        fs::create_dir_all(repo.join("scripts")).unwrap();
        fs::write(repo.join("scripts/one.sh"), "echo one\n").unwrap();
        fs::write(repo.join("scripts/two.sh"), "echo two\n").unwrap();
        fs::write(repo.join("notes.md"), "keep\n").unwrap();
        fs::create_dir_all(repo.join("tmp")).unwrap();
        fs::write(repo.join("tmp/scratch.txt"), "scratch\n").unwrap();
        ignore_working_tree_path(&git_bin(), &repo, "tmp/scratch.txt", "folder").unwrap();
        let after_folder = working_tree(&git_bin(), &repo).unwrap();
        assert!(!after_folder.iter().any(|file| file.path.starts_with("tmp/")));
        assert!(fs::read_to_string(repo.join(".gitignore"))
            .unwrap()
            .contains("tmp/"));
        fs::write(repo.join("tracked.log"), "old\n").unwrap();
        git(&repo, &["add", "tracked.log"]);
        git(&repo, &["commit", "-m", "track log"]);
        fs::write(repo.join("tracked.log"), "dirty\n").unwrap();

        ignore_working_tree_path(&git_bin(), &repo, "scripts/one.sh", "file").unwrap();
        let after_file = working_tree(&git_bin(), &repo).unwrap();
        assert!(!after_file.iter().any(|file| file.path == "scripts/one.sh"));
        assert!(after_file.iter().any(|file| file.path == "scripts/two.sh"));
        assert!(fs::read_to_string(repo.join(".gitignore"))
            .unwrap()
            .contains("one.sh"));

        ignore_working_tree_path(&git_bin(), &repo, "scripts/two.sh", "extension").unwrap();
        let after_ext = working_tree(&git_bin(), &repo).unwrap();
        assert!(!after_ext.iter().any(|file| file.path.ends_with(".sh")));
        assert!(fs::read_to_string(repo.join(".gitignore"))
            .unwrap()
            .contains("*.sh"));

        ignore_working_tree_path(&git_bin(), &repo, "tracked.log", "file").unwrap();
        let after_tracked = working_tree(&git_bin(), &repo).unwrap();
        assert!(after_tracked
            .iter()
            .any(|file| file.path == "tracked.log" && file.staged && file.status == "Deleted"));
        assert!(!after_tracked
            .iter()
            .any(|file| file.path == "tracked.log" && !file.staged));
        assert!(repo.join("tracked.log").exists());
        assert!(after_tracked.iter().any(|file| file.path == "notes.md"));
    }

    #[test]
    fn stashes_a_single_working_tree_file() {
        let repo = init_repo();
        fs::write(repo.join("README.md"), "changed\n").unwrap();
        fs::write(repo.join("notes.txt"), "untracked\n").unwrap();

        stash_file(&git_bin(), &repo, "notes.txt").unwrap();
        assert!(!repo.join("notes.txt").exists());
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "README.md"));
        assert!(!files.iter().any(|file| file.path == "notes.txt"));
        assert_eq!(stash_list(&git_bin(), &repo).unwrap().len(), 1);

        stash_file(&git_bin(), &repo, "README.md").unwrap();
        assert!(working_tree(&git_bin(), &repo).unwrap().is_empty());
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "hello\n");
        assert_eq!(stash_list(&git_bin(), &repo).unwrap().len(), 2);
        assert!(stash_file(&git_bin(), &repo, "README.md").is_err());
    }

    #[test]
    fn deletes_untracked_and_tracked_working_tree_files() {
        let repo = init_repo();
        fs::write(repo.join("notes.txt"), "untracked\n").unwrap();
        fs::write(repo.join("README.md"), "changed\n").unwrap();
        fs::write(repo.join("fresh.txt"), "staged new\n").unwrap();
        git(&repo, &["add", "fresh.txt"]);

        delete_working_tree_file(&git_bin(), &repo, "notes.txt").unwrap();
        assert!(!repo.join("notes.txt").exists());

        delete_working_tree_file(&git_bin(), &repo, "README.md").unwrap();
        assert!(!repo.join("README.md").exists());
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files
            .iter()
            .any(|file| file.path == "README.md" && !file.staged && file.status == "Deleted"));

        delete_working_tree_file(&git_bin(), &repo, "fresh.txt").unwrap();
        assert!(!repo.join("fresh.txt").exists());
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(!files.iter().any(|file| file.path == "fresh.txt"));
        assert!(delete_working_tree_file(&git_bin(), &repo, "../secret").is_err());
        assert!(reveal_file_in_finder(&repo, "missing.txt").is_err());
    }

    #[test]
    fn stages_and_unstages_working_tree_files() {
        let repo = init_repo();
        fs::write(repo.join("new.txt"), "fresh\n").unwrap();
        fs::write(repo.join("README.md"), "changed\n").unwrap();

        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "new.txt" && !file.staged && file.untracked));
        assert!(files.iter().any(|file| file.path == "README.md" && !file.staged));
        assert!(!files.iter().any(|file| file.staged));

        stage_file(&git_bin(), &repo, "new.txt").unwrap();
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "new.txt" && file.staged));
        assert!(!files.iter().any(|file| file.path == "new.txt" && !file.staged));
        assert!(files.iter().any(|file| file.path == "README.md" && !file.staged));

        stage_all(&git_bin(), &repo).unwrap();
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().all(|file| file.staged));
        assert!(files.iter().any(|file| file.path == "README.md" && file.staged));

        let staged_diff = file_diff(&git_bin(), &repo, "README.md", true).unwrap();
        assert!(staged_diff.contains("-hello") || staged_diff.contains("+changed"));

        unstage_file(&git_bin(), &repo, "new.txt").unwrap();
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "new.txt" && !file.staged));
        assert!(!files.iter().any(|file| file.path == "new.txt" && file.staged));

        unstage_all(&git_bin(), &repo).unwrap();
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().all(|file| !file.staged));
    }

    #[test]
    fn commits_staged_files_with_title_and_description() {
        let repo = init_repo();
        fs::write(repo.join("README.md"), "updated\n").unwrap();
        stage_all(&git_bin(), &repo).unwrap();
        let message = commit(
            &git_bin(),
            &repo,
            "Update readme",
            "Describe the change.",
            false,
        )
        .unwrap();
        assert!(message.to_lowercase().contains("update readme") || message.contains("develop"));
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.is_empty());
        let commits = log_graph(&git_bin(), &repo).unwrap();
        assert_eq!(commits[0].subject, "Update readme");
        assert!(commit(&git_bin(), &repo, "Nothing", "", false).is_err());
    }

    #[test]
    fn reads_last_commit_and_amends_staged_files() {
        let repo = init_repo();
        let last = last_commit(&git_bin(), &repo).unwrap();
        assert_eq!(last.title, "initial");
        assert!(last.description.is_empty());
        assert!(!last.published);

        fs::write(repo.join("README.md"), "updated\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "Update readme", "-m", "More detail."]);
        let last = last_commit(&git_bin(), &repo).unwrap();
        assert_eq!(last.title, "Update readme");
        assert_eq!(last.description, "More detail.");

        fs::write(repo.join("extra.txt"), "forgot\n").unwrap();
        stage_file(&git_bin(), &repo, "extra.txt").unwrap();
        let message = commit(&git_bin(), &repo, "Update readme", "More detail.", true).unwrap();
        assert!(message.to_lowercase().contains("update readme") || message.contains("develop"));
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.is_empty());
        let commits = log_graph(&git_bin(), &repo).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "Update readme");
        assert!(repo.join("extra.txt").exists());
        assert!(commit(&git_bin(), &repo, "Nothing", "", true).is_err());
    }

    #[test]
    fn last_commit_is_published_when_already_pushed() {
        let upstream = init_repo();
        let work = temp_dir();
        git(&work, &["clone", upstream.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);
        let last = last_commit(&git_bin(), &work).unwrap();
        assert_eq!(last.title, "initial");
        assert!(last.published);

        fs::write(work.join("local.txt"), "mine\n").unwrap();
        git(&work, &["add", "local.txt"]);
        git(&work, &["commit", "-m", "local commit"]);
        let last = last_commit(&git_bin(), &work).unwrap();
        assert_eq!(last.title, "local commit");
        assert!(!last.published);
    }

    #[test]
    fn soft_resets_unpushed_commits_to_upstream() {
        let upstream = init_repo();
        let work = temp_dir();
        git(&work, &["clone", upstream.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);

        fs::write(work.join("one.txt"), "first\n").unwrap();
        git(&work, &["add", "one.txt"]);
        git(&work, &["commit", "-m", "first local"]);
        fs::write(work.join("two.txt"), "second\n").unwrap();
        git(&work, &["add", "two.txt"]);
        git(&work, &["commit", "-m", "second local"]);
        fs::write(work.join("README.md"), "dirty\n").unwrap();

        let ahead = live_status(&git_bin(), &work).unwrap();
        assert_eq!(ahead.ahead, 2);

        let message = reset_unpushed_commits(&git_bin(), &work).unwrap();
        assert!(message.contains("2 unpushed commits"));

        let status = live_status(&git_bin(), &work).unwrap();
        assert_eq!(status.ahead, 0);
        assert_eq!(current_branch(&git_bin(), &work).unwrap(), "develop");
        assert_eq!(last_commit(&git_bin(), &work).unwrap().title, "initial");

        let files = working_tree(&git_bin(), &work).unwrap();
        assert!(files.iter().any(|file| file.path == "one.txt" && file.staged));
        assert!(files.iter().any(|file| file.path == "two.txt" && file.staged));
        assert!(files.iter().any(|file| file.path == "README.md" && !file.staged));
        assert_eq!(fs::read_to_string(work.join("one.txt")).unwrap(), "first\n");
        assert_eq!(fs::read_to_string(work.join("README.md")).unwrap(), "dirty\n");

        assert!(reset_unpushed_commits(&git_bin(), &work).is_err());
        assert!(reset_unpushed_commits(&git_bin(), &upstream).is_err());
    }

    #[test]
    fn refuses_to_reset_unpushed_commits_during_merge() {
        let repo = conflicted_merge_repo();
        assert!(reset_unpushed_commits(&git_bin(), &repo).is_err());
    }

    #[test]
    fn lists_and_checks_out_local_branches() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        let branches = local_branches(&git_bin(), &repo).unwrap();
        assert!(branches.contains(&"develop".into()));
        assert!(branches.contains(&"feature".into()));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "feature");

        let message = checkout_local_branch(&git_bin(), &repo, "develop").unwrap();
        assert!(message.contains("develop"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "develop");
        assert!(checkout_local_branch(&git_bin(), &repo, "missing").is_err());

        let created = create_and_checkout_branch(&git_bin(), &repo, "task/123", "").unwrap();
        assert!(created.contains("task/123"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "task/123");
        assert!(local_branches(&git_bin(), &repo)
            .unwrap()
            .contains(&"task/123".into()));
        assert!(create_and_checkout_branch(&git_bin(), &repo, "task/123", "").is_err());

        let renamed = rename_local_branch(&git_bin(), &repo, "task/123", "task/456").unwrap();
        assert!(renamed.contains("task/456"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "task/456");
        let after_rename = local_branches(&git_bin(), &repo).unwrap();
        assert!(after_rename.contains(&"task/456".into()));
        assert!(!after_rename.contains(&"task/123".into()));
        assert!(rename_local_branch(&git_bin(), &repo, "missing", "other").is_err());
        assert!(rename_local_branch(&git_bin(), &repo, "develop", "task/456").is_err());
    }

    #[test]
    fn creates_branch_from_selected_base() {
        let repo = init_repo();
        let base_hash = git(&repo, &["rev-parse", "HEAD"]).stdout.trim().to_string();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);

        let created =
            create_and_checkout_branch(&git_bin(), &repo, "task/from-develop", "develop").unwrap();
        assert!(created.contains("task/from-develop"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "task/from-develop");
        assert_eq!(git(&repo, &["rev-parse", "HEAD"]).stdout.trim(), base_hash);
        assert!(create_and_checkout_branch(&git_bin(), &repo, "task/missing-base", "nope").is_err());

        let empty = temp_dir();
        git(&empty, &["init", "-b", "develop"]);
        let from_current =
            create_and_checkout_branch(&git_bin(), &empty, "feature", "develop").unwrap();
        assert!(from_current.contains("feature"));
        assert_eq!(
            git(&empty, &["symbolic-ref", "--short", "HEAD"]).stdout.trim(),
            "feature"
        );
    }

    #[test]
    fn merges_local_branch_into_another() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);

        let message = merge_local_branch(&git_bin(), &repo, "feature", "develop").unwrap();
        assert!(
            message.to_lowercase().contains("merge")
                || message.to_lowercase().contains("fast-forward")
                || message.contains("feature")
        );
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "develop");
        assert_eq!(
            fs::read_to_string(repo.join("feature.txt")).unwrap(),
            "work\n"
        );

        let again = merge_local_branch(&git_bin(), &repo, "feature", "develop").unwrap();
        assert!(again.to_lowercase().contains("already"));

        git(&repo, &["checkout", "-b", "other"]);
        fs::write(repo.join("other.txt"), "other\n").unwrap();
        git(&repo, &["add", "other.txt"]);
        git(&repo, &["commit", "-m", "other work"]);
        git(&repo, &["checkout", "develop"]);
        let onto_current = merge_local_branch(&git_bin(), &repo, "other", "develop").unwrap();
        assert!(
            onto_current.to_lowercase().contains("merge")
                || onto_current.to_lowercase().contains("fast-forward")
                || onto_current.contains("other")
        );
        assert_eq!(fs::read_to_string(repo.join("other.txt")).unwrap(), "other\n");

        assert!(merge_local_branch(&git_bin(), &repo, "develop", "develop").is_err());
        assert!(merge_local_branch(&git_bin(), &repo, "missing", "develop").is_err());
        assert!(merge_local_branch(&git_bin(), &repo, "develop", "missing").is_err());
    }

    #[test]
    fn merge_local_branch_leaves_conflicts_in_progress() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("README.md"), "feature\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "feature change"]);
        git(&repo, &["checkout", "develop"]);
        fs::write(repo.join("README.md"), "develop\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "develop change"]);
        git(&repo, &["checkout", "feature"]);

        let err = merge_local_branch(&git_bin(), &repo, "feature", "develop").unwrap_err();
        assert!(err.to_lowercase().contains("conflict") || err.to_lowercase().contains("merge"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "develop");
        assert_eq!(current_operation(&repo), Some("merge"));
        assert!(merge_local_branch(&git_bin(), &repo, "feature", "develop").is_err());
    }

    #[test]
    fn merge_local_branch_stays_put_when_checkout_is_blocked() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);
        git(&repo, &["checkout", "develop"]);
        fs::write(repo.join("README.md"), "develop\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "develop edit"]);
        git(&repo, &["checkout", "feature"]);
        fs::write(repo.join("README.md"), "uncommitted\n").unwrap();

        assert!(merge_local_branch(&git_bin(), &repo, "feature", "develop").is_err());
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "feature");
        assert!(current_operation(&repo).is_none());
        assert_eq!(
            fs::read_to_string(repo.join("README.md")).unwrap(),
            "uncommitted\n"
        );
    }

    #[test]
    fn marks_and_deletes_merged_local_branches() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["merge", "--no-ff", "-m", "merge feature", "feature"]);

        let overview = branch_overview_with(&git_bin(), &repo, Some("develop"), true).unwrap();
        assert_eq!(overview.merge_target.as_deref(), Some("develop"));
        let feature = overview
            .branches
            .iter()
            .find(|branch| branch.name == "feature")
            .unwrap();
        assert!(feature.merged);
        assert!(!feature.partial);
        assert!(!feature.pending);
        assert!(!feature.current);
        let develop = overview
            .branches
            .iter()
            .find(|branch| branch.name == "develop")
            .unwrap();
        assert!(develop.protected_branch);
        assert!(develop.current);

        assert!(delete_local_branch(&git_bin(), &repo, "develop", false).is_err());
        let deleted =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), false, None).unwrap();
        assert_eq!(deleted.deleted, vec!["feature".to_string()]);
        assert!(deleted.refused.is_empty());
        git(&repo, &["checkout", "-b", "wip"]);
        fs::write(repo.join("wip.txt"), "unmerged\n").unwrap();
        git(&repo, &["add", "wip.txt"]);
        git(&repo, &["commit", "-m", "unmerged work"]);
        git(&repo, &["checkout", "develop"]);

        let leftover =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), false, None).unwrap();
        assert!(leftover.deleted.is_empty());
        assert!(leftover.refused.is_empty());
        assert!(leftover.message.contains("No merged"));
        let names = local_branches(&git_bin(), &repo).unwrap();
        assert!(names.contains(&"wip".into()));
        assert!(names.contains(&"develop".into()));
        assert!(!names.contains(&"feature".into()));
        assert!(delete_local_branch(&git_bin(), &repo, "develop", true).is_err());
    }

    #[test]
    fn does_not_mark_new_branch_at_merge_target_as_merged() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "fresh"]);

        let overview = branch_overview_with(&git_bin(), &repo, Some("develop"), true).unwrap();
        let fresh = overview
            .branches
            .iter()
            .find(|branch| branch.name == "fresh")
            .unwrap();
        assert!(fresh.current);
        assert!(!fresh.merged);
        assert!(!fresh.partial);
        assert!(!fresh.pending);

        git(&repo, &["checkout", "develop"]);
        let leftover =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), false, None).unwrap();
        assert!(leftover.deleted.is_empty());
        assert!(leftover.refused.is_empty());
        assert!(leftover.message.contains("No merged"));
        assert!(local_branches(&git_bin(), &repo)
            .unwrap()
            .contains(&"fresh".into()));
    }

    #[test]
    fn marks_squash_merged_local_branches() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["merge", "--squash", "feature"]);
        git(&repo, &["commit", "-m", "squash feature"]);

        let overview = branch_overview_with(&git_bin(), &repo, Some("develop"), true).unwrap();
        let feature = overview
            .branches
            .iter()
            .find(|branch| branch.name == "feature")
            .unwrap();
        assert!(feature.merged);
        assert!(!feature.partial);
        assert!(!feature.pending);
        assert!(!feature.current);
    }

    #[test]
    fn snapshot_marks_squash_merged_branches_pending() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["merge", "--squash", "feature"]);
        git(&repo, &["commit", "-m", "squash feature"]);

        let snapshot = branch_overview_with(&git_bin(), &repo, Some("develop"), false).unwrap();
        let feature = snapshot
            .branches
            .iter()
            .find(|branch| branch.name == "feature")
            .unwrap();
        assert!(feature.pending);
        assert!(!feature.merged);
        assert!(!feature.partial);
        let develop = snapshot
            .branches
            .iter()
            .find(|branch| branch.name == "develop")
            .unwrap();
        assert!(!develop.pending);
        assert!(develop.protected_branch);
        let classified = branch_overview_with(&git_bin(), &repo, Some("develop"), true).unwrap();
        let snapshot_names: Vec<&str> = snapshot
            .branches
            .iter()
            .map(|branch| branch.name.as_str())
            .collect();
        let classified_names: Vec<&str> = classified
            .branches
            .iter()
            .map(|branch| branch.name.as_str())
            .collect();
        assert_eq!(snapshot_names, classified_names);
        assert_eq!(snapshot_names.first().copied(), Some("develop"));
    }

    #[test]
    fn overview_keeps_name_order_when_marks_fill_in() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "aaa"]);
        fs::write(repo.join("aaa.txt"), "squash\n").unwrap();
        git(&repo, &["add", "aaa.txt"]);
        git(&repo, &["commit", "-m", "aaa"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["merge", "--squash", "aaa"]);
        git(&repo, &["commit", "-m", "squash aaa"]);
        git(&repo, &["checkout", "-b", "mid"]);
        fs::write(repo.join("mid.txt"), "merged\n").unwrap();
        git(&repo, &["add", "mid.txt"]);
        git(&repo, &["commit", "-m", "mid"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["merge", "mid"]);
        git(&repo, &["checkout", "-b", "zzz"]);
        fs::write(repo.join("zzz.txt"), "unique\n").unwrap();
        git(&repo, &["add", "zzz.txt"]);
        git(&repo, &["commit", "-m", "zzz"]);
        git(&repo, &["checkout", "develop"]);

        let snapshot = branch_overview_with(&git_bin(), &repo, Some("develop"), false).unwrap();
        let classified = branch_overview_with(&git_bin(), &repo, Some("develop"), true).unwrap();
        let names = |overview: &BranchOverview| {
            overview
                .branches
                .iter()
                .map(|branch| branch.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(&snapshot),
            vec!["develop", "aaa", "mid", "zzz"]
        );
        assert_eq!(names(&snapshot), names(&classified));
        let aaa = classified
            .branches
            .iter()
            .find(|branch| branch.name == "aaa")
            .unwrap();
        assert!(aaa.merged);
        assert!(!aaa.pending);
    }

    #[test]
    fn marks_partially_merged_local_branches() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("one.txt"), "one\n").unwrap();
        git(&repo, &["add", "one.txt"]);
        git(&repo, &["commit", "-m", "first change"]);
        fs::write(repo.join("two.txt"), "two\n").unwrap();
        git(&repo, &["add", "two.txt"]);
        git(&repo, &["commit", "-m", "second change"]);
        git(&repo, &["checkout", "develop"]);
        fs::write(repo.join("one.txt"), "one\n").unwrap();
        git(&repo, &["add", "one.txt"]);
        git(&repo, &["commit", "-m", "same first change"]);

        let overview = branch_overview_with(&git_bin(), &repo, Some("develop"), true).unwrap();
        let feature = overview
            .branches
            .iter()
            .find(|branch| branch.name == "feature")
            .unwrap();
        assert!(!feature.merged);
        assert!(feature.partial);
        assert!(!feature.pending);
        let leftover =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), false, None).unwrap();
        assert!(leftover.deleted.is_empty());
        let names = local_branches(&git_bin(), &repo).unwrap();
        assert!(names.contains(&"feature".into()));
    }

    #[test]
    fn delete_merged_refuses_when_not_merged_into_head() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "other"]);
        fs::write(repo.join("other.txt"), "other\n").unwrap();
        git(&repo, &["add", "other.txt"]);
        git(&repo, &["commit", "-m", "other work"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["merge", "--no-ff", "-m", "merge feature", "feature"]);
        git(&repo, &["checkout", "other"]);

        let refused =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), false, None).unwrap();
        assert!(refused.deleted.is_empty());
        assert_eq!(refused.refused, vec!["feature".to_string()]);
        assert!(local_branches(&git_bin(), &repo)
            .unwrap()
            .contains(&"feature".into()));

        let forced =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), true, None).unwrap();
        assert_eq!(forced.deleted, vec!["feature".to_string()]);
        assert!(forced.refused.is_empty());
        assert!(!local_branches(&git_bin(), &repo)
            .unwrap()
            .contains(&"feature".into()));
    }

    #[test]
    fn deletes_several_merged_branches_in_one_pass() {
        let repo = init_repo();
        for name in ["one", "two", "three"] {
            git(&repo, &["checkout", "-b", name]);
            fs::write(repo.join(format!("{name}.txt")), "work\n").unwrap();
            git(&repo, &["add", &format!("{name}.txt")]);
            git(&repo, &["commit", "-m", name]);
            git(&repo, &["checkout", "develop"]);
            git(&repo, &["merge", "--no-ff", "-m", &format!("merge {name}"), name]);
        }

        let deleted =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), false, None).unwrap();
        let mut names = deleted.deleted.clone();
        names.sort();
        assert_eq!(
            names,
            vec!["one".to_string(), "three".to_string(), "two".to_string()]
        );
        let leftover = local_branches(&git_bin(), &repo).unwrap();
        assert_eq!(leftover, vec!["develop".to_string()]);
    }

    #[test]
    fn delete_named_branches_skips_rescan() {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("feature.txt"), "work\n").unwrap();
        git(&repo, &["add", "feature.txt"]);
        git(&repo, &["commit", "-m", "feature work"]);
        git(&repo, &["checkout", "develop"]);
        git(&repo, &["merge", "--squash", "feature"]);
        git(&repo, &["commit", "-m", "squash feature"]);

        let names = vec!["feature".to_string()];
        let refused =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), false, Some(&names))
                .unwrap();
        assert!(refused.deleted.is_empty());
        assert_eq!(refused.refused, vec!["feature".to_string()]);

        let forced =
            delete_merged_branches(&git_bin(), &repo, Some("develop"), true, Some(&names)).unwrap();
        assert_eq!(forced.deleted, vec!["feature".to_string()]);
        assert!(!local_branches(&git_bin(), &repo)
            .unwrap()
            .contains(&"feature".into()));
    }

    #[test]
    fn parses_batch_branch_delete_output() {
        let output = GitOutput {
            stdout: "Deleted branch one (was abc123).\nDeleted branch two (was def456).\n".into(),
            stderr: "error: The branch 'three' is not fully merged.\nIf you are sure you want to delete it, run 'git branch -D three'.\n".into(),
            success: false,
        };
        let (deleted, refused, errors) = classify_branch_delete_output(
            &["one".into(), "two".into(), "three".into()],
            &output,
        );
        assert_eq!(deleted, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(refused, vec!["three".to_string()]);
        assert!(errors.is_empty());
    }

    fn bare_origin_with_clone() -> (PathBuf, PathBuf) {
        let seed = init_repo();
        let origin = temp_dir();
        git(&origin, &["init", "--bare", "-b", "develop"]);
        git(&seed, &["remote", "add", "origin", origin.to_str().unwrap()]);
        git(&seed, &["push", "-u", "origin", "develop"]);

        let work = temp_dir();
        git(&work, &["clone", origin.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);
        (origin, work)
    }

    fn commit_file(repo: &Path, name: &str, contents: &str) {
        fs::write(repo.join(name), contents).unwrap();
        git(repo, &["add", name]);
        git(repo, &["commit", "-m", name]);
    }

    /// A fork layout: `work` clones `origin` (the fork) and adds `upstream`. `upstream_dev`
    /// is a clone of upstream used to land new upstream commits.
    struct Fork {
        origin: PathBuf,
        upstream_dev: PathBuf,
        work: PathBuf,
    }

    fn fork_with_upstream() -> Fork {
        let seed = init_repo();
        let upstream = temp_dir();
        git(&upstream, &["init", "--bare", "-b", "develop"]);
        git(&seed, &["remote", "add", "origin", upstream.to_str().unwrap()]);
        git(&seed, &["push", "-u", "origin", "develop"]);

        let origin = temp_dir();
        git(&origin, &["clone", "--bare", upstream.to_str().unwrap(), "."]);

        let work = temp_dir();
        git(&work, &["clone", origin.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);
        add_remote(&git_bin(), &work, "upstream", upstream.to_str().unwrap()).unwrap();

        let upstream_dev = temp_dir();
        git(&upstream_dev, &["clone", upstream.to_str().unwrap(), "."]);
        git(&upstream_dev, &["config", "user.name", "Shipyard Test"]);
        git(&upstream_dev, &["config", "user.email", "test@shipyard.local"]);
        Fork {
            origin,
            upstream_dev,
            work,
        }
    }

    fn land_upstream_commit(fork: &Fork, name: &str) {
        commit_file(&fork.upstream_dev, name, "upstream\n");
        git(&fork.upstream_dev, &["push", "origin", "develop"]);
        fetch_named_remote(&git_bin(), &fork.work, "upstream").unwrap();
    }

    #[test]
    fn validates_remote_names() {
        assert!(validate_remote_name("upstream").is_ok());
        assert!(validate_remote_name("my-fork_2.old").is_ok());
        for bad in ["", "-x", ".x", "a b", "a/b", "x.lock", "a;b"] {
            assert!(validate_remote_name(bad).is_err(), "{bad} should be rejected");
        }
        assert!(validate_remote_url("--upload-pack=evil").is_err());
        assert!(validate_remote_url("git@github.com:owner/repo.git").is_ok());
    }

    #[test]
    fn lists_remotes_origin_first_with_branch_counts() {
        let fork = fork_with_upstream();
        let remotes = list_remotes(&git_bin(), &fork.work).unwrap();
        let names: Vec<&str> = remotes.iter().map(|remote| remote.name.as_str()).collect();
        assert_eq!(names, ["origin", "upstream"]);
        assert_eq!(remotes[0].branch_count, 1);
        assert_eq!(remotes[1].branch_count, 1);
        assert_eq!(remotes[0].fetch_url, fork.origin.to_str().unwrap());
    }

    #[test]
    fn remote_branches_compare_against_same_named_local_branch() {
        let fork = fork_with_upstream();
        land_upstream_commit(&fork, "one.txt");
        land_upstream_commit(&fork, "two.txt");

        let overview = remote_branches(&git_bin(), &fork.work, "upstream").unwrap();
        let develop = &overview.branches[0];
        assert_eq!(develop.name, "develop");
        assert_eq!(develop.local.as_deref(), Some("develop"));
        assert!(!develop.tracked, "local develop tracks origin, not upstream");
        assert!(develop.current);
        assert_eq!((develop.ahead, develop.behind), (0, 2));

        let origin = remote_branches(&git_bin(), &fork.work, "origin").unwrap();
        assert!(origin.branches[0].tracked);
        assert!(origin.branches[0].is_default);
    }

    #[test]
    fn syncs_fork_from_upstream_then_pushes_to_origin() {
        let fork = fork_with_upstream();
        land_upstream_commit(&fork, "sync.txt");

        let message =
            merge_remote_branch(&git_bin(), &fork.work, "upstream", "develop", "develop", false).unwrap();
        assert!(message.starts_with("Fast-forwarded develop"), "{message}");
        assert!(fork.work.join("sync.txt").exists());

        push_local_branch(&git_bin(), &fork.work, "develop").unwrap();
        let origin_tip = git(&fork.origin, &["rev-parse", "develop"]).stdout;
        let upstream_tip = git(&fork.upstream_dev, &["rev-parse", "HEAD"]).stdout;
        assert_eq!(origin_tip, upstream_tip);

        let again =
            merge_remote_branch(&git_bin(), &fork.work, "upstream", "develop", "develop", false).unwrap();
        assert!(again.contains("already has everything"), "{again}");
    }

    #[test]
    fn fast_forwards_a_branch_that_is_not_checked_out() {
        let fork = fork_with_upstream();
        git(&fork.work, &["checkout", "-b", "feature"]);
        land_upstream_commit(&fork, "ff.txt");

        merge_remote_branch(&git_bin(), &fork.work, "upstream", "develop", "develop", false).unwrap();
        assert_eq!(current_branch(&git_bin(), &fork.work).unwrap(), "feature");
        let develop = git(&fork.work, &["rev-parse", "develop"]).stdout;
        let upstream = git(&fork.work, &["rev-parse", "upstream/develop"]).stdout;
        assert_eq!(develop, upstream);
        assert!(!fork.work.join("ff.txt").exists());
    }

    #[test]
    fn diverged_sync_needs_permission_for_a_merge_commit() {
        let fork = fork_with_upstream();
        commit_file(&fork.work, "local.txt", "local\n");
        land_upstream_commit(&fork, "remote.txt");

        let err = merge_remote_branch(&git_bin(), &fork.work, "upstream", "develop", "develop", false)
            .unwrap_err();
        assert!(err.starts_with(NOT_FAST_FORWARD_PREFIX), "{err}");
        assert!(!fork.work.join("remote.txt").exists());

        let message =
            merge_remote_branch(&git_bin(), &fork.work, "upstream", "develop", "develop", true).unwrap();
        assert!(message.starts_with("Merged upstream/develop"), "{message}");
        assert!(fork.work.join("remote.txt").exists());
        assert!(fork.work.join("local.txt").exists());
    }

    #[test]
    fn checkout_remote_branch_asks_for_a_name_when_local_exists() {
        let fork = fork_with_upstream();
        let err = checkout_remote_branch(&git_bin(), &fork.work, "upstream", "develop", None).unwrap_err();
        assert!(err.starts_with(LOCAL_BRANCH_EXISTS_PREFIX), "{err}");

        checkout_remote_branch(&git_bin(), &fork.work, "upstream", "develop", Some("upstream-develop"))
            .unwrap();
        assert_eq!(current_branch(&git_bin(), &fork.work).unwrap(), "upstream-develop");
        let upstream = git(&fork.work, &["rev-parse", "--abbrev-ref", "@{upstream}"]).stdout;
        assert_eq!(upstream.trim(), "upstream/develop");

        git(&fork.work, &["checkout", "develop"]);
        checkout_remote_branch(&git_bin(), &fork.work, "upstream", "develop", None).unwrap();
        assert_eq!(current_branch(&git_bin(), &fork.work).unwrap(), "upstream-develop");
    }

    #[test]
    fn deletes_remote_branch_but_not_the_default() {
        let fork = fork_with_upstream();
        git(&fork.work, &["push", "origin", "develop:old-feature"]);
        fetch_named_remote(&git_bin(), &fork.work, "origin").unwrap();

        let err = delete_remote_branch(&git_bin(), &fork.work, "origin", "develop").unwrap_err();
        assert!(err.contains("default branch"), "{err}");

        delete_remote_branch(&git_bin(), &fork.work, "origin", "old-feature").unwrap();
        assert!(!ref_exists(&git_bin(), &fork.origin, "refs/heads/old-feature"));
        assert!(!ref_exists(&git_bin(), &fork.work, "refs/remotes/origin/old-feature"));
    }

    #[test]
    fn renames_and_removes_remotes() {
        let fork = fork_with_upstream();
        let url = list_remotes(&git_bin(), &fork.work).unwrap()[1].fetch_url.clone();
        update_remote(&git_bin(), &fork.work, "upstream", "source", &url).unwrap();
        assert!(ref_exists(&git_bin(), &fork.work, "refs/remotes/source/develop"));
        assert!(add_remote(&git_bin(), &fork.work, "source", &url).is_err());

        checkout_remote_branch(&git_bin(), &fork.work, "source", "develop", Some("src")).unwrap();
        let message = remove_remote(&git_bin(), &fork.work, "source").unwrap();
        assert!(message.contains("src"), "{message}");
        assert!(!ref_exists(&git_bin(), &fork.work, "refs/remotes/source/develop"));
    }

    #[test]
    fn pull_links_missing_upstream_to_same_named_origin_branch() {
        let (origin, work) = bare_origin_with_clone();
        git(&work, &["checkout", "-b", "feature"]);
        git(&work, &["push", "origin", "feature"]);
        assert!(!has_upstream(&git_bin(), &work));

        let other = temp_dir();
        git(&other, &["clone", origin.to_str().unwrap(), "."]);
        git(&other, &["config", "user.name", "Shipyard Test"]);
        git(&other, &["config", "user.email", "test@shipyard.local"]);
        git(&other, &["checkout", "feature"]);
        commit_file(&other, "remote.txt", "remote\n");
        git(&other, &["push"]);

        git(&work, &["fetch"]);
        pull(&git_bin(), &work).unwrap();
        assert!(has_upstream(&git_bin(), &work));
        assert!(work.join("remote.txt").exists());
    }

    #[test]
    fn push_sets_upstream_for_new_branch() {
        let (origin, work) = bare_origin_with_clone();
        git(&work, &["checkout", "-b", "fresh"]);
        commit_file(&work, "fresh.txt", "fresh\n");
        push(&git_bin(), &work).unwrap();
        assert!(has_upstream(&git_bin(), &work));
        assert!(ref_exists(&git_bin(), &origin, "refs/heads/fresh"));
    }

    #[test]
    fn diverged_push_is_rejected_with_pull_hint_and_never_forced() {
        let (origin, work) = bare_origin_with_clone();
        let other = temp_dir();
        git(&other, &["clone", origin.to_str().unwrap(), "."]);
        git(&other, &["config", "user.name", "Shipyard Test"]);
        git(&other, &["config", "user.email", "test@shipyard.local"]);
        commit_file(&other, "theirs.txt", "theirs\n");
        git(&other, &["push"]);
        let remote_tip = git(&origin, &["rev-parse", "develop"]).stdout;

        commit_file(&work, "mine.txt", "mine\n");
        let err = push(&git_bin(), &work).unwrap_err();
        assert!(err.starts_with(PUSH_REJECTED_PREFIX), "{err}");
        assert_eq!(git(&origin, &["rev-parse", "develop"]).stdout, remote_tip);

        pull(&git_bin(), &work).unwrap();
        push(&git_bin(), &work).unwrap();
        assert!(work.join("theirs.txt").exists());
    }

    #[test]
    fn push_and_pull_use_standard_git_without_force() {
        let seed = init_repo();
        let origin = temp_dir();
        git(&origin, &["init", "--bare", "-b", "develop"]);
        git(&seed, &["remote", "add", "origin", origin.to_str().unwrap()]);
        git(&seed, &["push", "-u", "origin", "develop"]);

        let work = temp_dir();
        git(&work, &["clone", origin.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);
        fs::write(work.join("README.md"), "from work\n").unwrap();
        git(&work, &["add", "README.md"]);
        git(&work, &["commit", "-m", "work commit"]);
        push(&git_bin(), &work).unwrap();

        let other = temp_dir();
        git(&other, &["clone", origin.to_str().unwrap(), "."]);
        git(&other, &["config", "user.name", "Shipyard Test"]);
        git(&other, &["config", "user.email", "test@shipyard.local"]);
        assert!(fs::read_to_string(other.join("README.md"))
            .unwrap()
            .contains("from work"));

        fs::write(work.join("README.md"), "from upstream\n").unwrap();
        git(&work, &["add", "README.md"]);
        git(&work, &["commit", "-m", "upstream commit"]);
        push(&git_bin(), &work).unwrap();
        pull(&git_bin(), &other).unwrap();
        assert!(fs::read_to_string(other.join("README.md"))
            .unwrap()
            .contains("from upstream"));
    }

    #[test]
    fn reports_ahead_and_behind_counts() {
        let upstream = init_repo();
        let work = temp_dir();
        git(&work, &["clone", upstream.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);
        let even = live_status(&git_bin(), &work).unwrap();
        assert_eq!((even.ahead, even.behind), (0, 0));

        fs::write(upstream.join("README.md"), "upstream\n").unwrap();
        git(&upstream, &["add", "README.md"]);
        git(&upstream, &["commit", "-m", "upstream commit"]);
        git(&work, &["fetch"]);
        let behind = live_status(&git_bin(), &work).unwrap();
        assert_eq!((behind.ahead, behind.behind), (0, 1));

        fs::write(work.join("local.txt"), "mine\n").unwrap();
        git(&work, &["add", "local.txt"]);
        git(&work, &["commit", "-m", "local commit"]);
        let diverged = live_status(&git_bin(), &work).unwrap();
        assert_eq!((diverged.ahead, diverged.behind), (1, 1));
    }

    fn tracking_named(items: &[BranchTracking], name: &str) -> BranchTracking {
        items
            .iter()
            .find(|item| item.name == name)
            .cloned()
            .unwrap_or_else(|| panic!("missing branch {name}"))
    }

    #[test]
    fn marks_local_only_branches_without_a_remote() {
        let repo = init_repo();
        git(&repo, &["branch", "feature"]);
        let items = local_branch_tracking(&git_bin(), &repo).unwrap();
        let develop = tracking_named(&items, "develop");
        let feature = tracking_named(&items, "feature");
        assert!(develop.local_only);
        assert_eq!((develop.ahead, develop.behind), (0, 0));
        assert!(develop.upstream.is_none());
        assert!(feature.local_only);
        assert_eq!((feature.ahead, feature.behind), (0, 0));
    }

    #[test]
    fn reports_branch_ahead_behind_against_upstream() {
        let upstream = init_repo();
        let work = temp_dir();
        git(&work, &["clone", upstream.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);

        let even = tracking_named(&local_branch_tracking(&git_bin(), &work).unwrap(), "develop");
        assert!(!even.local_only);
        assert_eq!((even.ahead, even.behind), (0, 0));
        assert_eq!(even.upstream.as_deref(), Some("origin/develop"));

        git(&work, &["branch", "feature"]);
        let local = tracking_named(&local_branch_tracking(&git_bin(), &work).unwrap(), "feature");
        assert!(local.local_only);

        fs::write(upstream.join("README.md"), "upstream\n").unwrap();
        git(&upstream, &["add", "README.md"]);
        git(&upstream, &["commit", "-m", "upstream commit"]);
        git(&work, &["fetch"]);
        let behind = tracking_named(&local_branch_tracking(&git_bin(), &work).unwrap(), "develop");
        assert_eq!((behind.ahead, behind.behind), (0, 1));

        fs::write(work.join("local.txt"), "mine\n").unwrap();
        git(&work, &["add", "local.txt"]);
        git(&work, &["commit", "-m", "local commit"]);
        let diverged = tracking_named(&local_branch_tracking(&git_bin(), &work).unwrap(), "develop");
        assert!(!diverged.local_only);
        assert_eq!((diverged.ahead, diverged.behind), (1, 1));
    }

    #[test]
    fn treats_matching_origin_ref_as_remote_without_upstream() {
        let upstream = init_repo();
        let work = temp_dir();
        git(&work, &["clone", upstream.to_str().unwrap(), "."]);
        git(&work, &["config", "user.name", "Shipyard Test"]);
        git(&work, &["config", "user.email", "test@shipyard.local"]);
        git(&work, &["branch", "feature"]);
        let feature_tip = git(&work, &["rev-parse", "refs/heads/feature"])
            .stdout
            .trim()
            .to_string();
        git(
            &work,
            &["update-ref", "refs/remotes/origin/feature", &feature_tip],
        );

        let even = tracking_named(&local_branch_tracking(&git_bin(), &work).unwrap(), "feature");
        assert!(!even.local_only);
        assert_eq!((even.ahead, even.behind), (0, 0));
        assert_eq!(even.upstream.as_deref(), Some("origin/feature"));

        git(&work, &["checkout", "feature"]);
        fs::write(work.join("feature.txt"), "only local\n").unwrap();
        git(&work, &["add", "feature.txt"]);
        git(&work, &["commit", "-m", "local feature"]);
        let ahead = tracking_named(&local_branch_tracking(&git_bin(), &work).unwrap(), "feature");
        assert!(!ahead.local_only);
        assert_eq!((ahead.ahead, ahead.behind), (1, 0));
    }

    #[test]
    fn parse_upstream_track_reads_ahead_behind_and_gone() {
        assert_eq!(parse_upstream_track(""), Some((0, 0)));
        assert_eq!(parse_upstream_track("[ahead 2]"), Some((2, 0)));
        assert_eq!(parse_upstream_track("[behind 3]"), Some((0, 3)));
        assert_eq!(parse_upstream_track("[ahead 2, behind 3]"), Some((2, 3)));
        assert_eq!(parse_upstream_track("[gone]"), None);
    }

    #[test]
    fn lists_commit_files_and_file_diff() {
        let repo = init_repo();
        let commits = log_graph(&git_bin(), &repo).unwrap();
        let initial = &commits[0].hash;
        let files = commit_files(&git_bin(), &repo, initial).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "README.md");
        assert_eq!(files[0].status, "Added");
        let added = commit_file_diff(&git_bin(), &repo, initial, "README.md").unwrap();
        assert!(added.contains("+hello"));

        fs::write(repo.join("README.md"), "changed\n").unwrap();
        fs::write(repo.join("new.txt"), "fresh\n").unwrap();
        git(&repo, &["add", "README.md", "new.txt"]);
        git(&repo, &["commit", "-m", "update files"]);
        let commits = log_graph(&git_bin(), &repo).unwrap();
        let update = &commits[0].hash;
        let files = commit_files(&git_bin(), &repo, update).unwrap();
        assert!(files.iter().any(|file| file.path == "README.md" && file.status == "Modified"));
        assert!(files.iter().any(|file| file.path == "new.txt" && file.status == "Added"));
        let diff = commit_file_diff(&git_bin(), &repo, update, "README.md").unwrap();
        assert!(diff.contains("-hello") || diff.contains("+changed"));

        git(&repo, &["mv", "new.txt", "renamed.txt"]);
        git(&repo, &["commit", "-m", "rename file"]);
        let commits = log_graph(&git_bin(), &repo).unwrap();
        let renamed = commit_files(&git_bin(), &repo, &commits[0].hash).unwrap();
        assert!(renamed.iter().any(|file| {
            file.path == "renamed.txt"
                && file.status == "Renamed"
                && file.old_path.as_deref() == Some("new.txt")
        }));

        git(&repo, &["rm", "renamed.txt"]);
        git(&repo, &["commit", "-m", "remove file"]);
        let commits = log_graph(&git_bin(), &repo).unwrap();
        let deleted = commit_files(&git_bin(), &repo, &commits[0].hash).unwrap();
        assert!(deleted.iter().any(|file| file.path == "renamed.txt" && file.status == "Deleted"));

        assert!(commit_files(&git_bin(), &repo, "not-a-hash").is_err());
        assert!(commit_file_diff(&git_bin(), &repo, initial, "").is_err());
    }

    #[test]
    fn lists_applies_pops_and_drops_stashes() {
        let repo = init_repo();
        assert!(stash_list(&git_bin(), &repo).unwrap().is_empty());
        assert!(stash_apply(&git_bin(), &repo, 0).is_err());
        assert!(stash_push(&git_bin(), &repo, "").is_err());

        fs::write(repo.join("README.md"), "stashed-a\n").unwrap();
        fs::write(repo.join("notes.txt"), "untracked\n").unwrap();
        stash_push(&git_bin(), &repo, "first stash").unwrap();
        assert!(working_tree(&git_bin(), &repo).unwrap().is_empty());
        assert!(!repo.join("notes.txt").exists());
        fs::write(repo.join("README.md"), "stashed-b\n").unwrap();
        git(&repo, &["stash", "push", "-m", "second stash"]);

        let stashes = stash_list(&git_bin(), &repo).unwrap();
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].index, 0);
        assert_eq!(stashes[1].index, 1);
        assert!(stashes[0].message.contains("second stash"));
        assert!(stashes[1].message.contains("first stash"));

        stash_apply(&git_bin(), &repo, 0).unwrap();
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(files.iter().any(|file| file.path == "README.md"));
        assert_eq!(stash_list(&git_bin(), &repo).unwrap().len(), 2);
        assert!(fs::read_to_string(repo.join("README.md"))
            .unwrap()
            .contains("stashed-b"));

        git(&repo, &["checkout", "--", "README.md"]);
        stash_drop(&git_bin(), &repo, 1).unwrap();
        let after_drop = stash_list(&git_bin(), &repo).unwrap();
        assert_eq!(after_drop.len(), 1);
        assert!(after_drop[0].message.contains("second stash"));

        stash_pop(&git_bin(), &repo, 0).unwrap();
        assert!(stash_list(&git_bin(), &repo).unwrap().is_empty());
        assert!(fs::read_to_string(repo.join("README.md"))
            .unwrap()
            .contains("stashed-b"));
    }

    #[test]
    fn lists_creates_and_deletes_tags() {
        let repo = init_repo();
        assert!(tag_list(&git_bin(), &repo).unwrap().is_empty());
        assert!(create_tag(&git_bin(), &repo, "", "", "").is_err());
        assert!(create_tag(&git_bin(), &repo, "bad name", "", "").is_err());
        assert!(delete_tag(&git_bin(), &repo, "missing").is_err());

        create_tag(&git_bin(), &repo, "v1.0.0", "", "").unwrap();
        let tags = tag_list(&git_bin(), &repo).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0.0");
        assert!(!tags[0].annotated);
        assert!(!tags[0].hash.is_empty());
        assert!(create_tag(&git_bin(), &repo, "v1.0.0", "", "").is_err());

        create_tag(&git_bin(), &repo, "v1.1.0", "First annotated", "").unwrap();
        let tags = tag_list(&git_bin(), &repo).unwrap();
        assert_eq!(tags.len(), 2);
        let annotated = tags.iter().find(|tag| tag.name == "v1.1.0").unwrap();
        assert!(annotated.annotated);
        assert!(annotated.message.contains("First annotated"));
        assert!(!annotated.hash.is_empty());

        fs::write(repo.join("README.md"), "later\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "later commit"]);
        let commits = log_graph(&git_bin(), &repo).unwrap();
        let older = commits[1].hash.clone();
        create_tag(&git_bin(), &repo, "v0.9.0", "", &older).unwrap();
        let tags = tag_list(&git_bin(), &repo).unwrap();
        let older_tag = tags.iter().find(|tag| tag.name == "v0.9.0").unwrap();
        assert!(older.starts_with(&older_tag.hash) || older_tag.hash == older);

        delete_tag(&git_bin(), &repo, "v1.0.0").unwrap();
        let names: Vec<_> = tag_list(&git_bin(), &repo)
            .unwrap()
            .into_iter()
            .map(|tag| tag.name)
            .collect();
        assert!(!names.iter().any(|name| name == "v1.0.0"));
        assert!(names.iter().any(|name| name == "v1.1.0"));
        assert!(names.iter().any(|name| name == "v0.9.0"));
    }

    #[test]
    fn pull_from_named_branch_into_current() {
        let upstream = init_repo();
        fs::write(upstream.join("README.md"), "upstream develop\n").unwrap();
        git(&upstream, &["add", "README.md"]);
        git(&upstream, &["commit", "-m", "develop update"]);

        let work = temp_dir();
        git(&work, &["clone", upstream.to_str().unwrap(), "."]);
        git(&work, &["checkout", "-b", "feature"]);
        let pull = run_git(&git_bin(), &work, &["pull", "origin", "develop"]).unwrap();
        assert!(pull.success, "{}", combined_message(&pull));
        assert_eq!(current_branch(&git_bin(), &work).unwrap(), "feature");
        let contents = fs::read_to_string(work.join("README.md")).unwrap();
        assert!(contents.contains("upstream develop"));
    }

    fn isolated_gitconfig() -> PathBuf {
        temp_dir().join("gitconfig")
    }

    #[test]
    fn reads_empty_git_config_when_file_is_missing() {
        let file = isolated_gitconfig();
        let config = read_git_config_at(&git_bin(), Some(&file)).unwrap();
        assert_eq!(config.path, file.to_str().unwrap());
        assert!(config.contents.is_empty());
        assert!(config.user_name.is_empty());
        assert!(config.user_email.is_empty());
        assert!(config.default_branch.is_empty());
        assert!(config.pull_rebase.is_empty());
        assert!(config.default_remote.is_empty());
    }

    #[test]
    fn sets_and_unsets_git_identity() {
        let file = isolated_gitconfig();
        let git = git_bin();
        let config = update_git_config_value_at(&git, "user.name", "Ada Lovelace", Some(&file)).unwrap();
        assert_eq!(config.user_name, "Ada Lovelace");
        let config =
            update_git_config_value_at(&git, "user.email", "ada@shipyard.local", Some(&file)).unwrap();
        assert_eq!(config.user_email, "ada@shipyard.local");
        assert!(config.contents.contains("Ada Lovelace"));
        let config = update_git_config_value_at(&git, "user.name", "  ", Some(&file)).unwrap();
        assert!(config.user_name.is_empty());
        assert_eq!(config.user_email, "ada@shipyard.local");
    }

    #[test]
    fn sets_default_branch_and_pull_rebase() {
        let file = isolated_gitconfig();
        let git = git_bin();
        let config =
            update_git_config_value_at(&git, "init.defaultBranch", "develop", Some(&file)).unwrap();
        assert_eq!(config.default_branch, "develop");
        let config = update_git_config_value_at(&git, "pull.rebase", "true", Some(&file)).unwrap();
        assert_eq!(config.pull_rebase, "true");
        let config = update_git_config_value_at(&git, "pull.rebase", "FALSE", Some(&file)).unwrap();
        assert_eq!(config.pull_rebase, "false");
        let config = update_git_config_value_at(&git, "pull.rebase", "", Some(&file)).unwrap();
        assert!(config.pull_rebase.is_empty());
        let config =
            update_git_config_value_at(&git, "checkout.defaultRemote", "origin", Some(&file)).unwrap();
        assert_eq!(config.default_remote, "origin");
        let config =
            update_git_config_value_at(&git, "checkout.defaultRemote", "", Some(&file)).unwrap();
        assert!(config.default_remote.is_empty());
    }

    #[test]
    fn rejects_unknown_or_invalid_git_config_values() {
        let file = isolated_gitconfig();
        let git = git_bin();
        assert!(update_git_config_value_at(&git, "core.editor", "vim", Some(&file)).is_err());
        assert!(update_git_config_value_at(&git, "user.name", "line\nbreak", Some(&file)).is_err());
        assert!(update_git_config_value_at(&git, "init.defaultBranch", "..nope", Some(&file)).is_err());
        assert!(update_git_config_value_at(&git, "pull.rebase", "interactive", Some(&file)).is_err());
        assert!(update_git_config_value_at(&git, "checkout.defaultRemote", "..nope", Some(&file)).is_err());
        assert!(!file.exists());
    }

    #[test]
    fn writes_git_config_file_and_restores_invalid_edits() {
        let file = isolated_gitconfig();
        let git = git_bin();
        let config = write_git_config_at(
            &git,
            "[user]\n\tname = From File\n\temail = file@shipyard.local\n",
            Some(&file),
        )
        .unwrap();
        assert_eq!(config.user_name, "From File");
        assert_eq!(config.user_email, "file@shipyard.local");
        assert!(config.contents.contains("From File"));

        let err = write_git_config_at(&git, "[user\n\tname = broken\n", Some(&file)).unwrap_err();
        assert!(err.contains("valid") || err.contains("fatal") || err.contains("error"));
        let restored = read_git_config_at(&git, Some(&file)).unwrap();
        assert_eq!(restored.user_name, "From File");
        assert!(restored.contents.contains("From File"));
    }

    #[test]
    fn maps_editor_setting_to_macos_app_name() {
        assert_eq!(editor_app_name("system"), None);
        assert_eq!(editor_app_name("cursor").as_deref(), Some("Cursor"));
        assert_eq!(
            editor_app_name("vscode").as_deref(),
            Some("Visual Studio Code")
        );
        assert_eq!(editor_app_name("BBEdit").as_deref(), Some("BBEdit"));
    }

    #[test]
    fn remote_browse_url_rewrites_https_ssh_and_scp() {
        assert_eq!(
            remote_browse_url("https://github.com/owner/repo.git").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            remote_browse_url("https://user:token@github.com/owner/repo.git/").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            remote_browse_url("git@github.com:owner/repo.git").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            remote_browse_url("org-123@github.com:owner/repo.git").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            remote_browse_url("ssh://git@github.com/owner/repo.git").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            remote_browse_url("ssh://git@github.com:22/owner/repo.git").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            remote_browse_url("git@gitlab.com:group/sub/repo.git").unwrap(),
            "https://gitlab.com/group/sub/repo"
        );
        assert_eq!(
            remote_browse_url("git@bitbucket.org:owner/repo.git").unwrap(),
            "https://bitbucket.org/owner/repo"
        );
        assert_eq!(
            remote_browse_url("git://github.com/owner/repo.git").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(
            commit_browse_url("https://github.com/owner/repo", "abc1234").unwrap(),
            "https://github.com/owner/repo/commit/abc1234"
        );
        assert_eq!(
            commit_browse_url("https://gitlab.com/group/sub/repo", "abc1234").unwrap(),
            "https://gitlab.com/group/sub/repo/-/commit/abc1234"
        );
        assert_eq!(
            commit_browse_url("https://bitbucket.org/owner/repo", "abc1234").unwrap(),
            "https://bitbucket.org/owner/repo/commits/abc1234"
        );
        assert!(commit_browse_url("https://github.com/owner/repo", "bad hash").is_err());
    }

    #[test]
    fn remote_browse_url_rejects_local_and_empty_remotes() {
        assert!(remote_browse_url("").is_err());
        assert!(remote_browse_url("   ").is_err());
        assert!(remote_browse_url("file:///Users/me/repo").is_err());
        assert!(remote_browse_url("/Users/me/repo").is_err());
        assert!(remote_browse_url("C:\\Users\\me\\repo").is_err());
        assert!(remote_browse_url("not a remote").is_err());
    }

    #[test]
    fn checks_out_creates_cherry_picks_and_reverts_commits() {
        let repo = init_repo();
        let first = head_commit(&git_bin(), &repo).unwrap();
        fs::write(repo.join("README.md"), "second\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "second"]);

        let detached = checkout_commit(&git_bin(), &repo, &first).unwrap();
        assert!(detached.to_lowercase().contains("detach") || detached.contains(&first[..7]));
        assert!(current_branch(&git_bin(), &repo)
            .unwrap()
            .starts_with("detached"));
        assert_eq!(head_commit(&git_bin(), &repo).unwrap(), first);
        assert_eq!(
            checkout_commit(&git_bin(), &repo, &first).unwrap(),
            "Already on this commit."
        );
        assert!(checkout_commit(&git_bin(), &repo, "deadbee").is_err());

        let created = create_and_checkout_branch(&git_bin(), &repo, "from-first", &first).unwrap();
        assert!(created.contains("from-first"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "from-first");
        assert_eq!(head_commit(&git_bin(), &repo).unwrap(), first);

        git(&repo, &["checkout", "develop"]);
        git(&repo, &["checkout", "-b", "side"]);
        fs::write(repo.join("extra.txt"), "extra\n").unwrap();
        git(&repo, &["add", "extra.txt"]);
        git(&repo, &["commit", "-m", "extra"]);
        let extra = head_commit(&git_bin(), &repo).unwrap();
        git(&repo, &["checkout", "develop"]);
        let picked = cherry_pick_commits(&git_bin(), &repo, &[extra]).unwrap();
        assert!(picked.to_lowercase().contains("cherry") || picked.contains("extra"));
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "develop");
        assert_eq!(fs::read_to_string(repo.join("extra.txt")).unwrap(), "extra\n");

        fs::write(repo.join("README.md"), "third\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "third"]);
        let third = head_commit(&git_bin(), &repo).unwrap();
        let reverted = revert_commits(&git_bin(), &repo, &[third]).unwrap();
        assert!(reverted.to_lowercase().contains("revert") || reverted.contains("third"));
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "second\n");

        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("one.txt"), "one\n").unwrap();
        git(&repo, &["add", "one.txt"]);
        git(&repo, &["commit", "-m", "one"]);
        let one = head_commit(&git_bin(), &repo).unwrap();
        fs::write(repo.join("two.txt"), "two\n").unwrap();
        git(&repo, &["add", "two.txt"]);
        git(&repo, &["commit", "-m", "two"]);
        let two = head_commit(&git_bin(), &repo).unwrap();
        git(&repo, &["checkout", "develop"]);
        cherry_pick_commits(&git_bin(), &repo, &[one, two]).unwrap();
        assert_eq!(fs::read_to_string(repo.join("one.txt")).unwrap(), "one\n");
        assert_eq!(fs::read_to_string(repo.join("two.txt")).unwrap(), "two\n");

        assert!(cherry_pick_commits(&git_bin(), &repo, &[]).is_err());
        assert!(revert_commits(&git_bin(), &repo, &[]).is_err());
        assert!(cherry_pick_commits(&git_bin(), &repo, &["missing1".into()]).is_err());
    }

    #[test]
    fn repo_remote_browse_url_reads_origin() {
        let repo = init_repo();
        let err = repo_remote_browse_url(&git_bin(), &repo).unwrap_err();
        assert!(err.contains("origin"));
        git(
            &repo,
            &["remote", "add", "origin", "git@github.com:owner/repo.git"],
        );
        assert_eq!(
            repo_remote_browse_url(&git_bin(), &repo).unwrap(),
            "https://github.com/owner/repo"
        );
        let hash = head_commit(&git_bin(), &repo).unwrap();
        assert_eq!(
            commit_remote_url(&git_bin(), &repo, &hash).unwrap(),
            format!("https://github.com/owner/repo/commit/{hash}")
        );
    }

    #[test]
    fn open_repo_in_finder_rejects_missing_path() {
        let err = open_repo_in_finder(Path::new("/definitely/missing/shipyard-test")).unwrap_err();
        assert!(err.contains("missing"));
    }

    fn conflicted_merge_repo() -> PathBuf {
        let repo = init_repo();
        git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("README.md"), "feature\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "feature change"]);
        git(&repo, &["checkout", "develop"]);
        fs::write(repo.join("README.md"), "develop\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "develop change"]);
        let merge = run_git(&git_bin(), &repo, &["merge", "feature"]).unwrap();
        assert!(!merge.success);
        repo
    }

    #[test]
    fn reports_merge_conflicts_and_lists_them_once() {
        let repo = conflicted_merge_repo();
        let status = live_status(&git_bin(), &repo).unwrap();
        assert_eq!(status.operation, "merge");
        assert_eq!(status.conflicted_files, 1);
        assert!(status.dirty);
        let files = working_tree(&git_bin(), &repo).unwrap();
        let conflicted: Vec<_> = files
            .iter()
            .filter(|file| file.status == "Conflicted")
            .collect();
        assert_eq!(conflicted.len(), 1);
        assert_eq!(conflicted[0].path, "README.md");
        assert!(!conflicted[0].staged);
        assert!(discard_all_changes(&git_bin(), &repo).is_err());
        assert!(discard_file_changes(&git_bin(), &repo, "README.md", false).is_err());
    }

    #[test]
    fn discards_changes_for_one_file() {
        let repo = init_repo();
        fs::write(repo.join("README.md"), "changed\n").unwrap();
        fs::write(repo.join("other.txt"), "other\n").unwrap();
        discard_file_changes(&git_bin(), &repo, "README.md", false).unwrap();
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "hello\n");
        assert_eq!(fs::read_to_string(repo.join("other.txt")).unwrap(), "other\n");

        discard_file_changes(&git_bin(), &repo, "other.txt", false).unwrap();
        assert!(!repo.join("other.txt").exists());

        fs::remove_file(repo.join("README.md")).unwrap();
        discard_file_changes(&git_bin(), &repo, "README.md", false).unwrap();
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "hello\n");

        fs::write(repo.join("README.md"), "staged\n").unwrap();
        git(&repo, &["add", "README.md"]);
        discard_file_changes(&git_bin(), &repo, "README.md", true).unwrap();
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "hello\n");
        assert!(working_tree(&git_bin(), &repo).unwrap().is_empty());

        fs::write(repo.join("new.txt"), "new\n").unwrap();
        git(&repo, &["add", "new.txt"]);
        discard_file_changes(&git_bin(), &repo, "new.txt", true).unwrap();
        assert!(!repo.join("new.txt").exists());

        fs::write(repo.join("README.md"), "staged\n").unwrap();
        git(&repo, &["add", "README.md"]);
        fs::write(repo.join("README.md"), "staged\nunstaged\n").unwrap();
        discard_file_changes(&git_bin(), &repo, "README.md", false).unwrap();
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "staged\n");
        let staged_only = working_tree(&git_bin(), &repo).unwrap();
        assert!(staged_only
            .iter()
            .any(|file| file.path == "README.md" && file.staged));
        assert!(!staged_only
            .iter()
            .any(|file| file.path == "README.md" && !file.staged));

        fs::write(repo.join("README.md"), "staged\nunstaged\n").unwrap();
        let err = discard_file_changes(&git_bin(), &repo, "README.md", true).unwrap_err();
        assert!(err.to_lowercase().contains("unstaged"));
        assert_eq!(
            fs::read_to_string(repo.join("README.md")).unwrap(),
            "staged\nunstaged\n"
        );

        fs::write(repo.join("README.md"), "staged\n").unwrap();
        git(&repo, &["add", "README.md"]);
        fs::remove_file(repo.join("README.md")).unwrap();
        discard_file_changes(&git_bin(), &repo, "README.md", true).unwrap();
        assert!(!repo.join("README.md").exists());
        assert!(working_tree(&git_bin(), &repo)
            .unwrap()
            .iter()
            .any(|file| file.path == "README.md" && !file.staged && file.status == "Deleted"));

        git(&repo, &["reset", "--hard"]);
        git(&repo, &["clean", "-fd"]);
        git(&repo, &["mv", "README.md", "guide.md"]);
        discard_file_changes(&git_bin(), &repo, "guide.md", true).unwrap();
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "hello\n");
        assert!(!repo.join("guide.md").exists());
        assert!(working_tree(&git_bin(), &repo).unwrap().is_empty());

        git(&repo, &["mv", "README.md", "my file.md"]);
        discard_file_changes(&git_bin(), &repo, "my file.md", true).unwrap();
        assert!(repo.join("README.md").is_file());
        assert!(!repo.join("my file.md").exists());

        git(&repo, &["reset", "--hard"]);
        fs::write(repo.join("README.md"), "hello\nstaged\n").unwrap();
        git(&repo, &["add", "README.md"]);
        fs::write(repo.join("README.md"), "hello\nSTAGED\n").unwrap();
        let before = fs::read(repo.join("README.md")).unwrap();
        let err = discard_file_changes(&git_bin(), &repo, "README.md", true).unwrap_err();
        assert!(err.to_lowercase().contains("unstaged"));
        assert_eq!(fs::read(repo.join("README.md")).unwrap(), before);
        assert!(working_tree(&git_bin(), &repo)
            .unwrap()
            .iter()
            .any(|file| file.path == "README.md" && file.staged));

        assert!(discard_file_changes(&git_bin(), &repo, "../README.md", false).is_err());
    }

    #[test]
    fn discard_refuses_to_lose_unstaged_adds_and_renames() {
        let repo = init_repo();
        fs::write(repo.join("new.txt"), "new\n").unwrap();
        git(&repo, &["add", "new.txt"]);
        fs::write(repo.join("new.txt"), "new\nextra\n").unwrap();
        let err = discard_file_changes(&git_bin(), &repo, "new.txt", true).unwrap_err();
        assert!(err.to_lowercase().contains("unstaged"));
        assert_eq!(fs::read_to_string(repo.join("new.txt")).unwrap(), "new\nextra\n");
        assert!(working_tree(&git_bin(), &repo)
            .unwrap()
            .iter()
            .any(|file| file.path == "new.txt" && file.staged));

        git(&repo, &["reset", "--hard"]);
        git(&repo, &["clean", "-fd"]);
        git(&repo, &["mv", "README.md", "guide.md"]);
        fs::write(repo.join("guide.md"), "hello\nextra\n").unwrap();
        let err = discard_file_changes(&git_bin(), &repo, "guide.md", true).unwrap_err();
        assert!(err.to_lowercase().contains("unstaged"));
        assert_eq!(
            fs::read_to_string(repo.join("guide.md")).unwrap(),
            "hello\nextra\n"
        );
        assert!(!repo.join("README.md").exists());
        assert!(working_tree(&git_bin(), &repo)
            .unwrap()
            .iter()
            .any(|file| file.path == "guide.md" && file.staged));

        fs::write(repo.join("guide.md"), "hello\n").unwrap();
        fs::write(repo.join("README.md"), "custom\n").unwrap();
        let guide = fs::read(repo.join("guide.md")).unwrap();
        let err = discard_file_changes(&git_bin(), &repo, "guide.md", true).unwrap_err();
        assert!(err.contains("README.md"));
        assert_eq!(fs::read(repo.join("guide.md")).unwrap(), guide);
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "custom\n");
        assert!(working_tree(&git_bin(), &repo)
            .unwrap()
            .iter()
            .any(|file| file.path == "guide.md" && file.staged));
    }

    #[test]
    fn discard_staged_keeps_unstaged_edits_in_place() {
        let repo = init_repo();
        fs::write(repo.join("list.txt"), "a\nb\nc\nd\ne\nf\ng\nh\n").unwrap();
        git(&repo, &["add", "list.txt"]);
        git(&repo, &["commit", "-qm", "list"]);

        fs::write(repo.join("list.txt"), "a\nb\nc\nd\nf\ng\nh\n").unwrap();
        git(&repo, &["add", "list.txt"]);
        fs::write(repo.join("list.txt"), "X\nY\nZ\na\nb\nc\nd\nf\ng\nh\n").unwrap();
        discard_file_changes(&git_bin(), &repo, "list.txt", true).unwrap();
        assert_eq!(
            fs::read_to_string(repo.join("list.txt")).unwrap(),
            "X\nY\nZ\na\nb\nc\nd\ne\nf\ng\nh\n"
        );
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(!files.iter().any(|file| file.staged));
        assert!(files
            .iter()
            .any(|file| file.path == "list.txt" && !file.staged));

        git(&repo, &["checkout", "--", "list.txt"]);
        fs::write(repo.join("list.txt"), "A\nb\nc\nd\ne\nf\ng\nh\n").unwrap();
        git(&repo, &["add", "list.txt"]);
        fs::write(repo.join("list.txt"), "A\nb\nc\nd\ne\nf\ng\nH\n").unwrap();
        discard_file_changes(&git_bin(), &repo, "list.txt", true).unwrap();
        assert_eq!(
            fs::read_to_string(repo.join("list.txt")).unwrap(),
            "a\nb\nc\nd\ne\nf\ng\nH\n"
        );

        git(&repo, &["checkout", "--", "list.txt"]);
        fs::write(repo.join("list.txt"), "A\nb\nc\nd\ne\nf\ng\nh\n").unwrap();
        git(&repo, &["add", "list.txt"]);
        fs::write(repo.join("list.txt"), "A\nB\nc\nd\ne\nf\ng\nh\n").unwrap();
        let err = discard_file_changes(&git_bin(), &repo, "list.txt", true).unwrap_err();
        assert!(err.to_lowercase().contains("unstaged"));
        assert_eq!(
            fs::read_to_string(repo.join("list.txt")).unwrap(),
            "A\nB\nc\nd\ne\nf\ng\nh\n"
        );
        assert!(working_tree(&git_bin(), &repo)
            .unwrap()
            .iter()
            .any(|file| file.path == "list.txt" && file.staged));
    }

    #[test]
    fn marks_conflict_resolved_and_continues_merge() {
        let repo = conflicted_merge_repo();
        fs::write(repo.join("README.md"), "resolved\n").unwrap();
        stage_file(&git_bin(), &repo, "README.md").unwrap();
        let files = working_tree(&git_bin(), &repo).unwrap();
        assert!(!files.iter().any(|file| file.status == "Conflicted"));
        assert!(files.iter().any(|file| file.path == "README.md" && file.staged));
        let message = continue_operation(&git_bin(), &repo).unwrap();
        assert!(message.to_lowercase().contains("merge") || message.to_lowercase().contains("commit"));
        let status = live_status(&git_bin(), &repo).unwrap();
        assert!(status.operation.is_empty());
        assert_eq!(status.conflicted_files, 0);
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "resolved\n");
    }

    #[test]
    fn aborts_merge_and_rebase_conflicts() {
        let repo = conflicted_merge_repo();
        let message = abort_operation(&git_bin(), &repo).unwrap();
        assert!(message.to_lowercase().contains("abort"));
        let status = live_status(&git_bin(), &repo).unwrap();
        assert!(status.operation.is_empty());
        assert_eq!(status.conflicted_files, 0);
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "develop\n");

        git(&repo, &["checkout", "-b", "rebased"]);
        fs::write(repo.join("README.md"), "rebased\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "rebased change"]);
        git(&repo, &["checkout", "develop"]);
        fs::write(repo.join("README.md"), "develop again\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "develop again"]);
        git(&repo, &["checkout", "rebased"]);
        let rebase = run_git(&git_bin(), &repo, &["rebase", "develop"]).unwrap();
        assert!(!rebase.success);
        let status = live_status(&git_bin(), &repo).unwrap();
        assert_eq!(status.operation, "rebase");
        assert!(status.conflicted_files >= 1);
        let message = abort_operation(&git_bin(), &repo).unwrap();
        assert!(message.to_lowercase().contains("abort"));
        let status = live_status(&git_bin(), &repo).unwrap();
        assert_eq!(status.operation, "");
        assert_eq!(current_branch(&git_bin(), &repo).unwrap(), "rebased");
        assert_eq!(fs::read_to_string(repo.join("README.md")).unwrap(), "rebased\n");
    }
}
