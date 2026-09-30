use serde::{Deserialize, Serialize};

fn default_refresh_interval() -> u64 {
    300
}

fn default_files_pane_width() -> u32 {
    320
}

fn default_terminal_pane_height() -> u32 {
    280
}

fn default_diff_mode() -> String {
    "split".into()
}

fn default_editor() -> String {
    "system".into()
}

fn default_code_font() -> String {
    "jetbrains".into()
}

fn default_diff_font_size() -> f64 {
    13.0
}

fn default_terminal_font_size() -> f64 {
    14.0
}

pub fn sanitize_font_family(value: &str) -> String {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 80
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ';' | '{' | '}'))
    {
        return default_code_font();
    }
    match value.to_ascii_lowercase().as_str() {
        "jetbrains" | "jetbrains mono" => "jetbrains".into(),
        "system" | "system mono" | "default" | "ui-monospace" => "system".into(),
        "sf-mono" | "sf mono" | "sfmono" => "sf-mono".into(),
        "menlo" => "menlo".into(),
        "monaco" => "monaco".into(),
        "courier" | "courier new" => "courier".into(),
        _ => value.replace(['\'', '"'], ""),
    }
}

pub fn sanitize_font_size(value: f64, default: f64) -> f64 {
    if !value.is_finite() {
        return default;
    }
    let clamped = value.clamp(9.0, 22.0);
    (clamped * 2.0).round() / 2.0
}

pub fn sanitize_editor(value: &str) -> String {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 80
        || value.chars().any(|c| c.is_control() || c == '/' || c == '\\' || c == '\0')
    {
        return default_editor();
    }
    match value.to_ascii_lowercase().as_str() {
        "system" | "default" => "system".into(),
        "cursor" => "cursor".into(),
        "vscode" | "code" | "visual studio code" => "vscode".into(),
        "phpstorm" => "phpstorm".into(),
        "webstorm" => "webstorm".into(),
        "intellij" | "intellij idea" => "intellij".into(),
        "sublime" | "sublime text" => "sublime".into(),
        "nova" => "nova".into(),
        "zed" => "zed".into(),
        "textedit" | "text edit" => "textedit".into(),
        _ => value.to_string(),
    }
}

pub const BUSINESS_REFRESH_HOURS_START: &str = "08:00";
pub const BUSINESS_REFRESH_HOURS_END: &str = "18:00";
pub const PERSONAL_REFRESH_HOURS_START: &str = "06:00";
pub const PERSONAL_REFRESH_HOURS_END: &str = "23:00";

fn default_refresh_hours_preset() -> String {
    "business".into()
}

fn default_refresh_hours_start() -> String {
    BUSINESS_REFRESH_HOURS_START.into()
}

fn default_refresh_hours_end() -> String {
    BUSINESS_REFRESH_HOURS_END.into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshActiveHours {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_refresh_hours_preset")]
    pub preset: String,
    #[serde(default = "default_refresh_hours_start")]
    pub start: String,
    #[serde(default = "default_refresh_hours_end")]
    pub end: String,
}

impl Default for RefreshActiveHours {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: default_refresh_hours_preset(),
            start: default_refresh_hours_start(),
            end: default_refresh_hours_end(),
        }
    }
}

pub fn sanitize_clock(value: &str) -> Option<String> {
    let value = value.trim();
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<u32>().ok()?;
    let minute = parts.next()?.parse::<u32>().ok()?;
    if let Some(seconds) = parts.next() {
        let seconds = seconds
            .split('.')
            .next()?
            .parse::<u32>()
            .ok()?;
        if seconds > 59 {
            return None;
        }
    }
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    Some(format!("{hour:02}:{minute:02}"))
}

pub fn sanitize_refresh_active_hours(hours: RefreshActiveHours) -> RefreshActiveHours {
    let preset = match hours.preset.trim() {
        "personal" => "personal",
        "custom" => "custom",
        _ => "business",
    };
    let (start, end) = match preset {
        "personal" => (
            PERSONAL_REFRESH_HOURS_START.to_string(),
            PERSONAL_REFRESH_HOURS_END.to_string(),
        ),
        "custom" => (
            sanitize_clock(&hours.start)
                .unwrap_or_else(|| BUSINESS_REFRESH_HOURS_START.to_string()),
            sanitize_clock(&hours.end).unwrap_or_else(|| BUSINESS_REFRESH_HOURS_END.to_string()),
        ),
        _ => (
            BUSINESS_REFRESH_HOURS_START.to_string(),
            BUSINESS_REFRESH_HOURS_END.to_string(),
        ),
    };
    RefreshActiveHours {
        enabled: hours.enabled,
        preset: preset.into(),
        start,
        end,
    }
}

