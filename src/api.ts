import { invoke } from "@tauri-apps/api/core";
import type {
  AppData,
  BranchOverview,
  BranchTracking,
  CommandLogEntry,
  CommitFile,
  CommitNode,
  DeleteMergedResult,
  DiffMode,
  FileBlame,
  GitConfig,
  LastCommit,
  RefreshActiveHours,
  RemoteEntry,
  RemoteOverview,
  RepoActionResult,
  RepoEntry,
  RepoGroup,
  RepoStatus,
  StashEntry,
  TagEntry,
  WindowState,
  RepoFile,
  WorkingTreeFile,
} from "./types";

export function getState() {
  return invoke<AppData>("get_state");
}

export function createGroup(name: string) {
  return invoke<RepoGroup>("create_group", { name });
}

export function renameGroup(groupId: string, name: string) {
  return invoke<void>("rename_group", { groupId, name });
}

export function deleteGroup(groupId: string) {
  return invoke<void>("delete_group", { groupId });
}

export function toggleGroup(groupId: string) {
  return invoke<boolean>("toggle_group", { groupId });
}

export function setAllGroupsExpanded(expanded: boolean) {
  return invoke<void>("set_all_groups_expanded", { expanded });
}

export function updateGroupSettings(
  groupId: string,
  pullFromBranch: string,
  checkoutFallbacks: string[],
  headerColor?: string,
) {
  return invoke<void>("update_group_settings", {
    groupId,
    pullFromBranch,
    checkoutFallbacks,
    headerColor,
  });
}

export function addRepo(groupId: string, path: string) {
  return invoke<RepoEntry>("add_repo", { groupId, path });
}

export function addStandaloneRepo(path: string) {
  return invoke<RepoEntry>("add_standalone_repo", { path });
}

export function cloneStandaloneRepo(url: string, parent: string, name: string) {
  return invoke<RepoEntry>("clone_standalone_repo", { url, parent, name });
}

export function updateStandaloneRepo(
  repoId: string,
  label?: string,
  headerColor?: string,
) {
  return invoke<RepoEntry>("update_standalone_repo", {
    repoId,
    label: label ?? null,
    headerColor: headerColor ?? null,
  });
}

export function removeStandaloneRepo(repoId: string) {
  return invoke<void>("remove_standalone_repo", { repoId });
}

export function standaloneStatus(fetch = false) {
  return invoke<RepoStatus[]>("standalone_status", { fetch });
}

export function removeRepo(groupId: string, repoId: string) {
  return invoke<void>("remove_repo", { groupId, repoId });
}

export function reorderGroupRepos(groupId: string, repoIds: string[]) {
  return invoke<void>("reorder_group_repos", { groupId, repoIds });
}

export function reorderGroups(groupIds: string[]) {
  return invoke<void>("reorder_groups", { groupIds });
}

export function reorderStandaloneRepos(repoIds: string[]) {
  return invoke<void>("reorder_standalone_repos", { repoIds });
}

export function updateAppSettings(refreshIntervalSeconds: number) {
  return invoke<number>("update_app_settings", { refreshIntervalSeconds });
}

export function updateFilesPaneWidth(width: number) {
  return invoke<number>("update_files_pane_width", { width });
}

export function updateTerminalPaneHeight(height: number) {
  return invoke<number>("update_terminal_pane_height", { height });
}

export function updateDiffMode(mode: DiffMode) {
  return invoke<DiffMode>("update_diff_mode", { mode });
}

export function updateDiffFontFamily(fontFamily: string) {
  return invoke<string>("update_diff_font_family", { fontFamily });
}

export function updateDiffFontSize(fontSize: number) {
  return invoke<number>("update_diff_font_size", { fontSize });
}

export function updateTerminalFontFamily(fontFamily: string) {
  return invoke<string>("update_terminal_font_family", { fontFamily });
}

export function updateTerminalFontSize(fontSize: number) {
  return invoke<number>("update_terminal_font_size", { fontSize });
}

