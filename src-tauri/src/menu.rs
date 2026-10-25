use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const CLOSE_TAB_OR_WINDOW_ID: &str = "close-tab-or-window";
pub const OPEN_SETTINGS_ID: &str = "open-settings";
pub const CHECK_FOR_UPDATES_ID: &str = "check-for-updates";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let pkg_info = app.package_info();
    let config = app.config();
    let about_metadata = AboutMetadata {
        name: Some(pkg_info.name.clone()),
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };

    let close = MenuItem::with_id(
        app,
        CLOSE_TAB_OR_WINDOW_ID,
        "Close",
        true,
        Some("CmdOrCtrl+W"),
    )?;
    let close_window_item =
        MenuItem::with_id(app, CLOSE_TAB_OR_WINDOW_ID, "Close", true, None::<&str>)?;
    let settings = MenuItem::with_id(
        app,
        OPEN_SETTINGS_ID,
        "Settings…",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    #[cfg(target_os = "macos")]
    let check_updates = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let check_updates_help = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &close_window_item,
        ],
    )?;

    Menu::with_items(
        app,
        &[
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                pkg_info.name.clone(),
                true,
                &[
                    &PredefinedMenuItem::about(app, None, Some(about_metadata.clone()))?,
                    &check_updates,
                    &PredefinedMenuItem::separator(app)?,
                    &settings,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?,
            #[cfg(target_os = "macos")]
            &Submenu::with_items(app, "File", true, &[&close])?,
            #[cfg(not(target_os = "macos"))]
            &Submenu::with_items(app, "File", true, &[&settings, &close])?,
            &Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?,
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                "View",
                true,
                &[&PredefinedMenuItem::fullscreen(app, None)?],
            )?,
            &window_menu,
            &Submenu::with_items(
                app,
                "Help",
                true,
                &[
                    #[cfg(not(target_os = "macos"))]
                    &PredefinedMenuItem::about(app, None, Some(about_metadata))?,
                    &check_updates_help,
                ],
            )?,
        ],
    )
}

pub fn handle_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    match id {
        CLOSE_TAB_OR_WINDOW_ID => {
            let _ = window.emit(CLOSE_TAB_OR_WINDOW_ID, ());
        }
        OPEN_SETTINGS_ID => {
            let _ = window.emit(OPEN_SETTINGS_ID, ());
        }
        CHECK_FOR_UPDATES_ID => {
            let _ = window.emit(CHECK_FOR_UPDATES_ID, ());
        }
        _ => {}
    }
}
