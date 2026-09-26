use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::git;
use crate::models::{
    sanitize_editor, sanitize_font_family, sanitize_font_size, sanitize_refresh_active_hours,
    AppData, BranchOverview, BranchTracking, CommitFile, CommitNode, DeleteMergedResult, FileBlame,
    GitConfig, LastCommit, RefreshActiveHours, RemoteEntry, RemoteOverview, RepoActionResult,
    RepoEntry, RepoFile, RepoGroup, RepoStatus, StashEntry, TagEntry, WorkingTreeFile,
};
use crate::persist;

pub struct AppState {
    pub data: Mutex<AppData>,
    pub git: Option<PathBuf>,
}

fn require_git(state: &AppState) -> Result<PathBuf, String> {
    state.git.clone().ok_or_else(|| {
        "Git was not found. Install Git and make sure it is available on your PATH.".into()
    })
}

fn find_group_mut<'a>(data: &'a mut AppData, group_id: &str) -> Result<&'a mut RepoGroup, String> {
    data.groups
        .iter_mut()
        .find(|group| group.id == group_id)
        .ok_or_else(|| "Group not found".to_string())
}

fn persist_data(app: &AppHandle, data: &AppData) -> Result<(), String> {
    persist::save(app, data)
}

const STANDALONE_GROUP_ID: &str = "standalone";

fn path_already_added(data: &AppData, root: &str) -> bool {
    data.repos.iter().any(|repo| repo.path == root)
        || data
            .groups
            .iter()
            .any(|group| group.repos.iter().any(|repo| repo.path == root))
}

fn sanitize_color(color: &str) -> Option<String> {
    let color = color.trim().to_string();
    if color.starts_with('#') && (color.len() == 7 || color.len() == 4) {
        Some(color)
    } else {
        None
    }
}

fn sanitize_repos(repos: &mut Vec<RepoEntry>) -> Result<(), String> {
    for repo in repos {
        if repo.id.trim().is_empty() {
            repo.id = uuid::Uuid::new_v4().to_string();
        }
        repo.path = repo.path.trim().to_string();
        if repo.path.is_empty() {
            return Err("Repository path cannot be empty".into());
        }
        repo.label = repo.label.trim().to_string();
        repo.header_color = sanitize_color(&repo.header_color).unwrap_or_default();
    }
    Ok(())
}

fn find_repo_entry(data: &AppData, group_id: &str, repo_id: &str) -> Result<RepoEntry, String> {
    if group_id == STANDALONE_GROUP_ID {
        return data
            .repos
            .iter()
            .find(|entry| entry.id == repo_id)
            .cloned()
            .ok_or_else(|| "Repository not found".to_string());
    }
    let group = data
        .groups
        .iter()
        .find(|group| group.id == group_id)
        .ok_or_else(|| "Group not found".to_string())?;
    group
        .repos
        .iter()
        .find(|entry| entry.id == repo_id)
        .cloned()
        .ok_or_else(|| "Repository not found".to_string())
}

fn repo_entry(state: &AppState, group_id: &str, repo_id: &str) -> Result<(PathBuf, RepoEntry), String> {
    let git = require_git(state)?;
    let data = state.data.lock().map_err(|err| err.to_string())?;
    let repo = find_repo_entry(&data, group_id, repo_id)?;
    Ok((git, repo))
}

fn repo_list(state: &AppState, group_id: &str) -> Result<(PathBuf, RepoGroup), String> {
    let git = require_git(state)?;
    let data = state.data.lock().map_err(|err| err.to_string())?;
    let group = data
        .groups
        .iter()
        .find(|group| group.id == group_id)
        .cloned()
        .ok_or_else(|| "Group not found".to_string())?;
    Ok((git, group))
}

#[tauri::command]
pub fn get_state(state: State<AppState>) -> Result<AppData, String> {
    let data = state.data.lock().map_err(|err| err.to_string())?;
    Ok(data.clone())
}

#[tauri::command]
pub fn create_group(
    app: AppHandle,
    state: State<AppState>,
    name: String,
) -> Result<RepoGroup, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Group name is required".into());
    }

    let group = RepoGroup {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        expanded: true,
        pull_from_branch: "develop".into(),
        checkout_fallbacks: vec!["develop".into()],
        header_color: "#16323c".into(),
        repos: Vec::new(),
    };

    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.groups.insert(0, group.clone());
    persist_data(&app, &data)?;
    Ok(group)
}

#[tauri::command]
pub fn rename_group(
    app: AppHandle,
    state: State<AppState>,
    group_id: String,
    name: String,
) -> Result<(), String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Group name is required".into());
    }
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    find_group_mut(&mut data, &group_id)?.name = name;
    persist_data(&app, &data)
}

#[tauri::command]
pub fn delete_group(app: AppHandle, state: State<AppState>, group_id: String) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let before = data.groups.len();
    data.groups.retain(|group| group.id != group_id);
    if data.groups.len() == before {
        return Err("Group not found".into());
    }
    persist_data(&app, &data)
}