export function updateEditor(editor: string) {
  return invoke<string>("update_editor", { editor });
}

export function updateRefreshActiveHours(hours: RefreshActiveHours) {
  return invoke<RefreshActiveHours>("update_refresh_active_hours", { hours });
}

export function getWindowState() {
  return invoke<WindowState>("get_window_state");
}

export function updateWindowState(window: WindowState) {
  return invoke<WindowState>("update_window_state", { window });
}

export function replaceAppData(data: AppData) {
  return invoke<AppData>("replace_app_data", { data });
}

export function groupStatus(groupId: string, fetch = false) {
  return invoke<RepoStatus[]>("group_status", { groupId, fetch });
}

export function refreshRepo(groupId: string, repoId: string, fetch = true) {
  return invoke<RepoStatus>("refresh_repo", { groupId, repoId, fetch });
}

export function pullRepo(groupId: string, repoId: string, branch?: string) {
  return invoke<RepoActionResult>("pull_repo", {
    groupId,
    repoId,
    branch: branch?.trim() || null,
  });
}

export function pullCurrent(groupId: string) {
  return invoke<RepoActionResult[]>("pull_current", { groupId });
}

export function pullFromBranch(groupId: string) {
  return invoke<RepoActionResult[]>("pull_from_branch", { groupId });
}

export function checkoutRepo(
  groupId: string,
  repoId: string,
  target: string,
  fallbacks: string[],
) {
  return invoke<RepoActionResult>("checkout_repo", { groupId, repoId, target, fallbacks });
}

export function checkoutAll(groupId: string, target: string, fallbacks: string[]) {
  return invoke<RepoActionResult[]>("checkout_all", { groupId, target, fallbacks });
}

export function fileLog(path: string, file: string) {
  return invoke<CommitNode[]>("file_log", { path, file });
}

export function logGraph(path: string) {
  return invoke<CommitNode[]>("log_graph", { path });
}

export function workingTree(path: string) {
  return invoke<WorkingTreeFile[]>("working_tree", { path });
}

export function repoFiles(path: string) {
  return invoke<RepoFile[]>("repo_files", { path });
}

export function watchRepo(path: string) {
  return invoke<void>("watch_repo", { path });
}

export function unwatchRepo(path: string) {
  return invoke<void>("unwatch_repo", { path });
}

export function watchRepoGit(path: string) {
  return invoke<void>("watch_repo_git", { path });
}

export function unwatchRepoGit(path: string) {
  return invoke<void>("unwatch_repo_git", { path });
}

export function discardAllChanges(path: string) {
  return invoke<void>("discard_all_changes", { path });
}

export function discardFileChanges(path: string, file: string, staged: boolean) {
  return invoke<void>("discard_file_changes", { path, file, staged });
}

export function lastCommit(path: string) {
  return invoke<LastCommit>("last_commit", { path });
}

export function commit(path: string, title: string, description: string, amend = false) {
  return invoke<string>("commit", { path, title, description, amend });
}

export function abortOperation(path: string) {
  return invoke<string>("abort_operation", { path });
}

export function continueOperation(path: string) {
  return invoke<string>("continue_operation", { path });
}

export function openInEditor(path: string, file: string) {
  return invoke<void>("open_in_editor", { path, file });
}

export function repoRemoteUrl(path: string) {
  return invoke<string>("repo_remote_url", { path });
}

export function openRepoInFinder(path: string) {
  return invoke<void>("open_repo_in_finder", { path });
}

export function revealFileInFinder(path: string, file: string) {
  return invoke<void>("reveal_file_in_finder", { path, file });
}

export function ignoreWorkingTreePath(path: string, file: string, kind: string) {
  return invoke<void>("ignore_working_tree_path", { path, file, kind });
}

export function deleteWorkingTreeFile(path: string, file: string) {
  return invoke<void>("delete_working_tree_file", { path, file });
}

export function stageFile(path: string, file: string) {
  return invoke<void>("stage_file", { path, file });
}

