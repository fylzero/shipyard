use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

pub const REPO_FILES_CHANGED_EVENT: &str = "repo-files-changed";
pub const REPO_GIT_CHANGED_EVENT: &str = "repo-git-changed";

/// Collapse a burst of writes into one refresh.
///
/// macOS FSEvents is created with `kFSEventStreamCreateFlagNoDefer` and a
/// latency of 0 inside notify 8.2, and neither is configurable. Kernel
/// coalescing cannot be turned on from here. This debounce is the coalesce.
/// A longer wait would only postpone the refresh.
const DEBOUNCE: Duration = Duration::from_millis(200);

const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
];

const IGNORED_GIT_DIRS: &[&str] = &["objects", "logs", "hooks"];

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoFilesChanged {
    pub path: String,
    pub git: bool,
}

/// `Files` watches the whole worktree for an open repo tab. `Git` watches
/// only git metadata, which is all the dashboard shows and cheap enough to
/// keep on for every listed repo.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum WatchScope {
    Files,
    Git,
}

impl WatchScope {
    fn event(self) -> &'static str {
        match self {
            WatchScope::Files => REPO_FILES_CHANGED_EVENT,
            WatchScope::Git => REPO_GIT_CHANGED_EVENT,
        }
    }
}

struct WatchEntry {
    _debouncer: Debouncer<RecommendedWatcher>,
    refs: usize,
}

#[derive(Default)]
pub struct RepoWatcherState {
    watches: Mutex<HashMap<(String, WatchScope), WatchEntry>>,
}

/// Where a repo keeps its files and metadata, with symlinks resolved.
///
/// FSEvents reports real paths (`/private/tmp/...` for `/tmp/...`), so
/// comparing against the path as the user added it drops every event.
/// A linked worktree has a `.git` file pointing at its own git dir, and
/// shares refs with the main repo through `commondir`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoLayout {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
}

fn resolve(path: PathBuf) -> PathBuf {
    fs::canonicalize(&path).unwrap_or(path)
}