#[tauri::command]
pub fn toggle_group(app: AppHandle, state: State<AppState>, group_id: String) -> Result<bool, String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let group = find_group_mut(&mut data, &group_id)?;
    group.expanded = !group.expanded;
    let expanded = group.expanded;
    persist_data(&app, &data)?;
    Ok(expanded)
}

#[tauri::command]
pub fn set_all_groups_expanded(
    app: AppHandle,
    state: State<AppState>,
    expanded: bool,
) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    for group in &mut data.groups {
        group.expanded = expanded;
    }
    persist_data(&app, &data)
}

#[tauri::command]
pub fn update_group_settings(
    app: AppHandle,
    state: State<AppState>,
    group_id: String,
    pull_from_branch: String,
    checkout_fallbacks: Vec<String>,
    header_color: Option<String>,
) -> Result<(), String> {
    let pull_from_branch = pull_from_branch.trim().to_string();
    if !pull_from_branch.is_empty() {
        git::validate_ref(&pull_from_branch)?;
    }
    let fallbacks: Vec<String> = checkout_fallbacks
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    for fallback in &fallbacks {
        git::validate_ref(fallback)?;
    }

    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let group = find_group_mut(&mut data, &group_id)?;
    group.pull_from_branch = if pull_from_branch.is_empty() {
        "develop".into()
    } else {
        pull_from_branch
    };
    group.checkout_fallbacks = if fallbacks.is_empty() {
        vec!["develop".into()]
    } else {
        fallbacks
    };
    if let Some(color) = header_color {
        if let Some(color) = sanitize_color(&color) {
            group.header_color = color;
        }
    }
    persist_data(&app, &data)
}

#[tauri::command]
pub fn update_app_settings(
    app: AppHandle,
    state: State<AppState>,
    refresh_interval_seconds: u64,
) -> Result<u64, String> {
    let seconds = if refresh_interval_seconds == 0 {
        0
    } else {
        refresh_interval_seconds.clamp(30, 86_400)
    };
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.refresh_interval_seconds = seconds;
    persist_data(&app, &data)?;
    Ok(seconds)
}

#[tauri::command]
pub fn update_files_pane_width(
    app: AppHandle,
    state: State<AppState>,
    width: u32,
) -> Result<u32, String> {
    let width = width.clamp(220, 800);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.files_pane_width = width;
    persist_data(&app, &data)?;
    Ok(width)
}

#[tauri::command]
pub fn update_terminal_pane_height(
    app: AppHandle,
    state: State<AppState>,
    height: u32,
) -> Result<u32, String> {
    let height = height.clamp(160, 720);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.terminal_pane_height = height;
    persist_data(&app, &data)?;
    Ok(height)
}

#[tauri::command]
pub fn update_diff_mode(
    app: AppHandle,
    state: State<AppState>,
    mode: String,
) -> Result<String, String> {
    let mode = sanitize_diff_mode(&mode)?;
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.diff_mode = mode.clone();
    persist_data(&app, &data)?;
    Ok(mode)
}

#[tauri::command]
pub fn update_diff_font_family(
    app: AppHandle,
    state: State<AppState>,
    font_family: String,
) -> Result<String, String> {
    let font_family = sanitize_font_family(&font_family);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.diff_font_family = font_family.clone();
    persist_data(&app, &data)?;
    Ok(font_family)
}

#[tauri::command]
pub fn update_diff_font_size(
    app: AppHandle,
    state: State<AppState>,
    font_size: f64,
) -> Result<f64, String> {
    let font_size = sanitize_font_size(font_size, 13.0);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.diff_font_size = font_size;
    persist_data(&app, &data)?;
    Ok(font_size)
}

#[tauri::command]
pub fn update_terminal_font_family(
    app: AppHandle,
    state: State<AppState>,
    font_family: String,
) -> Result<String, String> {
    let font_family = sanitize_font_family(&font_family);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.terminal_font_family = font_family.clone();
    persist_data(&app, &data)?;
    Ok(font_family)
}

#[tauri::command]
pub fn update_terminal_font_size(
    app: AppHandle,
    state: State<AppState>,
    font_size: f64,
) -> Result<f64, String> {
    let font_size = sanitize_font_size(font_size, 14.0);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.terminal_font_size = font_size;
    persist_data(&app, &data)?;
    Ok(font_size)
}

#[tauri::command]
pub fn update_editor(
    app: AppHandle,
    state: State<AppState>,
    editor: String,
) -> Result<String, String> {
    let editor = sanitize_editor(&editor);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.editor = editor.clone();
    persist_data(&app, &data)?;
    Ok(editor)
}

#[tauri::command]
pub fn update_refresh_active_hours(
    app: AppHandle,
    state: State<AppState>,
    hours: RefreshActiveHours,
) -> Result<RefreshActiveHours, String> {
    let hours = sanitize_refresh_active_hours(hours);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.refresh_active_hours = hours.clone();
    persist_data(&app, &data)?;
    Ok(hours)
}

#[tauri::command]
pub fn replace_app_data(
    app: AppHandle,
    state: State<AppState>,
    data: AppData,
) -> Result<AppData, String> {
    let sanitized = sanitize_app_data(data)?;
    let mut lock = state.data.lock().map_err(|err| err.to_string())?;
    *lock = sanitized.clone();
    persist_data(&app, &lock)?;
    Ok(sanitized)
}