pub const MIN_WINDOW_WIDTH: u32 = 960;
pub const MIN_WINDOW_HEIGHT: u32 = 640;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub maximized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub groups: Vec<RepoGroup>,
    #[serde(default)]
    pub repos: Vec<RepoEntry>,
    /// Group ids and ungrouped repository ids, in dashboard order.
    #[serde(default)]
    pub dashboard_order: Vec<String>,
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_seconds: u64,
    #[serde(default = "default_files_pane_width")]
    pub files_pane_width: u32,
    #[serde(default = "default_terminal_pane_height")]
    pub terminal_pane_height: u32,
    #[serde(default = "default_diff_mode")]
    pub diff_mode: String,
    #[serde(default = "default_code_font")]
    pub diff_font_family: String,
    #[serde(default = "default_diff_font_size")]
    pub diff_font_size: f64,
    #[serde(default = "default_code_font")]
    pub terminal_font_family: String,
    #[serde(default = "default_terminal_font_size")]
    pub terminal_font_size: f64,
    #[serde(default = "default_editor")]
    pub editor: String,
    #[serde(default)]
    pub refresh_active_hours: RefreshActiveHours,
    #[serde(default)]
    pub window: Option<WindowState>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            repos: Vec::new(),
            dashboard_order: Vec::new(),
            refresh_interval_seconds: default_refresh_interval(),
            files_pane_width: default_files_pane_width(),
            terminal_pane_height: default_terminal_pane_height(),
            diff_mode: default_diff_mode(),
            diff_font_family: default_code_font(),
            diff_font_size: default_diff_font_size(),
            terminal_font_family: default_code_font(),
            terminal_font_size: default_terminal_font_size(),
            editor: default_editor(),
            refresh_active_hours: RefreshActiveHours::default(),
            window: None,
        }
    }
}

impl AppData {
    /**
     * The saved dashboard order without stale ids. Anything missing from it
     * follows, ungrouped repositories first, matching the layout before
     * groups and repositories could be interleaved.
     */
    pub fn dashboard_ids(&self) -> Vec<String> {
        let known: Vec<&str> = self
            .repos
            .iter()
            .map(|repo| repo.id.as_str())
            .chain(self.groups.iter().map(|group| group.id.as_str()))
            .collect();
        let mut ids: Vec<String> = Vec::with_capacity(known.len());
        for id in self.dashboard_order.iter().map(String::as_str).chain(known.iter().copied()) {
            if known.contains(&id) && !ids.iter().any(|item| item == id) {
                ids.push(id.to_string());
            }
        }
        ids
    }
}