export function stageAll(path: string) {
  return invoke<void>("stage_all", { path });
}

export function unstageFile(path: string, file: string) {
  return invoke<void>("unstage_file", { path, file });
}

export function unstageAll(path: string) {
  return invoke<void>("unstage_all", { path });
}

export function listLocalBranches(path: string) {
  return invoke<string[]>("list_local_branches", { path });
}

export function listBranchTracking(path: string) {
  return invoke<BranchTracking[]>("list_branch_tracking", { path });
}

export function branchOverview(path: string, preferred?: string, classify = true) {
  return invoke<BranchOverview>("branch_overview", {
    path,
    preferred: preferred?.trim() || null,
    classify,
  });
}

export function deleteLocalBranch(path: string, branch: string, force = false) {
  return invoke<string>("delete_local_branch", { path, branch, force });
}

export function deleteMergedBranches(
  path: string,
  preferred?: string,
  force = false,
  names?: string[],
) {
  return invoke<DeleteMergedResult>("delete_merged_branches", {
    path,
    preferred: preferred?.trim() || null,
    force,
    names: names ?? null,
  });
}

export function checkoutLocalBranch(path: string, branch: string) {
  return invoke<string>("checkout_local_branch", { path, branch });
}

export function createAndCheckoutBranch(path: string, branch: string, start = "") {
  return invoke<string>("create_and_checkout_branch", {
    path,
    branch,
    start: start.trim() || null,
  });
}

export function checkoutCommit(path: string, hash: string) {
  return invoke<string>("checkout_commit", { path, hash });
}

export function cherryPickCommits(path: string, hashes: string[]) {
  return invoke<string>("cherry_pick_commits", { path, hashes });
}

export function revertCommits(path: string, hashes: string[]) {
  return invoke<string>("revert_commits", { path, hashes });
}

export function commitRemoteUrl(path: string, hash: string) {
  return invoke<string>("commit_remote_url", { path, hash });
}

export function mergeLocalBranch(path: string, source: string, target: string) {
  return invoke<string>("merge_local_branch", { path, source, target });
}

/** Matches `git::NOT_FAST_FORWARD_PREFIX`: syncing would need a merge commit. */
export const NOT_FAST_FORWARD_PREFIX = "Not a fast-forward:";

/** Matches `git::LOCAL_BRANCH_EXISTS_PREFIX`: checkout needs a different local name. */
export const LOCAL_BRANCH_EXISTS_PREFIX = "Local branch exists:";

export function listRemotes(path: string) {
  return invoke<RemoteEntry[]>("list_remotes", { path });
}

export function fetchNamedRemote(path: string, remote: string) {
  return invoke<string>("fetch_named_remote", { path, remote });
}

export function addRemote(path: string, name: string, url: string) {
  return invoke<string>("add_remote", { path, name, url });
}

export function updateRemote(path: string, name: string, newName: string, url: string) {
  return invoke<string>("update_remote", { path, name, newName, url });
}

export function removeRemote(path: string, name: string) {
  return invoke<string>("remove_remote", { path, name });
}

export function remoteBranches(path: string, remote: string) {
  return invoke<RemoteOverview>("remote_branches", { path, remote });
}

export function checkoutRemoteBranch(
  path: string,
  remote: string,
  branch: string,
  localName?: string,
) {
  return invoke<string>("checkout_remote_branch", {
    path,
    remote,
    branch,
    localName: localName?.trim() || null,
  });
}

export function mergeRemoteBranch(
  path: string,
  remote: string,
  branch: string,
  target: string,
  allowMergeCommit: boolean,
) {
  return invoke<string>("merge_remote_branch", {
    path,
    remote,
    branch,
    target,
    allowMergeCommit,
  });
}

export function pushLocalBranch(path: string, branch: string) {
  return invoke<string>("push_local_branch", { path, branch });
}