fn sanitize_diff_mode(mode: &str) -> Result<String, String> {
    match mode.trim() {
        "inline" | "split" => Ok(mode.trim().to_string()),
        _ => Err("diffMode must be \"inline\" or \"split\"".into()),
    }
}

fn sanitize_app_data(mut data: AppData) -> Result<AppData, String> {
    data.refresh_interval_seconds = if data.refresh_interval_seconds == 0 {
        0
    } else {
        data.refresh_interval_seconds.clamp(30, 86_400)
    };
    data.files_pane_width = data.files_pane_width.clamp(220, 800);
    data.terminal_pane_height = data.terminal_pane_height.clamp(160, 720);
    data.diff_mode = sanitize_diff_mode(&data.diff_mode)?;
    data.diff_font_family = sanitize_font_family(&data.diff_font_family);
    data.diff_font_size = sanitize_font_size(data.diff_font_size, 13.0);
    data.terminal_font_family = sanitize_font_family(&data.terminal_font_family);
    data.terminal_font_size = sanitize_font_size(data.terminal_font_size, 14.0);
    data.editor = sanitize_editor(&data.editor);
    data.refresh_active_hours = sanitize_refresh_active_hours(data.refresh_active_hours);
    if let Some(window) = &mut data.window {
        window.width = window.width.max(crate::models::MIN_WINDOW_WIDTH);
        window.height = window.height.max(crate::models::MIN_WINDOW_HEIGHT);
    }
    sanitize_repos(&mut data.repos)?;

    for group in &mut data.groups {
        if group.id.trim().is_empty() {
            group.id = uuid::Uuid::new_v4().to_string();
        }
        group.name = group.name.trim().to_string();
        if group.name.is_empty() {
            return Err("Every group needs a name".into());
        }
        let pull = group.pull_from_branch.trim().to_string();
        if !pull.is_empty() {
            git::validate_ref(&pull)?;
        }
        group.pull_from_branch = if pull.is_empty() {
            "develop".into()
        } else {
            pull
        };
        let fallbacks: Vec<String> = group
            .checkout_fallbacks
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        for fallback in &fallbacks {
            git::validate_ref(fallback)?;
        }
        group.checkout_fallbacks = if fallbacks.is_empty() {
            vec!["develop".into()]
        } else {
            fallbacks
        };
        group.header_color = sanitize_color(&group.header_color).unwrap_or_else(|| "#16323c".into());
        sanitize_repos(&mut group.repos)?;
    }
    Ok(data)
}

#[tauri::command]
pub fn add_repo(
    app: AppHandle,
    state: State<AppState>,
    group_id: String,
    path: String,
) -> Result<RepoEntry, String> {
    let git = require_git(&state)?;
    let root = git::repo_root(&git, Path::new(&path))?;

    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    if data.repos.iter().any(|repo| repo.path == root) {
        return Err("That repository is already added.".into());
    }
    let group = find_group_mut(&mut data, &group_id)?;
    if group.repos.iter().any(|repo| repo.path == root) {
        return Err("That repository is already in this group.".into());
    }

    let entry = RepoEntry {
        id: uuid::Uuid::new_v4().to_string(),
        path: root,
        label: String::new(),
        header_color: String::new(),
    };
    group.repos.push(entry.clone());
    persist_data(&app, &data)?;
    Ok(entry)
}

#[tauri::command]
pub fn remove_repo(
    app: AppHandle,
    state: State<AppState>,
    group_id: String,
    repo_id: String,
) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let group = find_group_mut(&mut data, &group_id)?;
    let before = group.repos.len();
    group.repos.retain(|repo| repo.id != repo_id);
    if group.repos.len() == before {
        return Err("Repository not found".into());
    }
    persist_data(&app, &data)
}

#[tauri::command]
pub fn reorder_group_repos(
    app: AppHandle,
    state: State<AppState>,
    group_id: String,
    repo_ids: Vec<String>,
) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let group = find_group_mut(&mut data, &group_id)?;
    if repo_ids.len() != group.repos.len() {
        return Err("Repository list does not match this group.".into());
    }
    let mut by_id: std::collections::HashMap<_, _> = group
        .repos
        .drain(..)
        .map(|repo| (repo.id.clone(), repo))
        .collect();
    let mut next = Vec::with_capacity(repo_ids.len());
    for id in repo_ids {
        let repo = by_id
            .remove(&id)
            .ok_or_else(|| "Repository not found".to_string())?;
        next.push(repo);
    }
    if !by_id.is_empty() {
        return Err("Repository list does not match this group.".into());
    }
    group.repos = next;
    persist_data(&app, &data)
}

#[tauri::command]
pub fn reorder_groups(
    app: AppHandle,
    state: State<AppState>,
    group_ids: Vec<String>,
) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    if group_ids.len() != data.groups.len() {
        return Err("Group list does not match saved groups.".into());
    }
    let mut by_id: std::collections::HashMap<_, _> = data
        .groups
        .drain(..)
        .map(|group| (group.id.clone(), group))
        .collect();
    let mut next = Vec::with_capacity(group_ids.len());
    for id in group_ids {
        let group = by_id
            .remove(&id)
            .ok_or_else(|| "Group not found".to_string())?;
        next.push(group);
    }
    if !by_id.is_empty() {
        return Err("Group list does not match saved groups.".into());
    }
    data.groups = next;
    persist_data(&app, &data)
}

