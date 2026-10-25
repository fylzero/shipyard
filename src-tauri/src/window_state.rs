use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, State, WebviewWindow, Window, WindowEvent,
};

use crate::commands::AppState;
use crate::models::{WindowState, MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};
use crate::persist;

static SAVE_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn apply(window: &WebviewWindow, state: &WindowState) {
    apply_inner(window, state, true);
}

fn apply_inner(window: &WebviewWindow, state: &WindowState, apply_geometry: bool) {
    let is_maximized = window.is_maximized().unwrap_or(false);
    if state.maximized {
        if !is_maximized {
            let _ = window.maximize();
        }
        return;
    }
    if is_maximized {
        let _ = window.unmaximize();
        if apply_geometry {
            let window = window.clone();
            let state = state.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(120));
                apply_frame(&window, &state);
            });
        }
        return;
    }
    apply_frame(window, state);
}

fn apply_frame(window: &WebviewWindow, state: &WindowState) {
    let width = state.width.max(MIN_WINDOW_WIDTH);
    let height = state.height.max(MIN_WINDOW_HEIGHT);
    let _ = window.set_size(LogicalSize::new(width, height));
    if position_is_visible(window, state.x, state.y) {
        let _ = window.set_position(LogicalPosition::new(state.x, state.y));
    }
}

pub fn current(app: &AppHandle) -> Result<WindowState, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    let live = capture_webview(&window)?;
    Ok(with_restore_bounds(app, live))
}

fn with_restore_bounds(app: &AppHandle, live: WindowState) -> WindowState {
    if !live.maximized {
        return live;
    }
    let handle = app.state::<AppState>();
    let Ok(state) = handle.data.lock() else {
        return live;
    };
    match &state.window {
        Some(prev) => WindowState {
            x: prev.x,
            y: prev.y,
            width: prev.width,
            height: prev.height,
            maximized: true,
        },
        None => live,
    }
}

pub fn handle_event(window: &Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => schedule_save(window.clone()),
        WindowEvent::CloseRequested { .. } => {
            SAVE_GENERATION.fetch_add(1, Ordering::SeqCst);
            let _ = persist_now(window);
        }
        _ => {}
    }
}

fn schedule_save(window: Window) {
    let generation = SAVE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(350));
        if SAVE_GENERATION.load(Ordering::SeqCst) != generation {
            return;
        }
        let _ = persist_now(&window);
    });
}

fn persist_now(window: &Window) -> Result<(), String> {
    if window.is_minimized().unwrap_or(false) {
        return Ok(());
    }
    let captured = capture(window)?;
    let handle = window.app_handle();
    let state = handle.state::<AppState>();
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    let next = if captured.maximized {
        match &data.window {
            Some(prev) => WindowState {
                x: prev.x,
                y: prev.y,
                width: prev.width,
                height: prev.height,
                maximized: true,
            },
            None => captured,
        }
    } else {
        captured
    };
    if data.window.as_ref() == Some(&next) {
        return Ok(());
    }
    data.window = Some(next);
    persist::save(handle, &data)
}

fn capture(window: &Window) -> Result<WindowState, String> {
    let scale = window.scale_factor().map_err(|err| err.to_string())?;
    let size = window
        .inner_size()
        .map_err(|err| err.to_string())?
        .to_logical::<u32>(scale);
    let position = window
        .outer_position()
        .map_err(|err| err.to_string())?
        .to_logical::<i32>(scale);
    Ok(WindowState {
        x: position.x,
        y: position.y,
        width: size.width.max(MIN_WINDOW_WIDTH),
        height: size.height.max(MIN_WINDOW_HEIGHT),
        maximized: window.is_maximized().unwrap_or(false),
    })
}

fn capture_webview(window: &WebviewWindow) -> Result<WindowState, String> {
    let scale = window.scale_factor().map_err(|err| err.to_string())?;
    let size = window
        .inner_size()
        .map_err(|err| err.to_string())?
        .to_logical::<u32>(scale);
    let position = window
        .outer_position()
        .map_err(|err| err.to_string())?
        .to_logical::<i32>(scale);
    Ok(WindowState {
        x: position.x,
        y: position.y,
        width: size.width.max(MIN_WINDOW_WIDTH),
        height: size.height.max(MIN_WINDOW_HEIGHT),
        maximized: window.is_maximized().unwrap_or(false),
    })
}

#[tauri::command]
pub fn get_window_state(app: AppHandle) -> Result<WindowState, String> {
    current(&app)
}

#[tauri::command]
pub fn update_window_state(
    app: AppHandle,
    state: State<AppState>,
    window: WindowState,
) -> Result<WindowState, String> {
    let mut next = window;
    next.width = next.width.max(MIN_WINDOW_WIDTH);
    next.height = next.height.max(MIN_WINDOW_HEIGHT);
    let mut geometry_changed = true;
    {
        let data = state.data.lock().map_err(|err| err.to_string())?;
        if let Some(prev) = &data.window {
            geometry_changed = prev.x != next.x
                || prev.y != next.y
                || prev.width != next.width
                || prev.height != next.height;
            if next.maximized {
                next.x = prev.x;
                next.y = prev.y;
                next.width = prev.width;
                next.height = prev.height;
                geometry_changed = false;
            }
        }
    }
    if let Some(win) = app.get_webview_window("main") {
        apply_inner(&win, &next, geometry_changed);
    }
    SAVE_GENERATION.fetch_add(1, Ordering::SeqCst);
    let mut data = state.data.lock().map_err(|err| err.to_string())?;
    data.window = Some(next.clone());
    persist::save(&app, &data)?;
    Ok(next)
}

fn position_is_visible(window: &WebviewWindow, x: i32, y: i32) -> bool {
    let Ok(scale) = window.scale_factor() else {
        return true;
    };
    let Ok(monitors) = window.available_monitors() else {
        return true;
    };
    if monitors.is_empty() {
        return true;
    }
    let physical = LogicalPosition::new(x, y).to_physical::<i32>(scale);
    monitors.iter().any(|monitor| {
        let origin = monitor.position();
        let size = monitor.size();
        let left = origin.x;
        let top = origin.y;
        let right = origin.x.saturating_add(size.width as i32);
        let bottom = origin.y.saturating_add(size.height as i32);
        physical.x >= left - 48
            && physical.x < right - 80
            && physical.y >= top
            && physical.y < bottom - 48
    })
}
