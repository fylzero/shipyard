use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A child process that doesn't flash a console window on Windows.
pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(target_os = "macos")]
pub const FILE_MANAGER: &str = "Finder";
#[cfg(windows)]
pub const FILE_MANAGER: &str = "File Explorer";
#[cfg(not(any(target_os = "macos", windows)))]
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

#[cfg(all(unix, not(target_os = "macos")))]
pub fn open_with(path: &Path, app: &str) -> io::Result<bool> {
    tauri_plugin_opener::open_path(path, Some(app))
        .map_err(|err| io::Error::other(err.to_string()))?;
    Ok(true)
}

/**
 * Editors install `.cmd` launchers on Windows (`code.cmd`, `cursor.cmd`),
 * which the opener can't start, so Shipyard finds the program itself and
 * runs it without a console window.
 */
#[cfg(windows)]
pub fn open_with(path: &Path, app: &str) -> io::Result<bool> {
    let program = find_program(app)
        .or_else(|| installed_editor(app))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{app} was not found")))?;
    command(program).arg(path).spawn()?;
    Ok(true)
}

/// A program on PATH, trying each PATHEXT extension the way cmd does.
#[cfg(windows)]
pub fn find_program(name: &str) -> Option<std::path::PathBuf> {
    let given = Path::new(name);
    if given.is_absolute() {
        return given.is_file().then(|| given.to_path_buf());
    }
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    find_in(name, &std::env::var_os("PATH")?, &extensions)
}

#[cfg(windows)]
fn find_in(name: &str, path: &OsStr, extensions: &str) -> Option<std::path::PathBuf> {
    let given = Path::new(name);
    std::env::split_paths(path).find_map(|dir| {
        let exact = dir.join(name);
        if given.extension().is_some() && exact.is_file() {
            return Some(exact);
        }
        extensions
            .split(';')
            .filter(|ext| !ext.is_empty())
            .map(|ext| dir.join(format!("{name}{}", ext.to_ascii_lowercase())))
            .find(|candidate| candidate.is_file())
    })
}

/// Where editors install themselves when they don't add a launcher to PATH.
#[cfg(windows)]
fn installed_editor(launcher: &str) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let dir = |var: &str| std::env::var_os(var).map(PathBuf::from);
    let local = dir("LOCALAPPDATA");
    let programs = local.as_ref().map(|dir| dir.join("Programs"));
    let program_files = dir("ProgramFiles");
    let candidates: Vec<Option<PathBuf>> = match launcher {
        "cursor" => vec![programs.map(|dir| dir.join("cursor").join("Cursor.exe"))],
        "code" => vec![
            programs.map(|dir| dir.join("Microsoft VS Code").join("Code.exe")),
            program_files.map(|dir| dir.join("Microsoft VS Code").join("Code.exe")),
        ],
        "zed" => vec![programs.map(|dir| dir.join("Zed").join("Zed.exe"))],
        "subl" => vec![program_files.map(|dir| dir.join("Sublime Text").join("subl.exe"))],
        "phpstorm" | "webstorm" | "idea" => vec![local.map(|dir| {
            dir.join("JetBrains")
                .join("Toolbox")
                .join("scripts")
                .join(format!("{launcher}.cmd"))
        })],
        _ => Vec::new(),
    };
    candidates.into_iter().flatten().find(|path| path.is_file())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn finds_cmd_launchers_through_pathext() {
        let dir = std::env::temp_dir().join(format!("shipyard-path-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("code.cmd"), "@echo off").unwrap();
        let path = std::env::join_paths([&dir]).unwrap();

        assert_eq!(
            find_in("code", &path, ".EXE;.CMD"),
            Some(dir.join("code.cmd"))
        );
        assert_eq!(
            find_in("code.cmd", &path, ".EXE"),
            Some(dir.join("code.cmd"))
        );
        assert_eq!(find_in("code", &path, ".EXE"), None);
        assert_eq!(find_in("zed", &path, ".EXE;.CMD"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