export function deleteRemoteBranch(path: string, remote: string, branch: string) {
  return invoke<string>("delete_remote_branch", { path, remote, branch });
}

export function renameLocalBranch(path: string, branch: string, newName: string) {
  return invoke<string>("rename_local_branch", { path, branch, newName });
}

export function repoPull(path: string) {
  return invoke<string>("repo_pull", { path });
}

/** Matches `git::PUSH_REJECTED_PREFIX`: the remote has commits the local branch lacks. */
export const PUSH_REJECTED_PREFIX = "Push rejected:";

export function repoPush(path: string) {
  return invoke<string>("repo_push", { path });
}

export function resetUnpushedCommits(path: string) {
  return invoke<string>("reset_unpushed_commits", { path });
}

export function fileDiff(path: string, file: string, staged = false) {
  return invoke<string>("file_diff", { path, file, staged });
}

export function commitFiles(path: string, hash: string) {
  return invoke<CommitFile[]>("commit_files", { path, hash });
}

export function commitFileDiff(path: string, hash: string, file: string) {
  return invoke<string>("commit_file_diff", { path, hash, file });
}

export function fileBlame(
  path: string,
  file: string,
  rev?: string,
  staged = false,
  oldPath?: string,
) {
  return invoke<FileBlame>("file_blame", {
    path,
    file,
    rev: rev?.trim() || null,
    staged,
    oldPath: oldPath?.trim() || null,
  });
}

export function stashList(path: string) {
  return invoke<StashEntry[]>("stash_list", { path });
}

export function stashPush(path: string, message: string) {
  return invoke<string>("stash_push", { path, message });
}

export function stashFile(path: string, file: string) {
  return invoke<string>("stash_file", { path, file });
}

export function stashApply(path: string, index: number) {
  return invoke<string>("stash_apply", { path, index });
}

export function stashPop(path: string, index: number) {
  return invoke<string>("stash_pop", { path, index });
}

export function stashDrop(path: string, index: number) {
  return invoke<string>("stash_drop", { path, index });
}

export function tagList(path: string) {
  return invoke<TagEntry[]>("tag_list", { path });
}

export function createTag(path: string, name: string, message = "", target = "") {
  return invoke<string>("create_tag", {
    path,
    name,
    message,
    target: target.trim() || null,
  });
}

export function deleteTag(path: string, name: string) {
  return invoke<string>("delete_tag", { path, name });
}

export function writeTextFile(path: string, contents: string) {
  return invoke<void>("write_text_file", { path, contents });
}

export function readTextFile(path: string) {
  return invoke<string>("read_text_file", { path });
}

export function gitConfig() {
  return invoke<GitConfig>("git_config");
}

export function updateGitConfigValue(key: string, value: string) {
  return invoke<GitConfig>("update_git_config_value", { key, value });
}

export function saveGitConfigFile(contents: string) {
  return invoke<GitConfig>("save_git_config_file", { contents });
}

export function revealGitConfigFile() {
  return invoke<void>("reveal_git_config_file");
}

export function settingsFilePath() {
  return invoke<string>("settings_file_path");
}

export function revealSettingsFile() {
  return invoke<void>("reveal_settings_file");
}

export function commandHistory() {
  return invoke<CommandLogEntry[]>("command_history");
}

export function commandHistoryPaused() {
  return invoke<boolean>("command_history_paused");
}

export function setCommandHistoryPaused(paused: boolean) {
  return invoke<void>("set_command_history_paused", { paused });
}

export function clearCommandHistory() {
  return invoke<void>("clear_command_history");
}

export function openTerminal(path: string, cols: number, rows: number) {
  return invoke<string>("open_terminal", { path, cols, rows });
}

export function writeTerminal(id: string, data: string) {
  return invoke<void>("write_terminal", { id, data });
}

export function resizeTerminal(id: string, cols: number, rows: number) {
  return invoke<void>("resize_terminal", { id, cols, rows });
}

export function closeTerminal(id: string) {
  return invoke<void>("close_terminal", { id });
}