fn default_header_color() -> String {
    "#16323c".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoGroup {
    pub id: String,
    pub name: String,
    pub expanded: bool,
    pub pull_from_branch: String,
    pub checkout_fallbacks: Vec<String>,
    #[serde(default = "default_header_color")]
    pub header_color: String,
    pub repos: Vec<RepoEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_groups_as_camel_case_json() {
        let data = AppData {
            refresh_interval_seconds: 300,
            files_pane_width: 320,
            terminal_pane_height: 280,
            diff_mode: "split".into(),
            diff_font_family: "jetbrains".into(),
            diff_font_size: 13.0,
            terminal_font_family: "jetbrains".into(),
            terminal_font_size: 14.0,
            editor: "system".into(),
            refresh_active_hours: RefreshActiveHours::default(),
            window: None,
            repos: Vec::new(),
            dashboard_order: Vec::new(),
            groups: vec![RepoGroup {
                id: "g1".into(),
                name: "Work".into(),
                expanded: true,
                pull_from_branch: "develop".into(),
                checkout_fallbacks: vec!["develop".into()],
                header_color: "#16323c".into(),
                repos: vec![RepoEntry {
                    id: "r1".into(),
                    path: "/tmp/api".into(),
                    label: String::new(),
                    header_color: String::new(),
                }],
            }],
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"pullFromBranch\":\"develop\""));
        assert!(json.contains("\"checkoutFallbacks\""));
        let parsed: AppData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.groups[0].repos[0].path, "/tmp/api");
        assert!(!parsed.refresh_active_hours.enabled);
        assert_eq!(parsed.refresh_active_hours.preset, "business");
        assert_eq!(parsed.refresh_active_hours.start, "08:00");
        assert_eq!(parsed.refresh_active_hours.end, "18:00");
        assert_eq!(parsed.editor, "system");
        assert_eq!(parsed.diff_font_family, "jetbrains");
        assert_eq!(parsed.diff_font_size, 13.0);
        assert_eq!(parsed.terminal_font_family, "jetbrains");
        assert_eq!(parsed.terminal_font_size, 14.0);
    }

    #[test]
    fn sanitizes_font_family_aliases_and_rejects_paths() {
        assert_eq!(sanitize_font_family("JetBrains Mono"), "jetbrains");
        assert_eq!(sanitize_font_family("SF Mono"), "sf-mono");
        assert_eq!(sanitize_font_family("Fira Code"), "Fira Code");
        assert_eq!(sanitize_font_family(""), "jetbrains");
        assert_eq!(sanitize_font_family("/System/Library/Fonts/Menlo.ttc"), "jetbrains");
    }

    #[test]
    fn sanitizes_font_size_range() {
        assert_eq!(sanitize_font_size(12.4, 12.0), 12.5);
        assert_eq!(sanitize_font_size(8.0, 12.0), 9.0);
        assert_eq!(sanitize_font_size(40.0, 12.5), 22.0);
        assert_eq!(sanitize_font_size(f64::NAN, 12.5), 12.5);
    }

    #[test]
    fn sanitizes_editor_aliases_and_rejects_paths() {
        assert_eq!(sanitize_editor("Cursor"), "cursor");
        assert_eq!(sanitize_editor("visual studio code"), "vscode");
        assert_eq!(sanitize_editor("BBEdit"), "BBEdit");
        assert_eq!(sanitize_editor(""), "system");
        assert_eq!(sanitize_editor("/Applications/Cursor.app"), "system");
    }

    #[test]
    fn missing_refresh_active_hours_uses_defaults() {
        let parsed: AppData = serde_json::from_str(
            r#"{"groups":[],"repos":[],"refreshIntervalSeconds":300,"filesPaneWidth":320,"diffMode":"split"}"#,
        )
        .unwrap();
        assert_eq!(parsed.refresh_active_hours, RefreshActiveHours::default());
        assert_eq!(parsed.terminal_pane_height, 280);
        assert_eq!(parsed.diff_font_family, "jetbrains");
        assert_eq!(parsed.diff_font_size, 13.0);
        assert_eq!(parsed.terminal_font_family, "jetbrains");
        assert_eq!(parsed.terminal_font_size, 14.0);
    }

    #[test]
    fn sanitizes_refresh_active_hours_presets_and_clocks() {
        let personal = sanitize_refresh_active_hours(RefreshActiveHours {
            enabled: true,
            preset: "personal".into(),
            start: "01:00".into(),
            end: "02:00".into(),
        });
        assert_eq!(personal.start, "06:00");
        assert_eq!(personal.end, "23:00");

        let custom = sanitize_refresh_active_hours(RefreshActiveHours {
            enabled: true,
            preset: "custom".into(),
            start: "9:5".into(),
            end: "22:30".into(),
        });
        assert_eq!(custom.start, "09:05");
        assert_eq!(custom.end, "22:30");

        let invalid = sanitize_refresh_active_hours(RefreshActiveHours {
            enabled: false,
            preset: "custom".into(),
            start: "25:00".into(),
            end: "nope".into(),
        });
        assert_eq!(invalid.start, "08:00");
        assert_eq!(invalid.end, "18:00");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoEntry {
    pub id: String,
    pub path: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub header_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatus {
    pub id: String,
    pub path: String,
    pub name: String,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
    pub insertions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    #[serde(default)]
    pub conflicted_files: u32,
    #[serde(default)]
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoActionResult {
    pub path: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastCommit {
    pub title: String,
    pub description: String,
    pub published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitNode {
    pub hash: String,
    pub parents: Vec<String>,
    pub subject: String,
    pub author: String,
    pub date: String,
    pub refs: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoFile {
    pub path: String,
    pub ignored: bool,
    pub directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingTreeFile {
    pub path: String,
    pub status: String,
    pub untracked: bool,
    #[serde(default)]
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameLine {
    pub line: u32,
    pub hash: String,
    pub author: String,
    pub email: String,
    pub timestamp: u64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileBlame {
    pub current: Vec<BlameLine>,
    pub previous: Vec<BlameLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalBranch {
    pub name: String,
    pub current: bool,
    pub merged: bool,
    pub partial: bool,
    #[serde(rename = "protected")]
    pub protected_branch: bool,
    #[serde(default)]
    pub pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchTracking {
    pub name: String,
    pub local_only: bool,
    pub ahead: u32,
    pub behind: u32,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMergedResult {
    pub deleted: Vec<String>,
    pub refused: Vec<String>,
    pub errors: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchOverview {
    pub merge_target: Option<String>,
    pub branches: Vec<LocalBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEntry {
    pub name: String,
    pub fetch_url: String,
    pub push_url: String,
    pub browse_url: Option<String>,
    pub branch_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBranch {
    pub name: String,
    pub remote: String,
    pub hash: String,
    pub date: String,
    pub subject: String,
    pub is_default: bool,
    /// Local branch that tracks this one, or a local branch with the same name.
    pub local: Option<String>,
    /// True when `local` has this branch configured as its upstream.
    pub tracked: bool,
    pub current: bool,
    /// Commits on `local` that are not on the remote branch.
    pub ahead: u32,
    /// Commits on the remote branch that are not on `local`.
    pub behind: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteOverview {
    pub remote: String,
    pub default_branch: Option<String>,
    pub branches: Vec<RemoteBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StashEntry {
    pub index: u32,
    pub hash: String,
    pub message: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagEntry {
    pub name: String,
    pub hash: String,
    pub date: String,
    pub message: String,
    pub annotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitConfig {
    pub path: String,
    pub contents: String,
    pub user_name: String,
    pub user_email: String,
    pub default_branch: String,
    pub pull_rebase: String,
    pub default_remote: String,
}