#[tauri::command]
pub fn reorder_standalone_repos(
    app: AppHandle,
    state: State<AppState>,
    repo_ids: Vec<String>,
) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    if repo_ids.len() != data.repos.len() {
        return Err("Repository list does not match saved repositories.".into());
    }
    let mut by_id: std::collections::HashMap<_, _> = data
        .repos
        .drain(..)
        .map(|repo| (repo.id.clone(), repo))
        .collect();
    let mut next = Vec::with_capacity(repo_ids.len());
    for id in repo_ids {
        let repo = by_id
            .remove(&id)
            .ok_or_else(|| "Repository not found".to_string())?;
        next.push(repo);
    }
    if !by_id.is_empty() {
        return Err("Repository list does not match saved repositories.".into());
    }
    data.repos = next;
    persist_data(&app, &data)
}

#[tauri::command]
pub fn add_standalone_repo(
    app: AppHandle,
    state: State<AppState>,
    path: String,
) -> Result<RepoEntry, String> {
    let git = require_git(&state)?;
    register_standalone_repo(&app, &state, &git, Path::new(&path))
}

#[tauri::command]
pub async fn clone_standalone_repo(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    parent: String,
    name: String,
) -> Result<RepoEntry, String> {
    let git = require_git(&state)?;
    let clone_git = git.clone();
    let dest = tauri::async_runtime::spawn_blocking(move || {
        git::clone_repo(&clone_git, &url, Path::new(parent.trim()), &name)
    })
    .await
    .map_err(|err| err.to_string())??;
    register_standalone_repo(&app, &state, &git, &dest)
}

fn register_standalone_repo(
    app: &AppHandle,
    state: &AppState,
    git: &Path,
    path: &Path,
) -> Result<RepoEntry, String> {
    let root = git::repo_root(git, path)?;

    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    if path_already_added(&data, &root) {
        return Err("That repository is already added.".into());
    }

    let entry = RepoEntry {
        id: uuid::Uuid::new_v4().to_string(),
        path: root,
        label: String::new(),
        header_color: String::new(),
    };
    data.repos.push(entry.clone());
    persist_data(app, &data)?;
    Ok(entry)
}

#[tauri::command]
pub fn update_standalone_repo(
    app: AppHandle,
    state: State<AppState>,
    repo_id: String,
    label: Option<String>,
    header_color: Option<String>,
) -> Result<RepoEntry, String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let updated = {
        let repo = data
            .repos
            .iter_mut()
            .find(|repo| repo.id == repo_id)
            .ok_or_else(|| "Repository not found".to_string())?;
        if let Some(label) = label {
            repo.label = label.trim().to_string();
        }
        if let Some(color) = header_color {
            repo.header_color = sanitize_color(&color).unwrap_or_default();
        }
        repo.clone()
    };
    persist_data(&app, &data)?;
    Ok(updated)
}

#[tauri::command]
pub fn remove_standalone_repo(
    app: AppHandle,
    state: State<AppState>,
    repo_id: String,
) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let before = data.repos.len();
    data.repos.retain(|repo| repo.id != repo_id);
    if data.repos.len() == before {
        return Err("Repository not found".into());
    }
    persist_data(&app, &data)
}

#[tauri::command]
pub fn standalone_status(
    state: State<AppState>,
    fetch: Option<bool>,
) -> Result<Vec<RepoStatus>, String> {
    let git = require_git(&state)?;
    let data = state.data.lock().map_err(|err| err.to_string())?;
    let repos = data.repos.clone();
    drop(data);
    let fetch = fetch.unwrap_or(false);
    let mut statuses = Vec::new();
    for repo in repos {
        let path = Path::new(&repo.path);
        if fetch {
            git::fetch_remote(&git, path);
        }
        statuses.push(status_from_live(&repo, git::live_status(&git, path)));
    }
    Ok(statuses)
}

#[tauri::command]
pub fn group_status(
    state: State<AppState>,
    group_id: String,
    fetch: Option<bool>,
) -> Result<Vec<RepoStatus>, String> {
    let (git, group) = repo_list(&state, &group_id)?;
    let fetch = fetch.unwrap_or(false);
    let mut statuses = Vec::new();
    for repo in group.repos {
        let path = Path::new(&repo.path);
        if fetch {
            git::fetch_remote(&git, path);
        }
        statuses.push(status_from_live(&repo, git::live_status(&git, path)));
    }
    Ok(statuses)
}

