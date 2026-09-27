import { computed, nextTick, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";
import type {
  AppData,
  DiffMode,
  RefreshActiveHours,
  RepoActionResult,
  RepoEntry,
  RepoFilesChanged,
  RepoGroup,
  RepoStatus,
  WindowState,
} from "../types";
import { STANDALONE_GROUP_ID } from "../types";
import {
  DEFAULT_CODE_FONT,
  DEFAULT_DIFF_FONT_SIZE,
  DEFAULT_TERMINAL_FONT_SIZE,
  clampFontSize,
  sanitizeFontFamily,
} from "../fonts";
import {
  DEFAULT_REFRESH_ACTIVE_HOURS,
  formatClockLabel,
  isWithinActiveHours,
  nextActiveHoursStart,
  normalizeRefreshActiveHours,
  resolvedRefreshHours,
} from "../refreshHours";

const groups = ref<RepoGroup[]>([]);
const standaloneRepos = ref<RepoEntry[]>([]);
const statuses = ref<Record<string, RepoStatus>>({});
const results = ref<Record<string, RepoActionResult[]>>({});
const busy = ref<Record<string, string>>({});
const error = ref("");
const loaded = ref(false);
const refreshIntervalSeconds = ref(300);
const refreshActiveHours = ref<RefreshActiveHours>({ ...DEFAULT_REFRESH_ACTIVE_HOURS });
const filesPaneWidth = ref(320);
const terminalPaneHeight = ref(280);
const diffMode = ref<DiffMode>("split");
const diffFontFamily = ref(DEFAULT_CODE_FONT);
const diffFontSize = ref(DEFAULT_DIFF_FONT_SIZE);
const terminalFontFamily = ref(DEFAULT_CODE_FONT);
const terminalFontSize = ref(DEFAULT_TERMINAL_FONT_SIZE);
const editor = ref("system");
const windowState = ref<WindowState | null>(null);
const FILES_PANE_MIN = 220;
const FILES_PANE_MAX = 800;
const TERMINAL_PANE_MIN = 160;
const TERMINAL_PANE_MAX = 720;
const refreshingAll = ref(false);
const lastRefreshAt = ref<Date | null>(null);
const nextRefreshAt = ref<number | null>(null);
const nowTick = ref(Date.now());
const refreshingRepos = ref<Record<string, boolean>>({});
const refreshingGroups = ref<Record<string, boolean>>({});
const refreshTotal = ref(0);
const refreshDone = ref(0);
const refreshCancelled = ref(false);
const refreshProgress = ref<Record<string, string>>({});
const toastMessage = ref("");
const toastKind = ref<"success" | "error">("success");
const toastHasDetails = ref(false);
const actionOutput = ref<{ title: string; results: RepoActionResult[] } | null>(null);
const actionOutputOpen = ref(false);
const pullingAll = ref(false);
const pullAllCancelled = ref(false);
const pullAllTotal = ref(0);
const pullAllDone = ref(0);
const pullProgress = ref<Record<string, string>>({});
const pullBranchByGroup = ref<Record<string, string>>({});
const pullCancelled = ref<Record<string, boolean>>({});
const checkoutProgress = ref<Record<string, string>>({});
const checkoutCancelled = ref<Record<string, boolean>>({});
let toastTimer: ReturnType<typeof setTimeout> | null = null;
let autoRefreshTimer: ReturnType<typeof setTimeout> | null = null;
let tickTimer: ReturnType<typeof setInterval> | null = null;
let autoRefreshStarted = false;
const gitWatches = new Set<string>();
let gitWatchSync: Promise<void> = Promise.resolve();
const statusInFlight = new Map<string, Promise<void>>();
const statusQueued = new Set<string>();

function repoWatchKey(path: string) {
  return path.trim().replace(/\/+$/, "");
}

const statusById = computed(() => statuses.value);

export function useApp() {
  async function load() {
    error.value = "";
    try {
      applyState(await api.getState());
      loaded.value = true;
      startAutoRefresh();
      await Promise.all([
        ...groups.value.filter((group) => group.expanded).map((group) => refreshStatus(group.id)),
        refreshStatus(STANDALONE_GROUP_ID),
      ]);
    } catch (err) {
      error.value = String(err);
    }
  }

  function applyState(data: AppData) {
    groups.value = data.groups;
    standaloneRepos.value = data.repos ?? [];
    refreshIntervalSeconds.value = data.refreshIntervalSeconds ?? 300;
    refreshActiveHours.value = normalizeRefreshActiveHours(data.refreshActiveHours);
    filesPaneWidth.value = clampFilesPaneWidth(data.filesPaneWidth ?? 320);
    terminalPaneHeight.value = clampTerminalPaneHeight(data.terminalPaneHeight ?? 280);
    diffMode.value = data.diffMode === "inline" ? "inline" : "split";
    diffFontFamily.value = sanitizeFontFamily(data.diffFontFamily ?? DEFAULT_CODE_FONT);
    diffFontSize.value = clampFontSize(data.diffFontSize ?? DEFAULT_DIFF_FONT_SIZE, DEFAULT_DIFF_FONT_SIZE);
    terminalFontFamily.value = sanitizeFontFamily(data.terminalFontFamily ?? DEFAULT_CODE_FONT);
    terminalFontSize.value = clampFontSize(
      data.terminalFontSize ?? DEFAULT_TERMINAL_FONT_SIZE,
      DEFAULT_TERMINAL_FONT_SIZE,
    );
    editor.value = data.editor?.trim() || "system";
    windowState.value = data.window ?? null;
  }

  function applyStatus(status: RepoStatus) {
    statuses.value = { ...statuses.value, [status.id]: status };
    lastRefreshAt.value = new Date();
  }

  function patchRepoStatus(repoId: string, patch: Partial<RepoStatus>) {
    const current = statuses.value[repoId];
    if (!current) {
      return;
    }
    applyStatus({ ...current, ...patch });
  }

  /**
   * The open repo tab and the dashboard watcher both ask for a status after
   * the same git change. Share one in-flight read and rerun once if another
   * request lands mid-read.
   */
  function refreshRepoStatus(groupId: string, repoId: string) {
    const key = `${groupId}:${repoId}`;
    const running = statusInFlight.get(key);
    if (running) {
      statusQueued.add(key);
      return running;
    }
    const run = (async () => {
      do {
        statusQueued.delete(key);
        try {
          applyStatus(await api.refreshRepo(groupId, repoId, false));
        } catch (err) {
          error.value = String(err);
        }
      } while (statusQueued.has(key));
    })().finally(() => {
      statusInFlight.delete(key);
    });
    statusInFlight.set(key, run);
    return run;
  }

  /** Repos whose status the dashboard shows, keyed by watch path. */
  function dashboardWatchTargets() {
    const targets = new Map<string, { groupId: string; repoId: string }[]>();
    const add = (groupId: string, repo: RepoEntry) => {
      const key = repoWatchKey(repo.path);
      if (!key) {
        return;
      }
      targets.set(key, [...(targets.get(key) ?? []), { groupId, repoId: repo.id }]);
    };
    for (const group of groups.value) {
      if (group.expanded) {
        group.repos.forEach((repo) => add(group.id, repo));
      }
    }
    standaloneRepos.value.forEach((repo) => add(STANDALONE_GROUP_ID, repo));
    return targets;
  }

  function syncGitWatches() {
    gitWatchSync = gitWatchSync.then(async () => {
      const wanted = new Set(dashboardWatchTargets().keys());
      for (const path of [...gitWatches]) {
        if (!wanted.has(path)) {
          gitWatches.delete(path);
          await api.unwatchRepoGit(path).catch(() => undefined);
        }
      }
      for (const path of wanted) {
        if (gitWatches.has(path)) {
          continue;
        }
        try {
          await api.watchRepoGit(path);
          gitWatches.add(path);
        } catch {
          /* missing or not a repo; retried on the next sync */
        }
      }
    });
  }

  function onRepoGitChanged(payload: RepoFilesChanged) {
    const targets = dashboardWatchTargets().get(repoWatchKey(payload.path)) ?? [];
    for (const { groupId, repoId } of targets) {
      if (refreshingRepos.value[repoId]) {
        continue;
      }
      void refreshRepoStatus(groupId, repoId);
    }
  }

  async function refreshStatus(groupId: string, fetch = false) {
    try {
      const list =
        groupId === STANDALONE_GROUP_ID
          ? await api.standaloneStatus(fetch)
          : await api.groupStatus(groupId, fetch);
      const next = { ...statuses.value };
      for (const status of list) {
        next[status.id] = status;
      }
      statuses.value = next;
      lastRefreshAt.value = new Date();
    } catch (err) {
      error.value = String(err);
    }
  }

  function cancelRefresh() {
    refreshCancelled.value = true;
  }

  function cancelPull(groupId: string) {
    pullCancelled.value = { ...pullCancelled.value, [groupId]: true };
  }

  function cancelPullAll() {
    pullAllCancelled.value = true;
  }

  function cancelCheckout(groupId: string) {
    checkoutCancelled.value = { ...checkoutCancelled.value, [groupId]: true };
  }

  function dismissToast() {
    if (toastTimer) {
      clearTimeout(toastTimer);
      toastTimer = null;
    }
    toastMessage.value = "";
  }

  function showToast(
    message: string,
    kind: "success" | "error" = "success",
    hasDetails = false,
  ) {
    dismissToast();
    toastKind.value = kind;
    toastHasDetails.value = hasDetails;
    toastMessage.value = message;
    toastTimer = setTimeout(
      () => {
        toastMessage.value = "";
        toastTimer = null;
      },
      kind === "error" || hasDetails ? 5600 : 3200,
    );
  }

  function dismissOutput() {
    actionOutputOpen.value = false;
  }

  function openOutput() {
    if (actionOutput.value) {
      actionOutputOpen.value = true;
    }
  }

  function hasDetailedOutput(outcomes: RepoActionResult[]) {
    return outcomes.some((item) => item.message.includes("\n") || item.message.length > 96);
  }

  function presentActionResults(
    title: string,
    outcomes: RepoActionResult[],
    messages: { success: string; error: string },
  ) {
    actionOutput.value = { title, results: outcomes };
    const failed = outcomes.filter((item) => !item.ok).length;
    if (failed) {
      actionOutputOpen.value = true;
      showToast(messages.error, "error");
      return;
    }
    showToast(messages.success, "success", hasDetailedOutput(outcomes));
  }

  function refreshDoneMessage(count: number, groupName?: string) {
    const repos = count === 1 ? "1 repository" : `${count} repositories`;
    return groupName
      ? `Fetched ${groupName} (${repos}).`
      : `Fetch complete. Updated ${repos}.`;
  }

  type RefreshJob = { groupId: string; repoId: string };

  // Fetch is network-bound; a small pool feels like GitKraken without
  // launching every `git fetch` at once.
  const REFRESH_CONCURRENCY = 8;

  async function runPool<T>(
    items: T[],
    worker: (item: T) => Promise<void>,
    cancelled: () => boolean = () => refreshCancelled.value,
  ) {
    if (!items.length) {
      return;
    }
    let next = 0;
    const workers = Math.min(REFRESH_CONCURRENCY, items.length);
    await Promise.all(
      Array.from({ length: workers }, async () => {
        while (!cancelled()) {
          const index = next++;
          if (index >= items.length) {
            return;
          }
          await worker(items[index]);
        }
      }),
    );
  }

  function interleaveRefreshJobs(jobs: RefreshJob[]) {
    const buckets = new Map<string, RefreshJob[]>();
    for (const job of jobs) {
      const list = buckets.get(job.groupId) ?? [];
      list.push(job);
      buckets.set(job.groupId, list);
    }
    const queues = [...buckets.values()];
    const ordered: RefreshJob[] = [];
    let added = true;
    while (added) {
      added = false;
      for (const queue of queues) {
        const job = queue.shift();
        if (job) {
          ordered.push(job);
          added = true;
        }
      }
    }
    return ordered;
  }

  function markReposRefreshing(repoIds: string[]) {
    refreshingRepos.value = {
      ...refreshingRepos.value,
      ...Object.fromEntries(repoIds.map((id) => [id, true])),
    };
  }

  function unmarkRepoRefreshing(repoId: string) {
    if (!refreshingRepos.value[repoId]) {
      return;
    }
    const next = { ...refreshingRepos.value };
    delete next[repoId];
    refreshingRepos.value = next;
  }

  function clearReposRefreshing(repoIds: string[]) {
    if (!repoIds.length) {
      return;
    }
    const next = { ...refreshingRepos.value };
    for (const id of repoIds) {
      delete next[id];
    }
    refreshingRepos.value = next;
  }

  async function refreshRepoJobs(jobs: RefreshJob[], groupTotals: Record<string, number>) {
    const completed: Record<string, number> = {};
    const progress = { ...refreshProgress.value };
    for (const [groupId, total] of Object.entries(groupTotals)) {
      completed[groupId] = 0;
      progress[groupId] = `0/${total}`;
    }
    refreshProgress.value = progress;
    markReposRefreshing(jobs.map((job) => job.repoId));
    await nextTick();

    const bumpProgress = (groupId: string) => {
      completed[groupId] = (completed[groupId] ?? 0) + 1;
      const total = groupTotals[groupId];
      if (total) {
        refreshProgress.value = {
          ...refreshProgress.value,
          [groupId]: `${completed[groupId]}/${total}`,
        };
      }
      if (refreshTotal.value > 0) {
        refreshDone.value += 1;
      }
    };

    try {
      await runPool(jobs, async (job) => {
        if (refreshCancelled.value) {
          unmarkRepoRefreshing(job.repoId);
          return;
        }
        try {
          applyStatus(await api.refreshRepo(job.groupId, job.repoId, true));
        } catch (err) {
          error.value = String(err);
        } finally {
          unmarkRepoRefreshing(job.repoId);
          bumpProgress(job.groupId);
        }
      });
    } finally {
      clearReposRefreshing(jobs.map((job) => job.repoId));
    }
  }

  async function refreshGroup(groupId: string) {
    const group = groups.value.find((item) => item.id === groupId);
    if (!group) {
      return;
    }
    refreshingGroups.value = { ...refreshingGroups.value, [groupId]: true };
    busy.value = { ...busy.value, [groupId]: "Fetching…" };
    error.value = "";
    const notify = !refreshingAll.value;
    if (!refreshingAll.value) {
      refreshCancelled.value = false;
      refreshDone.value = 0;
      refreshTotal.value = group.repos.length;
    }
    try {
      await refreshRepoJobs(
        group.repos.map((repo) => ({ groupId, repoId: repo.id })),
        { [groupId]: group.repos.length },
      );
      if (notify && !refreshCancelled.value) {
        showToast(refreshDoneMessage(group.repos.length, group.name));
      }
    } finally {
      const groupsBusy = { ...refreshingGroups.value };
      delete groupsBusy[groupId];
      refreshingGroups.value = groupsBusy;
      const next = { ...busy.value };
      delete next[groupId];
      busy.value = next;
      const progress = { ...refreshProgress.value };
      delete progress[groupId];
      refreshProgress.value = progress;
      if (!refreshingAll.value) {
        refreshCancelled.value = false;
        refreshDone.value = 0;
        refreshTotal.value = 0;
      }
    }
  }

  async function refreshAll(options?: { notify?: boolean }) {
    const standaloneCount = standaloneRepos.value.length;
    const groupedCount = groups.value.reduce((sum, group) => sum + group.repos.length, 0);
    if (refreshingAll.value || pullingAll.value || !(standaloneCount || groupedCount)) {
      return;
    }
    const notify = options?.notify ?? true;
    refreshingAll.value = true;
    refreshCancelled.value = false;
    error.value = "";
    refreshDone.value = 0;
    refreshTotal.value = standaloneCount + groupedCount;
    const groupTotals: Record<string, number> = {};
    if (standaloneCount) {
      groupTotals[STANDALONE_GROUP_ID] = standaloneCount;
    }
    const groupsBusy = { ...refreshingGroups.value };
    const progress = { ...refreshProgress.value };
    for (const group of groups.value) {
      if (!group.repos.length) {
        continue;
      }
      groupTotals[group.id] = group.repos.length;
      groupsBusy[group.id] = true;
      progress[group.id] = `0/${group.repos.length}`;
    }
    refreshingGroups.value = groupsBusy;
    refreshProgress.value = progress;
    if (autoRefreshTimer) {
      clearTimeout(autoRefreshTimer);
      autoRefreshTimer = null;
    }
    try {
      const jobs = interleaveRefreshJobs([
        ...standaloneRepos.value.map((repo) => ({
          groupId: STANDALONE_GROUP_ID,
          repoId: repo.id,
        })),
        ...groups.value.flatMap((group) =>
          group.repos.map((repo) => ({ groupId: group.id, repoId: repo.id })),
        ),
      ]);
      await refreshRepoJobs(jobs, groupTotals);
      if (notify && !refreshCancelled.value) {
        showToast(refreshDoneMessage(refreshTotal.value));
      }
    } finally {
      refreshingAll.value = false;
      refreshCancelled.value = false;
      refreshDone.value = 0;
      refreshTotal.value = 0;
      const leftoverGroups = { ...refreshingGroups.value };
      const leftoverProgress = { ...refreshProgress.value };
      for (const groupId of Object.keys(groupTotals)) {
        delete leftoverGroups[groupId];
        delete leftoverProgress[groupId];
      }
      refreshingGroups.value = leftoverGroups;
      refreshProgress.value = leftoverProgress;
      startAutoRefresh();
    }
  }

  function stopAutoRefresh() {
    if (autoRefreshTimer) {
      clearTimeout(autoRefreshTimer);
      autoRefreshTimer = null;
    }
    nextRefreshAt.value = null;
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    const seconds = refreshIntervalSeconds.value;
    if (seconds <= 0) {
      return;
    }
    const now = new Date();
    const sleeping =
      refreshActiveHours.value.enabled && !isWithinActiveHours(now, refreshActiveHours.value);
    const delayMs = sleeping
      ? Math.max(0, nextActiveHoursStart(now, refreshActiveHours.value).getTime() - now.getTime())
      : seconds * 1000;
    nextRefreshAt.value = Date.now() + delayMs;
    autoRefreshTimer = setTimeout(() => {
      if (
        refreshActiveHours.value.enabled &&
        !isWithinActiveHours(new Date(), refreshActiveHours.value)
      ) {
        startAutoRefresh();
        return;
      }
      void refreshAll({ notify: false });
    }, delayMs);
  }

  function clampFilesPaneWidth(width: number) {
    return Math.round(Math.min(FILES_PANE_MAX, Math.max(FILES_PANE_MIN, width)));
  }

  function setFilesPaneWidth(width: number) {
    filesPaneWidth.value = clampFilesPaneWidth(width);
  }

  async function saveFilesPaneWidth(width: number) {
    filesPaneWidth.value = await api.updateFilesPaneWidth(clampFilesPaneWidth(width));
  }

  function clampTerminalPaneHeight(height: number) {
    return Math.round(Math.min(TERMINAL_PANE_MAX, Math.max(TERMINAL_PANE_MIN, height)));
  }

  function setTerminalPaneHeight(height: number) {
    terminalPaneHeight.value = clampTerminalPaneHeight(height);
  }

  async function saveTerminalPaneHeight(height: number) {
    terminalPaneHeight.value = await api.updateTerminalPaneHeight(clampTerminalPaneHeight(height));
  }

  async function saveDiffMode(mode: DiffMode) {
    diffMode.value = await api.updateDiffMode(mode);
  }

  function setDiffFontSize(size: number) {
    diffFontSize.value = clampFontSize(size, DEFAULT_DIFF_FONT_SIZE);
  }

  async function saveDiffFontSize(size: number) {
    diffFontSize.value = await api.updateDiffFontSize(clampFontSize(size, DEFAULT_DIFF_FONT_SIZE));
  }

  async function saveDiffFontFamily(family: string) {
    diffFontFamily.value = await api.updateDiffFontFamily(sanitizeFontFamily(family));
  }

  function setTerminalFontSize(size: number) {
    terminalFontSize.value = clampFontSize(size, DEFAULT_TERMINAL_FONT_SIZE);
  }

  async function saveTerminalFontSize(size: number) {
    terminalFontSize.value = await api.updateTerminalFontSize(
      clampFontSize(size, DEFAULT_TERMINAL_FONT_SIZE),
    );
  }

  async function saveTerminalFontFamily(family: string) {
    terminalFontFamily.value = await api.updateTerminalFontFamily(sanitizeFontFamily(family));
  }

  async function saveEditor(next: string) {
    editor.value = await api.updateEditor(next);
  }

  async function replaceSettings(data: AppData) {
    const next = await api.replaceAppData(data);
    applyState(next);
    startAutoRefresh();
    return next;
  }

  async function saveRefreshInterval(seconds: number) {
    refreshIntervalSeconds.value = await api.updateAppSettings(seconds);
    startAutoRefresh();
  }

  async function saveRefreshActiveHours(hours: RefreshActiveHours) {
    refreshActiveHours.value = await api.updateRefreshActiveHours(
      normalizeRefreshActiveHours(hours),
    );
    startAutoRefresh();
  }

  async function saveWindowState(next: WindowState) {
    windowState.value = await api.updateWindowState(next);
    return windowState.value;
  }

  function isRepoRefreshing(repoId: string) {
    return Boolean(refreshingRepos.value[repoId]);
  }

  function isGroupRefreshing(groupId: string) {
    return Boolean(refreshingGroups.value[groupId]);
  }

  const refreshProgressLabel = computed(() => {
    if (!refreshingAll.value && !Object.keys(refreshingGroups.value).length) {
      return "";
    }
    if (!refreshTotal.value) {
      return "Fetching…";
    }
    return `Fetching ${refreshDone.value}/${refreshTotal.value}`;
  });

  const pullProgressLabel = computed(() => {
    if (!pullingAll.value) {
      return "";
    }
    if (!pullAllTotal.value) {
      return "Pulling…";
    }
    return `Pulling ${pullAllDone.value}/${pullAllTotal.value}`;
  });

  const autoRefreshPaused = computed(() => {
    return (
      refreshIntervalSeconds.value > 0 &&
      refreshActiveHours.value.enabled &&
      !isWithinActiveHours(new Date(nowTick.value), refreshActiveHours.value)
    );
  });

  const countdownLabel = computed(() => {
    if (refreshIntervalSeconds.value <= 0) {
      return "";
    }
    if (autoRefreshPaused.value) {
      const { start } = resolvedRefreshHours(refreshActiveHours.value);
      return `Paused until ${formatClockLabel(start)}`;
    }
    if (!nextRefreshAt.value) {
      return "";
    }
    const remaining = Math.max(0, Math.ceil((nextRefreshAt.value - nowTick.value) / 1000));
    const minutes = Math.floor(remaining / 60);
    const seconds = remaining % 60;
    return `Next fetch in ${minutes}:${String(seconds).padStart(2, "0")}`;
  });

  async function createGroup(name: string) {
    const group = await api.createGroup(name);
    groups.value = [group, ...groups.value];
    return group;
  }

  async function renameGroup(groupId: string, name: string) {
    await api.renameGroup(groupId, name);
    patchGroup(groupId, { name });
  }

  async function deleteGroup(groupId: string) {
    await api.deleteGroup(groupId);
    groups.value = groups.value.filter((group) => group.id !== groupId);
    const next = { ...results.value };
    delete next[groupId];
    results.value = next;
  }

  function groupNeedsStatus(group: RepoGroup) {
    return group.repos.some((repo) => !statuses.value[repo.id]);
  }

  function toggleGroup(groupId: string) {
    const group = groups.value.find((item) => item.id === groupId);
    if (!group) {
      return;
    }
    const expanded = !group.expanded;
    patchGroup(groupId, { expanded });
    void api.toggleGroup(groupId).catch((err) => {
      patchGroup(groupId, { expanded: !expanded });
      error.value = String(err);
    });
    if (expanded && groupNeedsStatus(group)) {
      void refreshStatus(groupId);
    }
  }

  function setAllGroupsExpanded(expanded: boolean) {
    if (!groups.value.length) {
      return;
    }
    const previous = groups.value.map((group) => group.expanded);
    groups.value = groups.value.map((group) =>
      group.expanded === expanded ? group : { ...group, expanded },
    );
    void api.setAllGroupsExpanded(expanded).catch((err) => {
      groups.value = groups.value.map((group, index) => ({
        ...group,
        expanded: previous[index] ?? group.expanded,
      }));
      error.value = String(err);
    });
    if (expanded) {
      for (const group of groups.value) {
        if (groupNeedsStatus(group)) {
          void refreshStatus(group.id);
        }
      }
    }
  }

  async function saveSettings(
    groupId: string,
    pullFromBranch: string,
    checkoutFallbacks: string[],
    headerColor?: string,
  ) {
    await api.updateGroupSettings(groupId, pullFromBranch, checkoutFallbacks, headerColor);
    patchGroup(groupId, { pullFromBranch, checkoutFallbacks, headerColor });
  }

  async function addStandaloneRepo(path: string) {
    return trackStandaloneRepo(await api.addStandaloneRepo(path));
  }

  async function cloneStandaloneRepo(url: string, parent: string, name: string) {
    return trackStandaloneRepo(await api.cloneStandaloneRepo(url, parent, name));
  }

  async function trackStandaloneRepo(repo: RepoEntry) {
    standaloneRepos.value = [...standaloneRepos.value, repo];
    applyStatus(await api.refreshRepo(STANDALONE_GROUP_ID, repo.id, false));
    return repo;
  }

  function patchStandaloneRepo(repoId: string, patch: Partial<RepoEntry>) {
    standaloneRepos.value = standaloneRepos.value.map((repo) =>
      repo.id === repoId ? { ...repo, ...patch } : repo,
    );
  }

  async function updateStandaloneRepo(repoId: string, label?: string, headerColor?: string) {
    const repo = await api.updateStandaloneRepo(repoId, label, headerColor);
    patchStandaloneRepo(repoId, repo);
    return repo;
  }

  async function removeStandaloneRepo(repoId: string) {
    await api.removeStandaloneRepo(repoId);
    standaloneRepos.value = standaloneRepos.value.filter((repo) => repo.id !== repoId);
    const next = { ...statuses.value };
    delete next[repoId];
    statuses.value = next;
  }

  async function addRepo(groupId: string, path: string) {
    const repo = await api.addRepo(groupId, path);
    const group = groups.value.find((item) => item.id === groupId);
    if (group) {
      patchGroup(groupId, { repos: [...group.repos, repo] });
    }
    await refreshStatus(groupId);
  }

  async function removeRepo(groupId: string, repoId: string) {
    await api.removeRepo(groupId, repoId);
    const group = groups.value.find((item) => item.id === groupId);
    if (group) {
      patchGroup(groupId, { repos: group.repos.filter((repo) => repo.id !== repoId) });
    }
    const next = { ...statuses.value };
    delete next[repoId];
    statuses.value = next;
  }

  async function reorderGroups(groupIds: string[]) {
    const byId = new Map(groups.value.map((group) => [group.id, group]));
    if (groupIds.length !== groups.value.length || groupIds.some((id) => !byId.has(id))) {
      throw new Error("Group list does not match saved groups.");
    }
    const previous = groups.value;
    groups.value = groupIds.map((id) => byId.get(id)!);
    try {
      await api.reorderGroups(groupIds);
    } catch (err) {
      groups.value = previous;
      throw err;
    }
  }

  async function reorderStandaloneRepos(repoIds: string[]) {
    const byId = new Map(standaloneRepos.value.map((repo) => [repo.id, repo]));
    if (
      repoIds.length !== standaloneRepos.value.length ||
      repoIds.some((id) => !byId.has(id))
    ) {
      throw new Error("Repository list does not match saved repositories.");
    }
    const previous = standaloneRepos.value;
    standaloneRepos.value = repoIds.map((id) => byId.get(id)!);
    try {
      await api.reorderStandaloneRepos(repoIds);
    } catch (err) {
      standaloneRepos.value = previous;
      throw err;
    }
  }

  async function reorderGroupRepos(groupId: string, repoIds: string[]) {
    const group = groups.value.find((item) => item.id === groupId);
    if (!group) {
      return;
    }
    const byId = new Map(group.repos.map((repo) => [repo.id, repo]));
    if (repoIds.length !== group.repos.length || repoIds.some((id) => !byId.has(id))) {
      throw new Error("Repository list does not match this group.");
    }
    const previous = group.repos;
    patchGroup(groupId, { repos: repoIds.map((id) => byId.get(id)!) });
    try {
      await api.reorderGroupRepos(groupId, repoIds);
    } catch (err) {
      patchGroup(groupId, { repos: previous });
      throw err;
    }
  }

  function repoDisplayName(repoId: string, path: string) {
    return (
      statuses.value[repoId]?.name ??
      path.split("/").filter(Boolean).pop() ??
      path
    );
  }

  async function refreshStandaloneRepo(repoId: string) {
    const repo = standaloneRepos.value.find((item) => item.id === repoId);
    if (!repo || refreshingRepos.value[repoId]) {
      return;
    }
    markReposRefreshing([repoId]);
    error.value = "";
    try {
      applyStatus(await api.refreshRepo(STANDALONE_GROUP_ID, repoId, true));
      showToast(`Fetched ${repoDisplayName(repoId, repo.path)}.`);
    } catch (err) {
      const text = String(err);
      error.value = text;
      showToast(text, "error");
    } finally {
      unmarkRepoRefreshing(repoId);
    }
  }

  async function pullStandaloneRepo(repoId: string, branch?: string) {
    const repo = standaloneRepos.value.find((item) => item.id === repoId);
    if (!repo || refreshingRepos.value[repoId] || pullingAll.value) {
      return;
    }
    markReposRefreshing([repoId]);
    error.value = "";
    try {
      const result = await api.pullRepo(STANDALONE_GROUP_ID, repoId, branch);
      applyStatus(await api.refreshRepo(STANDALONE_GROUP_ID, repoId, false));
      const name = repoDisplayName(repoId, repo.path);
      presentActionResults(
        branch ? `Pull ${branch}` : "Pull",
        [result],
        {
          success: branch ? `Pulled ${branch} into ${name}.` : `Pulled ${name}.`,
          error: `Pull failed for ${name}.`,
        },
      );
    } catch (err) {
      const text = String(err);
      error.value = text;
      showToast(text, "error");
    } finally {
      unmarkRepoRefreshing(repoId);
    }
  }

  async function checkoutStandaloneRepo(repoId: string, target: string, fallbacks: string[]) {
    const repo = standaloneRepos.value.find((item) => item.id === repoId);
    const branch = target.trim();
    if (!repo || !branch || refreshingRepos.value[repoId]) {
      return;
    }
    if (statuses.value[repoId]?.branch === branch) {
      showToast(`Already on ${branch}`);
      return;
    }
    markReposRefreshing([repoId]);
    error.value = "";
    try {
      const result = await api.checkoutRepo(STANDALONE_GROUP_ID, repoId, branch, fallbacks);
      applyStatus(await api.refreshRepo(STANDALONE_GROUP_ID, repoId, false));
      showToast(result.message, result.ok ? "success" : "error");
    } catch (err) {
      const text = String(err);
      error.value = text;
      showToast(text, "error");
    } finally {
      unmarkRepoRefreshing(repoId);
    }
  }

  async function pullGroup(groupId: string, branch?: string) {
    const group = groups.value.find((item) => item.id === groupId);
    if (!group || busy.value[groupId] || !group.repos.length || pullingAll.value) {
      return;
    }
    error.value = "";
    const outcomes: RepoActionResult[] = [];
    pullCancelled.value = { ...pullCancelled.value, [groupId]: false };
    try {
      for (const [index, repo] of group.repos.entries()) {
        if (pullCancelled.value[groupId]) {
          break;
        }
        const name = repoDisplayName(repo.id, repo.path);
        const progress = `${index + 1}/${group.repos.length}`;
        const activeBranch = branch || statuses.value[repo.id]?.branch || "…";
        pullProgress.value = { ...pullProgress.value, [groupId]: progress };
        pullBranchByGroup.value = { ...pullBranchByGroup.value, [groupId]: activeBranch };
        busy.value = {
          ...busy.value,
          [groupId]: `Pulling ${name} (${progress})…`,
        };
        refreshingRepos.value = { ...refreshingRepos.value, [repo.id]: true };
        await nextTick();
        try {
          outcomes.push(await api.pullRepo(groupId, repo.id, branch));
          applyStatus(await api.refreshRepo(groupId, repo.id, false));
        } catch (err) {
          outcomes.push({
            path: repo.path,
            ok: false,
            message: String(err),
          });
        } finally {
          const next = { ...refreshingRepos.value };
          delete next[repo.id];
          refreshingRepos.value = next;
          await nextTick();
        }
      }
      if (outcomes.length && !pullCancelled.value[groupId]) {
        results.value = { ...results.value, [groupId]: outcomes };
        const failed = outcomes.filter((item) => !item.ok).length;
        const repos =
          group.repos.length === 1 ? "1 repository" : `${group.repos.length} repositories`;
        presentActionResults(
          branch ? `Pull ${branch} — ${group.name}` : `Pull — ${group.name}`,
          outcomes,
          {
            success: branch
              ? `Pulled ${branch} into ${group.name} (${repos}).`
              : `Pulled ${group.name} (${repos}).`,
            error:
              failed === 1
                ? `Pull failed for 1 repository in ${group.name}.`
                : `Pull failed for ${failed} repositories in ${group.name}.`,
          },
        );
      }
    } finally {
      const next = { ...busy.value };
      delete next[groupId];
      busy.value = next;
      const progress = { ...pullProgress.value };
      delete progress[groupId];
      pullProgress.value = progress;
      const branches = { ...pullBranchByGroup.value };
      delete branches[groupId];
      pullBranchByGroup.value = branches;
      const cancelled = { ...pullCancelled.value };
      delete cancelled[groupId];
      pullCancelled.value = cancelled;
    }
  }

  function pullDoneMessage(count: number) {
    const repos = count === 1 ? "1 repository" : `${count} repositories`;
    return `Pulled ${repos}.`;
  }

  async function pullAll() {
    const standaloneCount = standaloneRepos.value.length;
    const groupedCount = groups.value.reduce((sum, group) => sum + group.repos.length, 0);
    if (
      pullingAll.value ||
      refreshingAll.value ||
      Object.keys(pullProgress.value).length ||
      !(standaloneCount || groupedCount)
    ) {
      return;
    }
    pullingAll.value = true;
    pullAllCancelled.value = false;
    error.value = "";
    pullAllDone.value = 0;
    pullAllTotal.value = standaloneCount + groupedCount;
    const groupTotals: Record<string, number> = {};
    const completed: Record<string, number> = {};
    const progress = { ...pullProgress.value };
    const branches = { ...pullBranchByGroup.value };
    if (standaloneCount) {
      groupTotals[STANDALONE_GROUP_ID] = standaloneCount;
      completed[STANDALONE_GROUP_ID] = 0;
      progress[STANDALONE_GROUP_ID] = `0/${standaloneCount}`;
    }
    for (const group of groups.value) {
      if (!group.repos.length) {
        continue;
      }
      groupTotals[group.id] = group.repos.length;
      completed[group.id] = 0;
      progress[group.id] = `0/${group.repos.length}`;
      branches[group.id] = "current";
    }
    pullProgress.value = progress;
    pullBranchByGroup.value = branches;
    const paths = new Map<string, string>();
    for (const repo of standaloneRepos.value) {
      paths.set(repo.id, repo.path);
    }
    for (const group of groups.value) {
      for (const repo of group.repos) {
        paths.set(repo.id, repo.path);
      }
    }
    const jobs = interleaveRefreshJobs([
      ...standaloneRepos.value.map((repo) => ({
        groupId: STANDALONE_GROUP_ID,
        repoId: repo.id,
      })),
      ...groups.value.flatMap((group) =>
        group.repos.map((repo) => ({ groupId: group.id, repoId: repo.id })),
      ),
    ]);
    markReposRefreshing(jobs.map((job) => job.repoId));
    await nextTick();
    const outcomes: RepoActionResult[] = [];
    if (autoRefreshTimer) {
      clearTimeout(autoRefreshTimer);
      autoRefreshTimer = null;
    }
    try {
      await runPool(
        jobs,
        async (job) => {
          if (pullAllCancelled.value) {
            unmarkRepoRefreshing(job.repoId);
            return;
          }
          const activeBranch = statuses.value[job.repoId]?.branch || "current";
          pullBranchByGroup.value = {
            ...pullBranchByGroup.value,
            [job.groupId]: activeBranch,
          };
          const path = paths.get(job.repoId) ?? job.repoId;
          try {
            outcomes.push(await api.pullRepo(job.groupId, job.repoId));
            applyStatus(await api.refreshRepo(job.groupId, job.repoId, false));
          } catch (err) {
            outcomes.push({
              path,
              ok: false,
              message: String(err),
            });
          } finally {
            unmarkRepoRefreshing(job.repoId);
            completed[job.groupId] = (completed[job.groupId] ?? 0) + 1;
            const total = groupTotals[job.groupId];
            if (total) {
              pullProgress.value = {
                ...pullProgress.value,
                [job.groupId]: `${completed[job.groupId]}/${total}`,
              };
            }
            if (pullAllTotal.value > 0) {
              pullAllDone.value += 1;
            }
          }
        },
        () => pullAllCancelled.value,
      );
      if (outcomes.length && !pullAllCancelled.value) {
        const failed = outcomes.filter((item) => !item.ok).length;
        presentActionResults(
          "Pull",
          outcomes,
          {
            success: pullDoneMessage(pullAllTotal.value),
            error:
              failed === 1
                ? "Pull failed for 1 repository."
                : `Pull failed for ${failed} repositories.`,
          },
        );
      }
    } finally {
      pullingAll.value = false;
      pullAllCancelled.value = false;
      pullAllDone.value = 0;
      pullAllTotal.value = 0;
      const leftoverProgress = { ...pullProgress.value };
      const leftoverBranches = { ...pullBranchByGroup.value };
      for (const groupId of Object.keys(groupTotals)) {
        delete leftoverProgress[groupId];
        delete leftoverBranches[groupId];
      }
      pullProgress.value = leftoverProgress;
      pullBranchByGroup.value = leftoverBranches;
      clearReposRefreshing(jobs.map((job) => job.repoId));
      startAutoRefresh();
    }
  }

  async function checkoutGroup(groupId: string, target: string, fallbacks: string[]) {
    const group = groups.value.find((item) => item.id === groupId);
    if (!group || busy.value[groupId] || !group.repos.length) {
      return;
    }
    const branch = target.trim();
    if (!branch) {
      return;
    }
    error.value = "";
    const outcomes: RepoActionResult[] = [];
    checkoutCancelled.value = { ...checkoutCancelled.value, [groupId]: false };
    try {
      for (const [index, repo] of group.repos.entries()) {
        if (checkoutCancelled.value[groupId]) {
          break;
        }
        const name = repoDisplayName(repo.id, repo.path);
        const progress = `${index + 1}/${group.repos.length}`;
        checkoutProgress.value = { ...checkoutProgress.value, [groupId]: progress };
        busy.value = {
          ...busy.value,
          [groupId]: `Checking out ${name} (${progress})…`,
        };
        const alreadyOn = statuses.value[repo.id]?.branch === branch;
        if (alreadyOn) {
          outcomes.push({
            path: repo.path,
            ok: true,
            message: `Already on ${branch}`,
          });
          continue;
        }
        refreshingRepos.value = { ...refreshingRepos.value, [repo.id]: true };
        await nextTick();
        try {
          outcomes.push(await api.checkoutRepo(groupId, repo.id, branch, fallbacks));
          applyStatus(await api.refreshRepo(groupId, repo.id, false));
        } catch (err) {
          outcomes.push({
            path: repo.path,
            ok: false,
            message: String(err),
          });
        } finally {
          const next = { ...refreshingRepos.value };
          delete next[repo.id];
          refreshingRepos.value = next;
          await nextTick();
        }
      }
      if (outcomes.length && !checkoutCancelled.value[groupId]) {
        results.value = { ...results.value, [groupId]: outcomes };
        const failed = outcomes.filter((item) => !item.ok).length;
        const repos =
          group.repos.length === 1 ? "1 repository" : `${group.repos.length} repositories`;
        presentActionResults(`Checkout — ${group.name}`, outcomes, {
          success: `Checked out ${group.name} (${repos}).`,
          error:
            failed === 1
              ? `Checkout failed for 1 repository in ${group.name}.`
              : `Checkout failed for ${failed} repositories in ${group.name}.`,
        });
      }
    } finally {
      const next = { ...busy.value };
      delete next[groupId];
      busy.value = next;
      const progress = { ...checkoutProgress.value };
      delete progress[groupId];
      checkoutProgress.value = progress;
      const cancelled = { ...checkoutCancelled.value };
      delete cancelled[groupId];
      checkoutCancelled.value = cancelled;
    }
  }

  async function runAction(
    groupId: string,
    label: string,
    action: () => Promise<RepoActionResult[]>,
    title?: string,
  ) {
    busy.value = { ...busy.value, [groupId]: label };
    error.value = "";
    try {
      const outcome = await action();
      results.value = { ...results.value, [groupId]: outcome };
      const group = groups.value.find((item) => item.id === groupId);
      const failed = outcome.filter((item) => !item.ok).length;
      const heading = title ?? label.replace(/…$/, "");
      presentActionResults(heading, outcome, {
        success: group ? `Finished ${heading.toLowerCase()} for ${group.name}.` : "Done.",
        error:
          failed === 1
            ? `${heading} failed for 1 repository.`
            : `${heading} failed for ${failed} repositories.`,
      });
      await refreshStatus(groupId);
    } catch (err) {
      error.value = String(err);
    } finally {
      const next = { ...busy.value };
      delete next[groupId];
      busy.value = next;
    }
  }

  function clearResults(groupId: string) {
    const next = { ...results.value };
    delete next[groupId];
    results.value = next;
  }

  function findRepo(repoId: string) {
    const standalone = standaloneRepos.value.find((item) => item.id === repoId);
    if (standalone) {
      return { group: null, repo: standalone, status: statuses.value[standalone.id] };
    }
    for (const group of groups.value) {
      const repo = group.repos.find((item) => item.id === repoId);
      if (repo) {
        return { group, repo, status: statuses.value[repo.id] };
      }
    }
    return null;
  }

  function patchGroup(groupId: string, patch: Partial<RepoGroup>) {
    groups.value = groups.value.map((group) =>
      group.id === groupId ? { ...group, ...patch } : group,
    );
  }

  if (!autoRefreshStarted) {
    autoRefreshStarted = true;
    watch([refreshIntervalSeconds, refreshActiveHours], () => {
      startAutoRefresh();
    });
    watch(
      () => [...dashboardWatchTargets().keys()].sort().join("\n"),
      () => {
        syncGitWatches();
      },
      { immediate: true },
    );
    void listen<RepoFilesChanged>("repo-git-changed", (event) => {
      onRepoGitChanged(event.payload);
    });
    if (!tickTimer) {
      tickTimer = setInterval(() => {
        nowTick.value = Date.now();
      }, 250);
    }
  }

  return {
    groups,
    standaloneRepos,
    statuses: statusById,
    results,
    busy,
    error,
    loaded,
    refreshIntervalSeconds,
    refreshActiveHours,
    saveRefreshActiveHours,
    filesPaneWidth,
    setFilesPaneWidth,
    saveFilesPaneWidth,
    terminalPaneHeight,
    setTerminalPaneHeight,
    saveTerminalPaneHeight,
    diffMode,
    saveDiffMode,
    diffFontFamily,
    saveDiffFontFamily,
    diffFontSize,
    setDiffFontSize,
    saveDiffFontSize,
    terminalFontFamily,
    saveTerminalFontFamily,
    terminalFontSize,
    setTerminalFontSize,
    saveTerminalFontSize,
    editor,
    saveEditor,
    windowState,
    saveWindowState,
    replaceSettings,
    refreshingAll,
    lastRefreshAt,
    countdownLabel,
    refreshProgressLabel,
    refreshCancelled,
    refreshProgress,
    toastMessage,
    toastKind,
    toastHasDetails,
    actionOutput,
    actionOutputOpen,
    showToast,
    dismissToast,
    presentActionResults,
    dismissOutput,
    openOutput,
    cancelRefresh,
    cancelPull,
    cancelPullAll,
    cancelCheckout,
    load,
    refreshStatus,
    refreshRepoStatus,
    patchRepoStatus,
    refreshGroup,
    refreshStandaloneRepo,
    refreshAll,
    pullStandaloneRepo,
    checkoutStandaloneRepo,
    pullAll,
    pullingAll,
    pullAllCancelled,
    pullProgressLabel,
    pullGroup,
    pullProgress,
    pullBranchByGroup,
    pullCancelled,
    checkoutGroup,
    checkoutProgress,
    checkoutCancelled,
    isRepoRefreshing,
    isGroupRefreshing,
    saveRefreshInterval,
    createGroup,
    renameGroup,
    deleteGroup,
    toggleGroup,
    setAllGroupsExpanded,
    saveSettings,
    addRepo,
    addStandaloneRepo,
    cloneStandaloneRepo,
    updateStandaloneRepo,
    removeStandaloneRepo,
    removeRepo,
    reorderGroups,
    reorderStandaloneRepos,
    reorderGroupRepos,
    repoDisplayName,
    runAction,
    clearResults,
    findRepo,
  };
}
