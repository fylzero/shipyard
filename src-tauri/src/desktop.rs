use std::io;
use std::path::Path;

#[cfg(target_os = "macos")]
pub const FILE_MANAGER: &str = "Finder";
#[cfg(not(target_os = "macos"))]
pub const FILE_MANAGER: &str = "the file manager";

/// Selects the item in the file manager. `Ok(false)` means the launcher ran but failed.
#[cfg(target_os = "macos")]
pub fn reveal(path: &Path) -> io::Result<bool> {
    Ok(std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .status()?
        .success())
}

#[cfg(not(target_os = "macos"))]
pub fn reveal(path: &Path) -> io::Result<bool> {
    tauri_plugin_opener::reveal_item_in_dir(path)
        .map_err(|err| io::Error::other(err.to_string()))?;
    Ok(true)
}

/// Opens a file in its default app, or a folder in the file manager.
#[cfg(target_os = "macos")]
pub fn open(path: &Path) -> io::Result<bool> {
    Ok(std::process::Command::new("open")
        .arg(path)
        .status()?
        .success())
}

#[cfg(not(target_os = "macos"))]
pub fn open(path: &Path) -> io::Result<bool> {
    tauri_plugin_opener::open_path(path, None::<&str>)
        .map_err(|err| io::Error::other(err.to_string()))?;
    Ok(true)
}

/// Opens a file with an app name on macOS, or a command on PATH elsewhere.
#[cfg(target_os = "macos")]
pub fn open_with(path: &Path, app: &str) -> io::Result<bool> {
    Ok(std::process::Command::new("open")
        .arg("-a")
        .arg(app)
        .arg("--")
        .arg(path)
        .status()?
        .success())
}

#[cfg(not(target_os = "macos"))]
pub fn open_with(path: &Path, app: &str) -> io::Result<bool> {
    tauri_plugin_opener::open_path(path, Some(app))
        .map_err(|err| io::Error::other(err.to_string()))?;
    Ok(true)
}
