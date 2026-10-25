use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::models::AppData;

const SETTINGS_FILE: &str = "settings.json";
const LEGACY_SETTINGS_FILE: &str = "groups.json";
const LEGACY_BUNDLE_ID: &str = "com.krakdown.app";

fn app_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("Could not resolve the app data directory: {err}"))?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("Could not create the app data directory: {err}"))?;
    Ok(dir)
}

fn migrate_legacy_bundle(dest: &Path) -> Result<(), String> {
    if dest.join(SETTINGS_FILE).exists() || dest.join(LEGACY_SETTINGS_FILE).exists() {
        return Ok(());
    }
    let Some(parent) = dest.parent() else {
        return Ok(());
    };
    let src = parent.join(LEGACY_BUNDLE_ID);
    if !src.is_dir() {
        return Ok(());
    }
    for name in [SETTINGS_FILE, LEGACY_SETTINGS_FILE, "command-history.jsonl"] {
        let from = src.join(name);
        let to = dest.join(name);
        if from.is_file() && !to.exists() {
            fs::copy(&from, &to).map_err(|err| {
                format!("Could not copy {name} from the previous app data folder: {err}")
            })?;
        }
    }
    Ok(())
}

pub fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_dir(app)?.join(SETTINGS_FILE))
}

fn legacy_data_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_dir(app)?.join(LEGACY_SETTINGS_FILE))
}

pub fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_dir(app)?.join("command-history.jsonl"))
}

fn read_app_data(path: &Path, label: &str) -> Result<AppData, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("Could not read {label}: {err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("Could not parse {label}: {err}"))
}

fn remove_legacy(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

pub fn load(app: &AppHandle) -> Result<AppData, String> {
    migrate_legacy_bundle(&app_dir(app)?)?;
    let path = data_path(app)?;
    let legacy = legacy_data_path(app)?;
    if path.exists() {
        let data = read_app_data(&path, SETTINGS_FILE)?;
        remove_legacy(&legacy);
        return Ok(data);
    }
    if legacy.exists() {
        let data = read_app_data(&legacy, LEGACY_SETTINGS_FILE)?;
        save(app, &data)?;
        remove_legacy(&legacy);
        return Ok(data);
    }
    Ok(AppData::default())
}

pub fn save(app: &AppHandle, data: &AppData) -> Result<(), String> {
    let path = data_path(app)?;
    let raw = serde_json::to_string_pretty(data)
        .map_err(|err| format!("Could not serialize {SETTINGS_FILE}: {err}"))?;
    fs::write(&path, raw).map_err(|err| format!("Could not write {SETTINGS_FILE}: {err}"))?;
    remove_legacy(&legacy_data_path(app)?);
    Ok(())
}