#[tauri::command]
pub async fn refresh_repo(
    state: State<'_, AppState>,
    group_id: String,
    repo_id: String,
    fetch: Option<bool>,
) -> Result<RepoStatus, String> {
    let (git, repo) = repo_entry(&state, &group_id, &repo_id)?;
    let should_fetch = fetch.unwrap_or(true);
    tauri::async_runtime::spawn_blocking(move || {
        let path = Path::new(&repo.path);
        if should_fetch {
            git::fetch_remote(&git, path);
        }
        Ok(status_from_live(&repo, git::live_status(&git, path)))
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn pull_repo(
    state: State<'_, AppState>,
    group_id: String,
    repo_id: String,
    branch: Option<String>,
) -> Result<RepoActionResult, String> {
    let (git, repo) = repo_entry(&state, &group_id, &repo_id)?;
    let branch = branch
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(ref name) = branch {
        git::validate_ref(name)?;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let path = Path::new(&repo.path);
        let output = match git::run_pull(&git, path, branch.as_deref()) {
            Ok(output) => output,
            Err(err) => {
                return Ok(RepoActionResult {
                    path: repo.path,
                    ok: false,
                    message: err,
                });
            }
        };
        let fallback = match &branch {
            Some(name) => format!("Pulled origin/{name} into the current branch"),
            None => "Pulled current branch".into(),
        };
        Ok(RepoActionResult {
            path: repo.path,
            ok: output.success,
            message: fallback_message(&git::combined_message(&output), output.success, &fallback),
        })
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn pull_current(state: State<AppState>, group_id: String) -> Result<Vec<RepoActionResult>, String> {
    let (git, group) = repo_list(&state, &group_id)?;
    Ok(group
        .repos
        .into_iter()
        .map(|repo| {
            let output = match git::run_pull(&git, Path::new(&repo.path), None) {
                Ok(output) => output,
                Err(err) => {
                    return RepoActionResult {
                        path: repo.path,
                        ok: false,
                        message: err,
                    };
                }
            };
            RepoActionResult {
                path: repo.path,
                ok: output.success,
                message: fallback_message(&git::combined_message(&output), output.success, "Pulled current branch"),
            }
        })
        .collect())
}

#[tauri::command]
pub fn pull_from_branch(
    state: State<AppState>,
    group_id: String,
) -> Result<Vec<RepoActionResult>, String> {
    let (git, group) = repo_list(&state, &group_id)?;
    let branch = group.pull_from_branch.trim();
    if branch.is_empty() {
        return Err("Pull-from branch is not set".into());
    }
    git::validate_ref(branch)?;
    Ok(group
        .repos
        .into_iter()
        .map(|repo| {
            let output = match git::run_pull(&git, Path::new(&repo.path), Some(branch)) {
                Ok(output) => output,
                Err(err) => {
                    return RepoActionResult {
                        path: repo.path,
                        ok: false,
                        message: err,
                    };
                }
            };
            RepoActionResult {
                path: repo.path,
                ok: output.success,
                message: fallback_message(
                    &git::combined_message(&output),
                    output.success,
                    &format!("Pulled origin/{branch} into the current branch"),
                ),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn checkout_repo(
    state: State<'_, AppState>,
    group_id: String,
    repo_id: String,
    target: String,
    fallbacks: Vec<String>,
) -> Result<RepoActionResult, String> {
    let (git, repo) = repo_entry(&state, &group_id, &repo_id)?;
    let mut branches = Vec::new();
    let target = target.trim().to_string();
    if !target.is_empty() {
        git::validate_ref(&target)?;
        branches.push(target);
    }
    for fallback in fallbacks {
        let fallback = fallback.trim().to_string();
        if !fallback.is_empty() && !branches.contains(&fallback) {
            git::validate_ref(&fallback)?;
            branches.push(fallback);
        }
    }
    if branches.is_empty() {
        return Err("Provide a branch to check out".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        match git::checkout_with_fallbacks(&git, Path::new(&repo.path), &branches) {
            Ok(message) => Ok(RepoActionResult {
                path: repo.path,
                ok: true,
                message,
            }),
            Err(message) => Ok(RepoActionResult {
                path: repo.path,
                ok: false,
                message,
            }),
        }
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn checkout_all(
    state: State<AppState>,
    group_id: String,
    target: String,
    fallbacks: Vec<String>,
) -> Result<Vec<RepoActionResult>, String> {
    let (git, group) = repo_list(&state, &group_id)?;
    let mut branches = Vec::new();
    let target = target.trim().to_string();
    if !target.is_empty() {
        git::validate_ref(&target)?;
        branches.push(target);
    }
    for fallback in fallbacks {
        let fallback = fallback.trim().to_string();
        if !fallback.is_empty() && !branches.contains(&fallback) {
            git::validate_ref(&fallback)?;
            branches.push(fallback);
        }
    }
    if branches.is_empty() {
        return Err("Provide a branch to check out".into());
    }

    Ok(group
        .repos
        .into_iter()
        .map(|repo| match git::checkout_with_fallbacks(&git, Path::new(&repo.path), &branches) {
            Ok(message) => RepoActionResult {
                path: repo.path,
                ok: true,
                message,
            },
            Err(message) => RepoActionResult {
                path: repo.path,
                ok: false,
                message,
            },
        })
        .collect())
}

#[tauri::command]
pub fn log_graph(state: State<AppState>, path: String) -> Result<Vec<CommitNode>, String> {
    let git = require_git(&state)?;
    git::log_graph(&git, Path::new(&path))
}

#[tauri::command]
pub fn file_log(state: State<AppState>, path: String, file: String) -> Result<Vec<CommitNode>, String> {
    let git = require_git(&state)?;
    git::file_log(&git, Path::new(&path), &file)
}

#[tauri::command]
pub fn working_tree(state: State<AppState>, path: String) -> Result<Vec<WorkingTreeFile>, String> {
    let git = require_git(&state)?;
    git::working_tree(&git, Path::new(&path))
}

#[tauri::command]
pub fn repo_files(state: State<AppState>, path: String) -> Result<Vec<RepoFile>, String> {
    let git = require_git(&state)?;
    git::repo_files(&git, Path::new(&path))
}

#[tauri::command]
pub fn discard_all_changes(state: State<AppState>, path: String) -> Result<(), String> {
    let git = require_git(&state)?;
    git::discard_all_changes(&git, Path::new(&path))
}

#[tauri::command]
pub fn discard_file_changes(
    state: State<AppState>,
    path: String,
    file: String,
    staged: bool,
) -> Result<(), String> {
    let git = require_git(&state)?;
    git::discard_file_changes(&git, Path::new(&path), &file, staged)
}

#[tauri::command]
pub fn last_commit(state: State<AppState>, path: String) -> Result<LastCommit, String> {
    let git = require_git(&state)?;
    git::last_commit(&git, Path::new(&path))
}

#[tauri::command]
pub async fn commit(
    state: State<'_, AppState>,
    path: String,
    title: String,
    description: String,
    amend: bool,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::commit(&git, Path::new(&path), &title, &description, amend)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn abort_operation(state: State<AppState>, path: String) -> Result<String, String> {
    let git = require_git(&state)?;
    git::abort_operation(&git, Path::new(&path))
}

#[tauri::command]
pub fn continue_operation(state: State<AppState>, path: String) -> Result<String, String> {
    let git = require_git(&state)?;
    git::continue_operation(&git, Path::new(&path))
}

#[tauri::command]
pub fn open_in_editor(state: State<AppState>, path: String, file: String) -> Result<(), String> {
    let editor = state
        .data
        .lock()
        .map_err(|err| err.to_string())?
        .editor
        .clone();
    git::open_in_editor(Path::new(&path), &file, &editor)
}

#[tauri::command]
pub fn repo_remote_url(state: State<AppState>, path: String) -> Result<String, String> {
    let git = require_git(&state)?;
    git::repo_remote_browse_url(&git, Path::new(&path))
}

#[tauri::command]
pub fn open_repo_in_finder(path: String) -> Result<(), String> {
    git::open_repo_in_finder(Path::new(&path))
}

#[tauri::command]
pub fn reveal_file_in_finder(path: String, file: String) -> Result<(), String> {
    git::reveal_file_in_finder(Path::new(&path), &file)
}

#[tauri::command]
pub fn ignore_working_tree_path(
    state: State<AppState>,
    path: String,
    file: String,
    kind: String,
) -> Result<(), String> {
    let git = require_git(&state)?;
    git::ignore_working_tree_path(&git, Path::new(&path), &file, &kind)
}

#[tauri::command]
pub fn delete_working_tree_file(
    state: State<AppState>,
    path: String,
    file: String,
) -> Result<(), String> {
    let git = require_git(&state)?;
    git::delete_working_tree_file(&git, Path::new(&path), &file)
}

#[tauri::command]
pub fn stage_file(state: State<AppState>, path: String, file: String) -> Result<(), String> {
    let git = require_git(&state)?;
    git::stage_file(&git, Path::new(&path), &file)
}

#[tauri::command]
pub fn stage_all(state: State<AppState>, path: String) -> Result<(), String> {
    let git = require_git(&state)?;
    git::stage_all(&git, Path::new(&path))
}

#[tauri::command]
pub fn unstage_file(state: State<AppState>, path: String, file: String) -> Result<(), String> {
    let git = require_git(&state)?;
    git::unstage_file(&git, Path::new(&path), &file)
}

#[tauri::command]
pub fn unstage_all(state: State<AppState>, path: String) -> Result<(), String> {
    let git = require_git(&state)?;
    git::unstage_all(&git, Path::new(&path))
}

#[tauri::command]
pub fn list_local_branches(state: State<AppState>, path: String) -> Result<Vec<String>, String> {
    let git = require_git(&state)?;
    git::local_branches(&git, Path::new(&path))
}

#[tauri::command]
pub async fn list_branch_tracking(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<BranchTracking>, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::local_branch_tracking(&git, Path::new(&path)))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn branch_overview(
    state: State<'_, AppState>,
    path: String,
    preferred: Option<String>,
    classify: Option<bool>,
) -> Result<BranchOverview, String> {
    let git = require_git(&state)?;
    let classify = classify.unwrap_or(true);
    tauri::async_runtime::spawn_blocking(move || {
        git::branch_overview_with(&git, Path::new(&path), preferred.as_deref(), classify)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn delete_local_branch(
    state: State<'_, AppState>,
    path: String,
    branch: String,
    force: bool,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::delete_local_branch(&git, Path::new(&path), &branch, force)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn delete_merged_branches(
    state: State<'_, AppState>,
    path: String,
    preferred: Option<String>,
    force: bool,
    names: Option<Vec<String>>,
) -> Result<DeleteMergedResult, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::delete_merged_branches(
            &git,
            Path::new(&path),
            preferred.as_deref(),
            force,
            names.as_deref(),
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn checkout_local_branch(
    state: State<'_, AppState>,
    path: String,
    branch: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::checkout_local_branch(&git, Path::new(&path), &branch)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn create_and_checkout_branch(
    state: State<'_, AppState>,
    path: String,
    branch: String,
    start: Option<String>,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::create_and_checkout_branch(
            &git,
            Path::new(&path),
            &branch,
            start.as_deref().unwrap_or(""),
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn checkout_commit(
    state: State<'_, AppState>,
    path: String,
    hash: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::checkout_commit(&git, Path::new(&path), &hash))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn cherry_pick_commits(
    state: State<'_, AppState>,
    path: String,
    hashes: Vec<String>,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::cherry_pick_commits(&git, Path::new(&path), &hashes)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn revert_commits(
    state: State<'_, AppState>,
    path: String,
    hashes: Vec<String>,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::revert_commits(&git, Path::new(&path), &hashes))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn commit_remote_url(
    state: State<'_, AppState>,
    path: String,
    hash: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::commit_remote_url(&git, Path::new(&path), &hash)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn merge_local_branch(
    state: State<'_, AppState>,
    path: String,
    source: String,
    target: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::merge_local_branch(&git, Path::new(&path), &source, &target)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn list_remotes(state: State<'_, AppState>, path: String) -> Result<Vec<RemoteEntry>, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::list_remotes(&git, Path::new(&path)))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn fetch_named_remote(
    state: State<'_, AppState>,
    path: String,
    remote: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::fetch_named_remote(&git, Path::new(&path), &remote)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn add_remote(
    state: State<'_, AppState>,
    path: String,
    name: String,
    url: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::add_remote(&git, Path::new(&path), name.trim(), &url)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn update_remote(
    state: State<'_, AppState>,
    path: String,
    name: String,
    new_name: String,
    url: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::update_remote(&git, Path::new(&path), &name, new_name.trim(), &url)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn remove_remote(
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::remove_remote(&git, Path::new(&path), &name))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn remote_branches(
    state: State<'_, AppState>,
    path: String,
    remote: String,
) -> Result<RemoteOverview, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::remote_branches(&git, Path::new(&path), &remote)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn checkout_remote_branch(
    state: State<'_, AppState>,
    path: String,
    remote: String,
    branch: String,
    local_name: Option<String>,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::checkout_remote_branch(
            &git,
            Path::new(&path),
            &remote,
            &branch,
            local_name.as_deref(),
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn merge_remote_branch(
    state: State<'_, AppState>,
    path: String,
    remote: String,
    branch: String,
    target: String,
    allow_merge_commit: bool,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::merge_remote_branch(
            &git,
            Path::new(&path),
            &remote,
            &branch,
            &target,
            allow_merge_commit,
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn push_local_branch(
    state: State<'_, AppState>,
    path: String,
    branch: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::push_local_branch(&git, Path::new(&path), &branch)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn delete_remote_branch(
    state: State<'_, AppState>,
    path: String,
    remote: String,
    branch: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::delete_remote_branch(&git, Path::new(&path), &remote, &branch)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn rename_local_branch(
    state: State<'_, AppState>,
    path: String,
    branch: String,
    new_name: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::rename_local_branch(&git, Path::new(&path), &branch, &new_name)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn repo_pull(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::pull(&git, Path::new(&path)))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn repo_push(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::push(&git, Path::new(&path)))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn reset_unpushed_commits(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::reset_unpushed_commits(&git, Path::new(&path)))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn file_diff(
    state: State<AppState>,
    path: String,
    file: String,
    staged: bool,
) -> Result<String, String> {
    let git = require_git(&state)?;
    git::file_diff(&git, Path::new(&path), &file, staged)
}

#[tauri::command]
pub fn commit_files(
    state: State<AppState>,
    path: String,
    hash: String,
) -> Result<Vec<CommitFile>, String> {
    let git = require_git(&state)?;
    git::commit_files(&git, Path::new(&path), &hash)
}

#[tauri::command]
pub fn stash_list(state: State<AppState>, path: String) -> Result<Vec<StashEntry>, String> {
    let git = require_git(&state)?;
    git::stash_list(&git, Path::new(&path))
}

#[tauri::command]
pub async fn stash_apply(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::stash_apply(&git, Path::new(&path), index))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn stash_pop(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::stash_pop(&git, Path::new(&path), index))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn stash_push(
    state: State<'_, AppState>,
    path: String,
    message: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::stash_push(&git, Path::new(&path), &message))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn stash_file(
    state: State<'_, AppState>,
    path: String,
    file: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::stash_file(&git, Path::new(&path), &file))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn stash_drop(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::stash_drop(&git, Path::new(&path), index))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn tag_list(state: State<AppState>, path: String) -> Result<Vec<TagEntry>, String> {
    let git = require_git(&state)?;
    git::tag_list(&git, Path::new(&path))
}

#[tauri::command]
pub async fn create_tag(
    state: State<'_, AppState>,
    path: String,
    name: String,
    message: String,
    target: Option<String>,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::create_tag(
            &git,
            Path::new(&path),
            &name,
            &message,
            target.as_deref().unwrap_or(""),
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub async fn delete_tag(
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || git::delete_tag(&git, Path::new(&path), &name))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub fn commit_file_diff(
    state: State<AppState>,
    path: String,
    hash: String,
    file: String,
) -> Result<String, String> {
    let git = require_git(&state)?;
    git::commit_file_diff(&git, Path::new(&path), &hash, &file)
}

#[tauri::command]
pub async fn file_blame(
    state: State<'_, AppState>,
    path: String,
    file: String,
    rev: Option<String>,
    staged: bool,
    old_path: Option<String>,
) -> Result<FileBlame, String> {
    let git = require_git(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        git::file_blame(
            &git,
            Path::new(&path),
            &file,
            rev.as_deref(),
            staged,
            old_path.as_deref(),
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

fn status_from_live(repo: &RepoEntry, live: Result<git::LiveStatus, String>) -> RepoStatus {
    match live {
        Ok(status) => RepoStatus {
            id: repo.id.clone(),
            name: git::folder_name(&repo.path),
            path: repo.path.clone(),
            branch: status.branch,
            ahead: status.ahead,
            behind: status.behind,
            dirty: status.dirty,
            insertions: status.insertions,
            deletions: status.deletions,
            changed_files: status.changed_files,
            conflicted_files: status.conflicted_files,
            operation: status.operation,
        },
        Err(err) => RepoStatus {
            id: repo.id.clone(),
            name: git::folder_name(&repo.path),
            path: repo.path.clone(),
            branch: format!("error: {err}"),
            ahead: 0,
            behind: 0,
            dirty: false,
            insertions: 0,
            deletions: 0,
            changed_files: 0,
            conflicted_files: 0,
            operation: String::new(),
        },
    }
}

#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Choose a file to export.".into());
    }
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("Could not create the export folder: {err}"))?;
        }
    }
    std::fs::write(&path, contents).map_err(|err| format!("Could not write the file: {err}"))
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("Choose a file to import.".into());
    }
    std::fs::read_to_string(&path).map_err(|err| format!("Could not read the file: {err}"))
}

#[tauri::command]
pub fn git_config(state: State<AppState>) -> Result<GitConfig, String> {
    let git = require_git(&state)?;
    git::read_git_config(&git)
}

#[tauri::command]
pub fn update_git_config_value(
    state: State<AppState>,
    key: String,
    value: String,
) -> Result<GitConfig, String> {
    let git = require_git(&state)?;
    git::update_git_config_value(&git, &key, &value)
}

#[tauri::command]
pub fn save_git_config_file(state: State<AppState>, contents: String) -> Result<GitConfig, String> {
    let git = require_git(&state)?;
    git::write_git_config(&git, &contents)
}

#[tauri::command]
pub fn reveal_git_config_file() -> Result<(), String> {
    let path = git::git_config_path();
    if path.is_file() {
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .status()
            .map_err(|err| format!("Could not reveal the git config file: {err}"))?;
        if status.success() {
            return Ok(());
        }
        return Err("Could not reveal the git config file.".into());
    }
    let parent = path.parent().filter(|dir| dir.is_dir()).ok_or_else(|| {
        "Git config file does not exist yet. Save a value to create it.".to_string()
    })?;
    let status = std::process::Command::new("open")
        .arg(parent)
        .status()
        .map_err(|err| format!("Could not open the git config folder: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not open the git config folder.".into())
    }
}

#[tauri::command]
pub fn settings_file_path(app: AppHandle) -> Result<String, String> {
    persist::data_path(&app)?
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "Settings path is not valid UTF-8".into())
}

#[tauri::command]
pub fn reveal_settings_file(app: AppHandle) -> Result<(), String> {
    let path = persist::data_path(&app)?;
    if !path.exists() {
        persist::save(&app, &AppData::default())?;
    }
    let status = std::process::Command::new("open")
        .arg("-R")
        .arg(&path)
        .status()
        .map_err(|err| format!("Could not reveal the settings file: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("Could not reveal the settings file.".into())
    }
}

#[tauri::command]
pub fn command_history() -> Vec<crate::command_log::CommandLogEntry> {
    crate::command_log::list()
}

#[tauri::command]
pub fn command_history_paused() -> bool {
    crate::command_log::paused()
}

#[tauri::command]
pub fn set_command_history_paused(paused: bool) -> Result<(), String> {
    crate::command_log::set_paused(paused)
}

#[tauri::command]
pub fn clear_command_history() -> Result<(), String> {
    crate::command_log::clear()
}

fn fallback_message(message: &str, ok: bool, success_fallback: &str) -> String {
    if !message.trim().is_empty() {
        message.trim().to_string()
    } else if ok {
        success_fallback.to_string()
    } else {
        "Git command failed".into()
    }
}
