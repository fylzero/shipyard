mod command_log;
mod commands;
mod git;
mod menu;
mod models;
mod persist;
mod pty;
mod watcher;
mod window_state;

use commands::AppState;
use pty::TerminalState;
use std::sync::Mutex;
use tauri::Manager;
use watcher::RepoWatcherState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .menu(|handle| menu::build(handle))
        .on_menu_event(|app, event| {
            menu::handle_event(app, event.id().as_ref());
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let data = persist::load(&handle).unwrap_or_default();
            let bounds = data.window.clone();
            if let Ok(path) = persist::history_path(&handle) {
                command_log::init(path, Some(handle.clone()));
            }
            app.manage(AppState {
                data: Mutex::new(data),
                git: git::resolve_git_binary(),
            });
            app.manage(TerminalState::default());
            app.manage(RepoWatcherState::default());
            if let (Some(window), Some(bounds)) = (app.get_webview_window("main"), bounds) {
                window_state::apply(&window, &bounds);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            window_state::handle_event(window, event);
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::create_group,
            commands::rename_group,
            commands::delete_group,
            commands::toggle_group,
            commands::set_all_groups_expanded,
            commands::update_group_settings,
            commands::update_app_settings,
            commands::update_files_pane_width,
            commands::update_terminal_pane_height,
            commands::update_diff_mode,
            commands::update_diff_font_family,
            commands::update_diff_font_size,
            commands::update_terminal_font_family,
            commands::update_terminal_font_size,
            commands::update_editor,
            commands::update_refresh_active_hours,
            window_state::get_window_state,
            window_state::update_window_state,
            commands::replace_app_data,
            commands::add_repo,
            commands::add_standalone_repo,
            commands::clone_standalone_repo,
            commands::update_standalone_repo,
            commands::remove_standalone_repo,
            commands::standalone_status,
            commands::remove_repo,
            commands::reorder_group_repos,
            commands::reorder_standalone_repos,
            commands::reorder_groups,
            commands::group_status,
            commands::refresh_repo,
            commands::pull_repo,
            commands::pull_current,
            commands::pull_from_branch,
            commands::checkout_repo,
            commands::checkout_all,
            commands::log_graph,
            commands::file_log,
            commands::working_tree,
            commands::repo_files,
            watcher::watch_repo,
            watcher::unwatch_repo,
            watcher::watch_repo_git,
            watcher::unwatch_repo_git,
            commands::discard_all_changes,
            commands::discard_file_changes,
            commands::last_commit,
            commands::commit,
            commands::abort_operation,
            commands::continue_operation,
            commands::open_in_editor,
            commands::repo_remote_url,
            commands::open_repo_in_finder,
            commands::reveal_file_in_finder,
            commands::ignore_working_tree_path,
            commands::delete_working_tree_file,
            commands::stage_file,
            commands::stage_all,
            commands::unstage_file,
            commands::unstage_all,
            commands::list_local_branches,
            commands::list_branch_tracking,
            commands::branch_overview,
            commands::delete_local_branch,
            commands::delete_merged_branches,
            commands::checkout_local_branch,
            commands::create_and_checkout_branch,
            commands::checkout_commit,
            commands::cherry_pick_commits,
            commands::revert_commits,
            commands::commit_remote_url,
            commands::merge_local_branch,
            commands::list_remotes,
            commands::fetch_named_remote,
            commands::add_remote,
            commands::update_remote,
            commands::remove_remote,
            commands::remote_branches,
            commands::checkout_remote_branch,
            commands::merge_remote_branch,
            commands::push_local_branch,
            commands::delete_remote_branch,
            commands::rename_local_branch,
            commands::repo_pull,
            commands::repo_push,
            commands::reset_unpushed_commits,
            commands::file_diff,
            commands::commit_files,
            commands::commit_file_diff,
            commands::file_blame,
            commands::stash_list,
            commands::stash_push,
            commands::stash_file,
            commands::stash_apply,
            commands::stash_pop,
            commands::stash_drop,
            commands::tag_list,
            commands::create_tag,
            commands::delete_tag,
            commands::write_text_file,
            commands::read_text_file,
            commands::git_config,
            commands::update_git_config_value,
            commands::save_git_config_file,
            commands::reveal_git_config_file,
            commands::settings_file_path,
            commands::reveal_settings_file,
            commands::command_history,
            commands::command_history_paused,
            commands::set_command_history_paused,
            commands::clear_command_history,
            pty::open_terminal,
            pty::write_terminal,
            pty::resize_terminal,
            pty::close_terminal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
