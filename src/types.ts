export interface RepoEntry {
  id: string;
  path: string;
  label?: string;
  headerColor?: string;
}

export interface RepoGroup {
  id: string;
  name: string;
  expanded: boolean;
  pullFromBranch: string;
  checkoutFallbacks: string[];
  headerColor?: string;
  repos: RepoEntry[];
}

export type DiffMode = "inline" | "split";
export type NotificationMode = "background" | "always" | "off";

export const EDITOR_OPTIONS: { id: string; label: string; short?: string }[] = [
  { id: "system", label: "System default" },
  { id: "cursor", label: "Cursor" },
  { id: "phpstorm", label: "PhpStorm" },
  { id: "sublime", label: "Sublime Text", short: "Sublime" },
  { id: "vscode", label: "Visual Studio Code", short: "VS Code" },
  { id: "zed", label: "Zed" },
];

export type RefreshHoursPreset = "business" | "personal" | "custom";

export interface RefreshActiveHours {
  enabled: boolean;
  preset: RefreshHoursPreset;
  start: string;
  end: string;
}

export const STANDALONE_GROUP_ID = "standalone";

export interface WindowState {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized?: boolean;
}

export const MIN_WINDOW_WIDTH = 960;
export const MIN_WINDOW_HEIGHT = 640;
export const DEFAULT_WINDOW_WIDTH = 1280;
export const DEFAULT_WINDOW_HEIGHT = 800;

export interface AppData {
  groups: RepoGroup[];
  repos?: RepoEntry[];
  dashboardOrder?: string[];
  refreshIntervalSeconds?: number;
  filesPaneWidth?: number;
  terminalPaneHeight?: number;
  diffMode?: DiffMode;
  diffFontFamily?: string;
  diffFontSize?: number;
  terminalFontFamily?: string;
  terminalFontSize?: number;
  editor?: string;
  notifications?: NotificationMode;
  refreshActiveHours?: RefreshActiveHours;
  window?: WindowState;
}

export interface RepoStatus {
  id: string;
  path: string;
  name: string;
  branch: string;
  ahead: number;
  behind: number;
  dirty: boolean;
  insertions: number;
  deletions: number;
  changedFiles: number;
  conflictedFiles: number;
  operation: string;
}

export interface RepoActionResult {
  path: string;
  ok: boolean;
  message: string;
}

export interface LastCommit {
  title: string;
  description: string;
  published: boolean;
}

export interface CommitNode {
  hash: string;
  parents: string[];
  subject: string;
  author: string;
  date: string;
  refs: string;
  path?: string;
  oldPath?: string;
  status?: string;
}

export interface WorkingTreeFile {
  path: string;
  status: string;
  untracked: boolean;
  staged: boolean;
}

export interface RepoFile {
  path: string;
  ignored: boolean;
  directory: boolean;
}

export interface RepoFilesChanged {
  path: string;
  git: boolean;
}

export interface CommitFile {
  path: string;
  oldPath: string | null;
  status: string;
}

export interface BlameLine {
  line: number;
  hash: string;
  author: string;
  email: string;
  timestamp: number;
  summary: string;
}

export interface FileBlame {
  current: BlameLine[];
  previous: BlameLine[];
}

export interface LocalBranch {
  name: string;
  current: boolean;
  merged: boolean;
  partial: boolean;
  protected: boolean;
  pending: boolean;
}

export interface BranchTracking {
  name: string;
  localOnly: boolean;
  ahead: number;
  behind: number;
  upstream: string | null;
}

export interface DeleteMergedResult {
  deleted: string[];
  refused: string[];
  errors: string[];
  message: string;
}

export interface BranchOverview {
  mergeTarget: string | null;
  branches: LocalBranch[];
}

export interface RemoteEntry {
  name: string;
  fetchUrl: string;
  pushUrl: string;
  browseUrl: string | null;
  branchCount: number;
}

export interface RemoteBranch {
  name: string;
  remote: string;
  hash: string;
  date: string;
  subject: string;
  isDefault: boolean;
  local: string | null;
  tracked: boolean;
  current: boolean;
  ahead: number;
  behind: number;
}

export interface RemoteOverview {
  remote: string;
  defaultBranch: string | null;
  branches: RemoteBranch[];
}

export interface StashEntry {
  index: number;
  hash: string;
  message: string;
  date: string;
}

export interface TagEntry {
  name: string;
  hash: string;
  date: string;
  message: string;
  annotated: boolean;
}

export interface GitConfig {
  path: string;
  contents: string;
  userName: string;
  userEmail: string;
  defaultBranch: string;
  pullRebase: string;
  defaultRemote: string;
}

export interface CommandLogEntry {
  id: string;
  at: number;
  cwd: string;
  program: string;
  args: string[];
  command: string;
  success: boolean;
  durationMs: number;
  stdout: string;
  stderr: string;
}