fn read_pointer(file: &Path, prefix: &str) -> Option<String> {
    let text = fs::read_to_string(file).ok()?;
    let line = text.lines().next()?.trim();
    let value = line.strip_prefix(prefix).unwrap_or(line).trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub fn resolve_layout(root: &Path) -> RepoLayout {
    let root = resolve(root.to_path_buf());
    let dot_git = root.join(".git");
    let git_dir = if dot_git.is_file() {
        read_pointer(&dot_git, "gitdir:")
            .map(|target| resolve(root.join(target)))
            .unwrap_or(dot_git)
    } else {
        resolve(dot_git)
    };
    let common_dir = read_pointer(&git_dir.join("commondir"), "")
        .map(|target| resolve(git_dir.join(target)))
        .unwrap_or_else(|| git_dir.clone());
    RepoLayout {
        root,
        git_dir,
        common_dir,
    }
}

impl RepoLayout {
    /// Directories to hand the OS watcher, without nesting one inside another.
    fn watch_roots(&self, scope: WatchScope) -> Vec<PathBuf> {
        let candidates = match scope {
            WatchScope::Files => vec![&self.root, &self.git_dir, &self.common_dir],
            WatchScope::Git => vec![&self.git_dir, &self.common_dir],
        };
        let mut roots: Vec<PathBuf> = Vec::new();
        for dir in candidates {
            if !dir.is_dir() || roots.iter().any(|root| dir.starts_with(root)) {
                continue;
            }
            roots.retain(|root| !root.starts_with(dir));
            roots.push(dir.clone());
        }
        roots
    }
}

fn watch_key(path: &str) -> String {
    path.trim().trim_end_matches('/').to_string()
}

fn path_file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

fn is_editor_junk(name: &str) -> bool {
    name == ".DS_Store" || name.ends_with('~') || name.ends_with(".swp") || name.ends_with(".swo")
}

fn is_git_lock(name: &str) -> bool {
    name.ends_with(".lock")
}

/// `HEAD`, `refs/`, `packed-refs`, and `reftable/` move history. The index
/// and fetch bookkeeping only change the working tree, and reloading the
/// graph for those was the multi-second stall after stage and unstage.
fn tracks_history(git_entry: &str) -> bool {
    matches!(git_entry, "HEAD" | "packed-refs" | "refs" | "reftable")
}

/// Classify a path relative to a git dir, same contract as `classify_change`.
fn classify_git_path(relative: &Path) -> Option<bool> {
    let mut components = relative.components().peekable();
    let Some(Component::Normal(first)) = components.next() else {
        return Some(false);
    };
    let first = first.to_str()?;
    if IGNORED_GIT_DIRS.contains(&first) {
        return None;
    }
    if is_git_lock(first) && components.peek().is_none() {
        return None;
    }
    if let Some(name) = path_file_name(relative) {
        if is_git_lock(name) || is_editor_junk(name) {
            return None;
        }
    }
    Some(tracks_history(first))
}

/// Classify a filesystem path relative to a repo.
/// `Some(true)` is history metadata, `Some(false)` is a worktree or index
/// change, `None` is noise.
pub fn classify_change(layout: &RepoLayout, changed: &Path) -> Option<bool> {
    if let Ok(relative) = changed.strip_prefix(&layout.git_dir) {
        return classify_git_path(relative);
    }
    if let Ok(relative) = changed.strip_prefix(&layout.common_dir) {
        // The main repo's index and other worktrees live here too; only
        // shared refs belong to this worktree.
        return classify_git_path(relative).filter(|history| *history);
    }

    let relative = changed.strip_prefix(&layout.root).ok()?;
    let mut components = relative.components();
    let first = match components.next() {
        None => return Some(false),
        Some(Component::Normal(name)) => name.to_str()?,
        Some(_) => return Some(false),
    };
    if first == ".git" {
        return Some(false);
    }
    if IGNORED_DIRS.contains(&first) {
        return None;
    }
    for component in components {
        if let Component::Normal(name) = component {
            if let Some(name) = name.to_str() {
                if IGNORED_DIRS.contains(&name) {
                    return None;
                }
            }
        }
    }
    if let Some(name) = path_file_name(relative) {
        if is_editor_junk(name) {
            return None;
        }
    }
    Some(false)
}

fn emit_changes(
    app: &AppHandle,
    repo_path: &str,
    layout: &RepoLayout,
    scope: WatchScope,
    events: DebounceEventResult,
) {
    let Ok(events) = events else {
        return;
    };
    let mut any = false;
    let mut git = false;
    for event in events {
        if let Some(is_git) = classify_change(layout, &event.path) {
            any = true;
            git |= is_git;
        }
    }
    if any {
        let _ = app.emit(
            scope.event(),
            RepoFilesChanged {
                path: repo_path.to_string(),
                git,
            },
        );
    }
}

fn start_watch(
    app: AppHandle,
    state: State<RepoWatcherState>,
    path: String,
    scope: WatchScope,
) -> Result<(), String> {
    let key = watch_key(&path);
    if key.is_empty() {
        return Err("Repository path cannot be empty".into());
    }
    if !Path::new(&key).is_dir() {
        return Err("Repository path is not a directory".into());
    }

    let mut watches = state
        .watches
        .lock()
        .map_err(|_| "Could not lock file watchers.".to_string())?;
    if let Some(entry) = watches.get_mut(&(key.clone(), scope)) {
        entry.refs += 1;
        return Ok(());
    }

    let layout = resolve_layout(Path::new(&key));
    let roots = layout.watch_roots(scope);
    if roots.is_empty() {
        return Err("Repository has no git directory to watch".into());
    }

    let watched = key.clone();
    let handle = app.clone();
    let classify_layout = layout.clone();
    let mut debouncer = new_debouncer(DEBOUNCE, move |result: DebounceEventResult| {
        emit_changes(&handle, &watched, &classify_layout, scope, result);
    })
    .map_err(|err| format!("Could not watch repository: {err}"))?;

    for root in &roots {
        debouncer
            .watcher()
            .watch(root, RecursiveMode::Recursive)
            .map_err(|err| format!("Could not watch repository: {err}"))?;
    }

    watches.insert(
        (key, scope),
        WatchEntry {
            _debouncer: debouncer,
            refs: 1,
        },
    );
    Ok(())
}

fn stop_watch(
    state: State<RepoWatcherState>,
    path: String,
    scope: WatchScope,
) -> Result<(), String> {
    let key = watch_key(&path);
    if key.is_empty() {
        return Ok(());
    }
    let mut watches = state
        .watches
        .lock()
        .map_err(|_| "Could not lock file watchers.".to_string())?;
    let entry_key = (key, scope);
    let Some(entry) = watches.get_mut(&entry_key) else {
        return Ok(());
    };
    entry.refs = entry.refs.saturating_sub(1);
    if entry.refs == 0 {
        watches.remove(&entry_key);
    }
    Ok(())
}

#[tauri::command]
pub fn watch_repo(
    app: AppHandle,
    state: State<RepoWatcherState>,
    path: String,
) -> Result<(), String> {
    start_watch(app, state, path, WatchScope::Files)
}

#[tauri::command]
pub fn unwatch_repo(state: State<RepoWatcherState>, path: String) -> Result<(), String> {
    stop_watch(state, path, WatchScope::Files)
}

#[tauri::command]
pub fn watch_repo_git(
    app: AppHandle,
    state: State<RepoWatcherState>,
    path: String,
) -> Result<(), String> {
    start_watch(app, state, path, WatchScope::Git)
}

#[tauri::command]
pub fn unwatch_repo_git(state: State<RepoWatcherState>, path: String) -> Result<(), String> {
    stop_watch(state, path, WatchScope::Git)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn plain(root: &str) -> RepoLayout {
        RepoLayout {
            root: PathBuf::from(root),
            git_dir: PathBuf::from(root).join(".git"),
            common_dir: PathBuf::from(root).join(".git"),
        }
    }

    fn linked() -> RepoLayout {
        RepoLayout {
            root: PathBuf::from("/wt"),
            git_dir: PathBuf::from("/main/.git/worktrees/wt"),
            common_dir: PathBuf::from("/main/.git"),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!("shipyard-watch-{name}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn classifies_worktree_and_git_paths() {
        let repo = plain("/repo");
        assert_eq!(classify_change(&repo, Path::new("/repo")), Some(false));
        assert_eq!(
            classify_change(&repo, Path::new("/repo/src/App.vue")),
            Some(false)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/index")),
            Some(false)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/FETCH_HEAD")),
            Some(false)
        );
        assert_eq!(classify_change(&repo, Path::new("/repo/.git")), Some(false));
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/HEAD")),
            Some(true)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/packed-refs")),
            Some(true)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/refs/heads/main")),
            Some(true)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/refs/tags/v1")),
            Some(true)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/reftable/tables.list")),
            Some(true)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/Cargo.lock")),
            Some(false)
        );
    }

    #[test]
    fn ignores_noisy_paths() {
        let repo = plain("/repo");
        assert_eq!(
            classify_change(&repo, Path::new("/repo/node_modules/foo/index.js")),
            None
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/objects/pack/foo.pack")),
            None
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/logs/HEAD")),
            None
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/index.lock")),
            None
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/.git/refs/tags/v1.lock")),
            None
        );
        assert_eq!(
            classify_change(&repo, Path::new("/repo/src/.DS_Store")),
            None
        );
        assert_eq!(
            classify_change(&repo, Path::new("/tmp/other/src/App.vue")),
            None
        );
    }

    #[test]
    fn classifies_linked_worktree_paths() {
        let repo = linked();
        assert_eq!(
            classify_change(&repo, Path::new("/main/.git/worktrees/wt/HEAD")),
            Some(true)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/main/.git/worktrees/wt/index")),
            Some(false)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/main/.git/refs/tags/v1")),
            Some(true)
        );
        assert_eq!(
            classify_change(&repo, Path::new("/main/.git/packed-refs")),
            Some(true)
        );
        assert_eq!(classify_change(&repo, Path::new("/main/.git/index")), None);
        assert_eq!(
            classify_change(&repo, Path::new("/main/.git/worktrees/other/HEAD")),
            None
        );
        assert_eq!(classify_change(&repo, Path::new("/wt/.git")), Some(false));
        assert_eq!(
            classify_change(&repo, Path::new("/wt/src/a.rs")),
            Some(false)
        );
        assert_eq!(classify_change(&repo, Path::new("/main/src/a.rs")), None);
    }

    #[test]
    fn watch_roots_do_not_nest() {
        let dir = scratch("roots");
        let main = dir.join("main");
        let wt = dir.join("wt");
        fs::create_dir_all(main.join(".git/worktrees/wt")).unwrap();
        fs::create_dir_all(&wt).unwrap();
        let layout = RepoLayout {
            root: wt.clone(),
            git_dir: main.join(".git/worktrees/wt"),
            common_dir: main.join(".git"),
        };
        assert_eq!(
            layout.watch_roots(WatchScope::Files),
            vec![wt.clone(), main.join(".git")]
        );
        assert_eq!(layout.watch_roots(WatchScope::Git), vec![main.join(".git")]);

        let plain = RepoLayout {
            root: main.clone(),
            git_dir: main.join(".git"),
            common_dir: main.join(".git"),
        };
        assert_eq!(plain.watch_roots(WatchScope::Files), vec![main.clone()]);
        assert_eq!(plain.watch_roots(WatchScope::Git), vec![main.join(".git")]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolves_symlinks_and_linked_worktrees() {
        let dir = scratch("layout");
        let real = resolve(dir.clone());
        let main = real.join("main");
        let wt = real.join("wt");
        fs::create_dir_all(main.join(".git/worktrees/wt")).unwrap();
        fs::create_dir_all(&wt).unwrap();
        fs::write(wt.join(".git"), "gitdir: ../main/.git/worktrees/wt\n").unwrap();
        fs::write(main.join(".git/worktrees/wt/commondir"), "../..\n").unwrap();

        assert_eq!(
            resolve_layout(&main),
            RepoLayout {
                root: main.clone(),
                git_dir: main.join(".git"),
                common_dir: main.join(".git"),
            }
        );
        assert_eq!(
            resolve_layout(&wt),
            RepoLayout {
                root: wt.clone(),
                git_dir: main.join(".git/worktrees/wt"),
                common_dir: main.join(".git"),
            }
        );

        #[cfg(unix)]
        {
            let link = real.join("link");
            std::os::unix::fs::symlink(&main, &link).unwrap();
            assert_eq!(resolve_layout(&link).root, main);
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
