<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import BranchList from "./BranchList.vue";
import BranchSelect from "./BranchSelect.vue";
import ChangesPanelTabs from "./ChangesPanelTabs.vue";
import CommitFiles from "./CommitFiles.vue";
import FileHistoryList from "./FileHistoryList.vue";
import FileTree from "./FileTree.vue";
import CommitContextMenu from "./CommitContextMenu.vue";
import CommitGraph from "./CommitGraph.vue";
import DiffViewer from "./DiffViewer.vue";
import Modal from "./Modal.vue";
import PathLabel from "./PathLabel.vue";
import RemoteList from "./RemoteList.vue";
import RepoToolbar from "./RepoToolbar.vue";
import RepoViewTabs, { type RepoViewTab } from "./RepoViewTabs.vue";
import StashList from "./StashList.vue";
import TagList from "./TagList.vue";
import TerminalPane from "./TerminalPane.vue";
import WorkingTree from "./WorkingTree.vue";
import { useApp } from "../composables/useApp";
import * as api from "../api";
import type {
  BranchOverview,
  BranchTracking,
  CommitFile,
  CommitNode,
  LastCommit,
  LocalBranch,
  RemoteBranch,
  RemoteEntry,
  RemoteOverview,
  RepoFile,
  RepoFilesChanged,
  StashEntry,
  TagEntry,
  WorkingTreeFile,
} from "../types";
import { STANDALONE_GROUP_ID } from "../types";
import { formatCommitDate } from "../graphLayout";
import {
  abortLabel,
  absoluteFilePath,
  continueLabel,
  fileBasename,
  isConflicted,
  openInEditorLabel,
  operationNoun,
  operationTitle,
  type IgnoreKind,
} from "../gitOperation";

const props = defineProps<{
  repoId: string;
}>();

const {
  findRepo,
  groups,
  standaloneRepos,
  loaded,
  filesPaneWidth,
  setFilesPaneWidth,
  saveFilesPaneWidth,
  diffMode,
  saveDiffMode,
  editor,
  refreshRepoStatus,
  patchRepoStatus,
  showToast,
  presentActionResults,
  repoDisplayName,
} = useApp();

const commits = ref<CommitNode[]>([]);
const files = ref<WorkingTreeFile[]>([]);
const branches = ref<string[]>([]);
const branchTracking = ref<BranchTracking[]>([]);
let trackingGeneration = 0;
const trackingPath = ref("");
const mainTab = ref<RepoViewTab>("commits");
const branchesView = computed(() => mainTab.value === "branches");
const graphStale = ref(false);
const stashes = ref<StashEntry[]>([]);
const stashView = computed(() => mainTab.value === "stashes");
const tags = ref<TagEntry[]>([]);
const tagView = computed(() => mainTab.value === "tags");
const remotesView = computed(() => mainTab.value === "remotes");
const remotes = ref<RemoteEntry[]>([]);
const selectedRemote = ref("");
const remoteOverview = ref<RemoteOverview | null>(null);
const remoteLoading = ref(false);
const remoteFetching = ref(false);
let remoteGeneration = 0;
const remoteForm = ref<{ mode: "add" | "edit"; original: string } | null>(null);
const remoteFormName = ref("");
const remoteFormUrl = ref("");
const remoteFormNameInput = ref<HTMLInputElement | null>(null);
const namingCheckout = ref<RemoteBranch | null>(null);
const checkoutLocalName = ref("");
const checkoutLocalNameInput = ref<HTMLInputElement | null>(null);
const syncingBranch = ref<RemoteBranch | null>(null);
const syncTarget = ref("");
const syncAllowMerge = ref(false);
const syncPush = ref(true);
const terminalStarted = ref(false);
const overview = ref<BranchOverview | null>(null);
let overviewGeneration = 0;
const selectedFile = ref<WorkingTreeFile | null>(null);
const selectedCommit = ref<CommitNode | null>(null);
const selectedStash = ref<StashEntry | null>(null);
const selectedCommitFile = ref<CommitFile | null>(null);
const commitFiles = ref<CommitFile[]>([]);
const commitFilesLoading = ref(false);
let commitFilesGeneration = 0;
const commitDetail = computed(() => {
  const stash = selectedStash.value;
  if (stash) {
    return {
      title: stash.message,
      meta: `${stashRef(stash.index)} · ${formatCommitDate(stash.date)}`,
      label: stashRef(stash.index),
    };
  }
  const commit = selectedCommit.value;
  if (commit) {
    return {
      title: commit.subject,
      meta: `${commit.hash.slice(0, 7)} · ${commit.author} · ${formatCommitDate(commit.date)}`,
      label: commit.hash.slice(0, 7),
    };
  }
  return null;
});
const historyOpen = ref(false);
const repoFiles = ref<RepoFile[]>([]);
const repoFilesLoading = ref(false);
const repoFilesError = ref("");
let repoFilesGeneration = 0;
const selectedHistoryFile = ref("");
const lastHistoryFile = ref("");
const historyCommits = ref<CommitNode[]>([]);
const historyCommitsLoading = ref(false);
const historyCommitsError = ref("");
const selectedHistoryCommit = ref<CommitNode | null>(null);
let historyCommitsGeneration = 0;
const diff = ref("");
const showingDiff = computed(
  () => Boolean(selectedFile.value || selectedCommitFile.value || selectedHistoryCommit.value),
);
const blameFile = computed(
  () =>
    selectedFile.value?.path ??
    selectedCommitFile.value?.path ??
    selectedHistoryCommit.value?.path ??
    selectedHistoryFile.value,
);
const blameOldPath = computed(
  () => selectedCommitFile.value?.oldPath ?? selectedHistoryCommit.value?.oldPath ?? "",
);
const blameRev = computed(() => {
  if (selectedFile.value) {
    return "";
  }
  if (selectedHistoryCommit.value) {
    return selectedHistoryCommit.value.hash;
  }
  return selectedStash.value?.hash ?? selectedCommit.value?.hash ?? "";
});
const blameStaged = computed(() => selectedFile.value?.staged ?? false);
const loading = ref(false);
const actionBusy = ref(false);
const actionLabel = ref("");
const actionBranch = ref("");
const message = ref("");
const creatingBranch = ref(false);
const newBranchName = ref("");
const baseBranch = ref("");
const newBranchStart = ref("");
const newBranchInput = ref<HTMLInputElement | null>(null);
const commitMenu = ref<{
  commit: CommitNode;
  hashes: string[];
  x: number;
  y: number;
} | null>(null);
const renamingBranch = ref<LocalBranch | null>(null);
const renameBranchName = ref("");
const renameBranchInput = ref<HTMLInputElement | null>(null);
const mergingBranch = ref(false);
const mergeSource = ref("");
const mergeTarget = ref("");
const committing = ref(false);
const amending = ref(false);
const lastCommit = ref<LastCommit | null>(null);
const draftTitle = ref("");
const draftDescription = ref("");
const commitTitle = ref("");
const commitDescription = ref("");
const commitTitleInput = ref<HTMLInputElement | null>(null);
const stashing = ref(false);
const stashMessage = ref("");
const stashMessageInput = ref<HTMLInputElement | null>(null);
const stashFileTargets = ref<WorkingTreeFile[]>([]);
const creatingTag = ref(false);
const newTagName = ref("");
const newTagMessage = ref("");
const newTagTarget = ref("");
const newTagInput = ref<HTMLInputElement | null>(null);
const pullingOptions = ref(false);
const pullSource = ref<"current" | "develop" | "master" | "main" | "specify">("current");
const specifyBranch = ref("");

const COMMIT_TITLE_MAX = 72;

function isDetachedBranch(name: string) {
  return name === "HEAD" || name === "detached HEAD" || name.startsWith("detached ");
}

const checkedOutBranch = computed(() => {
  const name = current.value?.status?.branch ?? "";
  return name && !isDetachedBranch(name) ? name : "";
});
const baseBranchOptions = computed(() => {
  const names = [...branches.value];
  const currentName = checkedOutBranch.value;
  if (currentName && !names.includes(currentName)) {
    names.unshift(currentName);
  }
  return names;
});
const canCreateBranch = computed(() => {
  if (!newBranchName.value.trim()) {
    return false;
  }
  if (newBranchStart.value.trim()) {
    return true;
  }
  return Boolean(baseBranch.value.trim());
});
const commitMenuHasMerge = computed(() => {
  const menu = commitMenu.value;
  if (!menu) {
    return false;
  }
  const selected = new Set(menu.hashes);
  return commits.value.some((commit) => selected.has(commit.hash) && commit.parents.length > 1);
});
const newBranchStartShort = computed(() => {
  const start = newBranchStart.value.trim();
  return start ? start.slice(0, 7) : "";
});
const canCreateTag = computed(() => Boolean(newTagName.value.trim()));
const canRenameBranch = computed(() => {
  const next = renameBranchName.value.trim();
  return Boolean(next) && next !== (renamingBranch.value?.name ?? "");
});
const localBranchNames = computed(() => {
  const fromOverview = overview.value?.branches.map((branch) => branch.name) ?? [];
  if (fromOverview.length) {
    return fromOverview;
  }
  return branches.value;
});
const mergeTargetOptions = computed(() =>
  localBranchNames.value.filter((name) => name !== mergeSource.value),
);
const mergeTargetHint = computed(() => {
  const raw = overview.value?.mergeTarget ?? preferredMergeTarget() ?? "";
  return raw.replace(/^origin\//, "").trim();
});
const canConfirmMerge = computed(() => {
  const source = mergeSource.value.trim();
  const target = mergeTarget.value.trim();
  return Boolean(source && target && source !== target);
});
const commitTitleLength = computed(() => [...commitTitle.value].length);
const commitTitleLeft = computed(() => Math.max(0, COMMIT_TITLE_MAX - commitTitleLength.value));
const conflictedFiles = computed(() => files.value.filter(isConflicted));
const unstagedCount = computed(
  () => files.value.filter((file) => !file.staged && !isConflicted(file)).length,
);
const stagedCount = computed(
  () => files.value.filter((file) => file.staged && !isConflicted(file)).length,
);
const conflictedCount = computed(() => conflictedFiles.value.length);
const commitTitleValid = computed(
  () => Boolean(commitTitle.value.trim()) && commitTitleLength.value <= COMMIT_TITLE_MAX,
);
const canCommit = computed(() => commitTitleValid.value && stagedCount.value > 0);
// Staging everything would mark conflicted files resolved.
const commitAllAvailable = computed(
  () => unstagedCount.value > 0 && conflictedCount.value === 0,
);
const canCommitAll = computed(() => commitTitleValid.value && commitAllAvailable.value);
const operation = computed(() => current.value?.status?.operation ?? "");
const conflictActive = computed(() => Boolean(operation.value || conflictedCount.value));
const openEditorLabel = computed(() => openInEditorLabel(editor.value));
const conflictHeading = computed(() => operationTitle(operation.value));
const conflictCopy = computed(() => {
  const count = conflictedCount.value;
  if (count > 0) {
    const filesLabel = count === 1 ? "1 file still has conflicts" : `${count} files still have conflicts`;
    return `${filesLabel}. Open each file, fix the markers, then mark it resolved.`;
  }
  if (operation.value) {
    return `All conflicted files are marked resolved. Continue the ${operationNoun(operation.value)} or abort it.`;
  }
  return "";
});

const current = computed(() => findRepo(props.repoId));
const resizing = ref(false);
let resizeStartX = 0;
let resizeStartWidth = 320;
let resizePointerId: number | null = null;
let loadGeneration = 0;
let filesGeneration = 0;
let worktreeMutation = 0;
let watchRefresh: Promise<void> | null = null;
let watchRefreshQueued = false;
let watchRefreshRefs = false;
let watchToken = 0;
let watchClosed = false;
let stopWatch: UnlistenFn | undefined;
let watchedPath = "";

function onResizeMove(event: PointerEvent) {
  setFilesPaneWidth(resizeStartWidth + (resizeStartX - event.clientX));
}

function stopResize(event?: PointerEvent) {
  if (resizePointerId === null) {
    return;
  }
  if (event && event.pointerId !== resizePointerId) {
    return;
  }
  window.removeEventListener("pointermove", onResizeMove);
  window.removeEventListener("pointerup", stopResize);
  window.removeEventListener("pointercancel", stopResize);
  resizing.value = false;
  resizePointerId = null;
  document.body.classList.remove("is-resizing", "is-resizing-x");
  void saveFilesPaneWidth(filesPaneWidth.value);
}

function startResize(event: PointerEvent) {
  event.preventDefault();
  resizeStartX = event.clientX;
  resizeStartWidth = filesPaneWidth.value;
  resizePointerId = event.pointerId;
  resizing.value = true;
  document.body.classList.add("is-resizing", "is-resizing-x");
  window.addEventListener("pointermove", onResizeMove);
  window.addEventListener("pointerup", stopResize);
  window.addEventListener("pointercancel", stopResize);
}

onUnmounted(() => {
  watchClosed = true;
  stopResize();
  stopWatch?.();
  void stopWatching(watchedPath);
});

function watchKey(path: string) {
  return path.replace(/\/+$/, "");
}

async function stopWatching(path: string) {
  if (!path) {
    return;
  }
  try {
    await api.unwatchRepo(path);
  } catch {
    /* already gone */
  }
}

async function startWatching(path: string) {
  const next = watchKey(path);
  if (watchedPath === next) {
    return;
  }
  const token = ++watchToken;
  const previous = watchedPath;
  watchedPath = next;
  await stopWatching(previous);
  if (token !== watchToken) {
    return;
  }
  if (!next) {
    return;
  }
  try {
    await api.watchRepo(next);
    if (token !== watchToken) {
      await stopWatching(next);
    }
  } catch {
    if (token === watchToken) {
      watchedPath = "";
    }
  }
}

async function refreshHistoryFile() {
  const match = current.value;
  const path = selectedHistoryFile.value;
  if (!match || !path) {
    return;
  }
  const generation = ++historyCommitsGeneration;
  try {
    const next = await api.fileLog(match.repo.path, path);
    if (generation !== historyCommitsGeneration) {
      return;
    }
    historyCommits.value = next;
  } catch {
    /* keep the list already on screen */
  }
}

async function refreshSelectedFileDiff(file: WorkingTreeFile) {
  const match = current.value;
  if (!match || selectedCommitFile.value || selectedHistoryCommit.value) {
    return;
  }
  try {
    const next = await api.fileDiff(match.repo.path, file.path, file.staged);
    if (!sameFile(file, selectedFile.value)) {
      return;
    }
    diff.value = next;
  } catch (err) {
    if (sameFile(file, selectedFile.value)) {
      diff.value = String(err);
    }
  }
}

function scheduleWatchRefresh(refs: boolean) {
  watchRefreshRefs ||= refs;
  watchRefreshQueued = true;
  if (watchRefresh || worktreeMutation > 0 || actionBusy.value) {
    return;
  }
  watchRefresh = drainWatchRefresh().finally(() => {
    watchRefresh = null;
    if (watchRefreshQueued && worktreeMutation === 0 && !actionBusy.value) {
      scheduleWatchRefresh(false);
    }
  });
}

async function drainWatchRefresh() {
  while (watchRefreshQueued && worktreeMutation === 0 && !actionBusy.value) {
    watchRefreshQueued = false;
    const refs = watchRefreshRefs;
    watchRefreshRefs = false;
    await refreshFromWatch(refs);
  }
}

async function refreshFromWatch(refs: boolean) {
  const match = current.value;
  if (!match) {
    return;
  }
  if (refs) {
    await loadRepo({
      silent: true,
      graph: true,
      overview: branchesView.value,
    });
  } else {
    await loadWorkingTree();
  }
  await refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
  if (refs && selectedHistoryFile.value) {
    void refreshHistoryFile();
  }
}

async function onRepoFilesChanged(payload: RepoFilesChanged) {
  const match = current.value;
  if (!match || watchKey(match.repo.path) !== watchKey(payload.path)) {
    return;
  }
  if (actionBusy.value) {
    return;
  }
  scheduleWatchRefresh(payload.git);
  await watchRefresh;
}

function mergeOverview(previous: BranchOverview | null, next: BranchOverview): BranchOverview {
  if (!previous || previous.mergeTarget !== next.mergeTarget) {
    return next;
  }
  const incoming = new Map(next.branches.map((branch) => [branch.name, branch]));
  const kept = previous.branches.flatMap((branch) => {
    const update = incoming.get(branch.name);
    if (!update) {
      return [];
    }
    if (update.pending && !branch.pending) {
      return [
        {
          ...update,
          merged: branch.merged,
          partial: branch.partial,
          pending: false,
        },
      ];
    }
    return [update];
  });
  const seen = new Set(kept.map((branch) => branch.name));
  const added = next.branches.filter((branch) => !seen.has(branch.name));
  return { ...next, branches: [...kept, ...added] };
}

async function classifyOverview(
  path: string,
  preferred: string | undefined,
  generation: number,
) {
  const full = await api.branchOverview(path, preferred, true);
  if (generation !== overviewGeneration || current.value?.repo.path !== path || !branchesView.value) {
    return;
  }
  overview.value = mergeOverview(overview.value, full);
}

async function loadOverview() {
  const match = current.value;
  if (!match) {
    overview.value = null;
    return;
  }
  const generation = ++overviewGeneration;
  const path = match.repo.path;
  const preferred = preferredMergeTarget();
  const snapshot = await api.branchOverview(path, preferred, false);
  if (generation !== overviewGeneration) {
    return;
  }
  const needsClassify = snapshot.branches.some((branch) => branch.pending);
  overview.value = mergeOverview(overview.value, snapshot);
  if (needsClassify) {
    await classifyOverview(path, preferred, generation);
  }
}

function applyWorkingTree(nextFiles: WorkingTreeFile[], silent: boolean) {
  const match = current.value;
  files.value = nextFiles;
  if (match && nextFiles.some(isConflicted)) {
    openChangesPane();
    void refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
  }
  if (historyOpen.value) {
    void loadRepoFiles({ silent });
  }
  const nextSelected = selectedFile.value
    ? nextFiles.find((file) => sameFile(file, selectedFile.value))
    : undefined;
  if (selectedFile.value && !nextSelected) {
    selectedFile.value = null;
    if (!selectedCommitFile.value && !selectedHistoryCommit.value) {
      diff.value = "";
    }
  } else if (nextSelected) {
    selectedFile.value = nextSelected;
    void refreshSelectedFileDiff(nextSelected);
  }
}

async function loadWorkingTree() {
  const match = current.value;
  if (!match) {
    return;
  }
  const path = match.repo.path;
  const generation = ++filesGeneration;
  try {
    const nextFiles = await api.workingTree(path);
    if (generation !== filesGeneration || current.value?.repo.path !== path) {
      return;
    }
    applyWorkingTree(nextFiles, true);
  } catch {
    /* keep the list already on screen */
  }
}

async function loadRepo(options?: { overview?: boolean; graph?: boolean; silent?: boolean }) {
  const match = current.value;
  const generation = ++loadGeneration;
  const fileGeneration = ++filesGeneration;
  if (!match) {
    overviewGeneration += 1;
    trackingGeneration += 1;
    commits.value = [];
    files.value = [];
    branches.value = [];
    branchTracking.value = [];
    trackingPath.value = "";
    stashes.value = [];
    tags.value = [];
    remotes.value = [];
    remoteOverview.value = null;
    overview.value = null;
    selectedFile.value = null;
    closeCommitDetail();
    message.value = loaded.value ? "Repository not found." : "";
    return;
  }

  const wantOverview = options?.overview ?? branchesView.value;
  const wantGraph = options?.graph ?? true;
  const overviewGen = wantOverview ? ++overviewGeneration : overviewGeneration;
  if (!options?.silent) {
    loading.value = true;
  }
  message.value = "";
  try {
    const [nextCommits, nextFiles, nextBranches, nextStashes, nextTags, nextOverview] = await Promise.all([
      wantGraph ? api.logGraph(match.repo.path) : Promise.resolve(commits.value),
      api.workingTree(match.repo.path),
      api.listLocalBranches(match.repo.path).catch(() => [] as string[]),
      api.stashList(match.repo.path).catch(() => [] as StashEntry[]),
      api.tagList(match.repo.path).catch(() => [] as TagEntry[]),
      wantOverview
        ? api.branchOverview(match.repo.path, preferredMergeTarget(), false).catch(() => null)
        : Promise.resolve(overview.value),
    ]);
    if (generation !== loadGeneration) {
      return;
    }
    if (wantGraph) {
      commits.value = nextCommits;
      graphStale.value = false;
    }
    if (fileGeneration === filesGeneration) {
      applyWorkingTree(nextFiles, Boolean(options?.silent));
    }
    branches.value = nextBranches;
    rememberTrackingPath(match.repo.path);
    void loadBranchTracking(match.repo.path);
    void refreshRemoteList(match.repo.path).then(() => {
      if (remotesView.value) {
        void loadRemoteOverview();
      }
    });
    stashes.value = nextStashes;
    if (selectedStash.value) {
      const nextStash = nextStashes.find((stash) => stash.hash === selectedStash.value?.hash);
      if (nextStash) {
        selectedStash.value = nextStash;
      } else {
        closeCommitDetail();
      }
    }
    tags.value = nextTags;
    if (wantOverview) {
      const needsClassify = nextOverview?.branches.some((branch) => branch.pending) ?? false;
      overview.value = nextOverview ? mergeOverview(overview.value, nextOverview) : nextOverview;
      if (needsClassify && overviewGen === overviewGeneration) {
        void classifyOverview(match.repo.path, preferredMergeTarget(), overviewGen);
      }
    }
    if (selectedCommit.value) {
      const nextCommit = nextCommits.find((commit) => commit.hash === selectedCommit.value?.hash);
      if (!nextCommit) {
        closeCommitDetail();
      } else {
        selectedCommit.value = nextCommit;
        await refreshCommitFiles();
      }
    }
  } catch (err) {
    if (generation === loadGeneration && !options?.silent) {
      message.value = String(err);
    }
  } finally {
    if (generation === loadGeneration) {
      loading.value = false;
    }
  }
}

function sameFile(file: WorkingTreeFile, other: WorkingTreeFile | null) {
  return !!other && file.path === other.path && file.staged === other.staged;
}

function closeDiff() {
  selectedFile.value = null;
  selectedCommitFile.value = null;
  selectedHistoryCommit.value = null;
  diff.value = "";
}

function closeCommitDetail() {
  selectedCommit.value = null;
  selectedStash.value = null;
  selectedCommitFile.value = null;
  commitFiles.value = [];
  commitFilesLoading.value = false;
  commitFilesGeneration += 1;
  if (!selectedFile.value) {
    diff.value = "";
  }
}

async function refreshCommitFiles() {
  const match = current.value;
  const stash = selectedStash.value;
  const hash = stash?.hash ?? selectedCommit.value?.hash;
  if (!match || !hash) {
    return;
  }
  const generation = ++commitFilesGeneration;
  commitFilesLoading.value = true;
  try {
    const next = stash
      ? await api.stashChanges(match.repo.path, hash)
      : await api.commitFiles(match.repo.path, hash);
    if (generation !== commitFilesGeneration) {
      return;
    }
    commitFiles.value = next;
    if (
      selectedCommitFile.value &&
      !commitFiles.value.some((file) => file.path === selectedCommitFile.value?.path)
    ) {
      selectedCommitFile.value = null;
      if (!selectedFile.value) {
        diff.value = "";
      }
    }
  } catch (err) {
    if (generation !== commitFilesGeneration) {
      return;
    }
    message.value = String(err);
    commitFiles.value = [];
  } finally {
    if (generation === commitFilesGeneration) {
      commitFilesLoading.value = false;
    }
  }
}

async function selectCommit(commit: CommitNode) {
  const match = current.value;
  if (!match) {
    return;
  }
  if (selectedCommit.value?.hash === commit.hash) {
    closeCommitDetail();
    return;
  }
  selectedFile.value = null;
  selectedStash.value = null;
  selectedCommit.value = commit;
  selectedCommitFile.value = null;
  diff.value = "";
  openChangesPane();
  await refreshCommitFiles();
  const first = commitFiles.value[0];
  if (first) {
    await selectCommitFile(first);
  }
}

async function selectStash(stash: StashEntry) {
  if (!current.value) {
    return;
  }
  if (selectedStash.value?.hash === stash.hash) {
    closeCommitDetail();
    return;
  }
  closeCommitDetail();
  selectedFile.value = null;
  selectedStash.value = stash;
  diff.value = "";
  openChangesPane();
  await refreshCommitFiles();
  const first = commitFiles.value[0];
  if (first && selectedStash.value?.hash === stash.hash) {
    await selectCommitFile(first);
  }
}

async function selectCommitFile(file: CommitFile) {
  const match = current.value;
  const stash = selectedStash.value;
  const hash = stash?.hash ?? selectedCommit.value?.hash;
  if (!match || !hash) {
    return;
  }
  if (selectedCommitFile.value?.path === file.path) {
    closeDiff();
    return;
  }
  selectedFile.value = null;
  selectedCommitFile.value = file;
  try {
    diff.value = stash
      ? await api.stashFileDiff(match.repo.path, hash, file.path, file.oldPath)
      : await api.commitFileDiff(match.repo.path, hash, file.path);
  } catch (err) {
    diff.value = String(err);
  }
}

async function selectFile(file: WorkingTreeFile, options?: { toggle?: boolean }) {
  const match = current.value;
  if (!match) {
    return;
  }
  if (sameFile(file, selectedFile.value)) {
    if (options?.toggle !== false) {
      closeDiff();
    }
    return;
  }
  closeCommitDetail();
  selectedFile.value = file;
  try {
    diff.value = await api.fileDiff(match.repo.path, file.path, file.staged);
  } catch (err) {
    diff.value = String(err);
  }
}

async function runWorktreeMutation(work: () => Promise<void>) {
  worktreeMutation += 1;
  let succeeded = false;
  try {
    await work();
    succeeded = true;
    await loadWorkingTree();
    const match = current.value;
    if (match) {
      await refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
    }
  } finally {
    worktreeMutation -= 1;
    if (worktreeMutation > 0) {
      return;
    }
    if (watchRefreshRefs || (!succeeded && watchRefreshQueued)) {
      scheduleWatchRefresh(watchRefreshRefs);
    } else {
      watchRefreshQueued = false;
      watchRefreshRefs = false;
    }
  }
}

async function forEachFile(
  targets: WorkingTreeFile[],
  work: (file: WorkingTreeFile) => Promise<void>,
) {
  for (const [index, file] of targets.entries()) {
    try {
      await work(file);
    } catch (err) {
      if (targets.length === 1) {
        throw err;
      }
      const progress = index ? `Finished ${index} of ${targets.length} files, then stopped. ` : "";
      throw `${progress}${file.path}: ${String(err)}`;
    }
  }
}

function fileListPreview(targets: WorkingTreeFile[]) {
  const shown = targets.slice(0, 8).map((file) => file.path);
  const more = targets.length - shown.length;
  return more > 0 ? [...shown, `…and ${more} more`].join("\n") : shown.join("\n");
}

async function stageFiles(targets: WorkingTreeFile[]) {
  const match = current.value;
  if (!match || !targets.length) {
    return;
  }
  try {
    await runWorktreeMutation(() =>
      api.stageFiles(
        match.repo.path,
        targets.map((file) => file.path),
      ),
    );
  } catch (err) {
    message.value = String(err);
  }
}

async function stageAll() {
  const match = current.value;
  if (!match || !files.value.some((file) => !file.staged)) {
    return;
  }
  try {
    await runWorktreeMutation(() => api.stageAll(match.repo.path));
  } catch (err) {
    message.value = String(err);
  }
}

async function unstageFiles(targets: WorkingTreeFile[]) {
  const match = current.value;
  if (!match || !targets.length) {
    return;
  }
  try {
    await runWorktreeMutation(() =>
      api.unstageFiles(
        match.repo.path,
        targets.map((file) => file.path),
      ),
    );
  } catch (err) {
    message.value = String(err);
  }
}

async function unstageAll() {
  const match = current.value;
  if (!match || !files.value.some((file) => file.staged)) {
    return;
  }
  try {
    await runWorktreeMutation(() => api.unstageAll(match.repo.path));
  } catch (err) {
    message.value = String(err);
  }
}

function markOverviewCurrent(name: string) {
  const currentOverview = overview.value;
  if (!currentOverview) {
    return;
  }
  const nextBranches = currentOverview.branches.map((branch) => ({
    ...branch,
    current: branch.name === name,
  }));
  nextBranches.sort(
    (left, right) =>
      Number(right.current) - Number(left.current) || left.name.localeCompare(right.name),
  );
  overview.value = { ...currentOverview, branches: nextBranches };
}

async function runRepoAction(label: string, work: () => Promise<string>, branch = "") {
  const match = current.value;
  if (!match || actionBusy.value) {
    return;
  }
  actionBusy.value = true;
  actionLabel.value = label;
  actionBranch.value = branch;
  message.value = "";
  try {
    const result = await work();
    showToast(result);
    await loadRepo();
    await refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
  } catch (err) {
    const text = String(err);
    message.value = text;
    showToast(text, "error");
    await loadRepo({ silent: true });
    await refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
    if (conflictActive.value) {
      openChangesPane();
      message.value = "";
      const first = conflictedFiles.value[0];
      if (first) {
        await selectFile(first);
      }
    }
  } finally {
    actionBusy.value = false;
    actionLabel.value = "";
    actionBranch.value = "";
  }
}

const pullBranch = computed(() => {
  if (pullSource.value === "current") {
    return "";
  }
  if (pullSource.value === "specify") {
    return specifyBranch.value.trim();
  }
  return pullSource.value;
});

const canConfirmPull = computed(
  () => pullSource.value === "current" || Boolean(pullBranch.value),
);

const pullRemoteLabel = computed(() => {
  if (pullSource.value === "current") {
    return "current-branch";
  }
  return pullBranch.value || "…";
});

const pullHint = computed(() =>
  pullSource.value === "current"
    ? "Use this to pick up others’ commits on the same branch."
    : "Brings that remote branch into this checkout. Conflicts appear in the files list so you can open them, mark them resolved, or abort.",
);

async function runPull(branch?: string): Promise<boolean> {
  const match = current.value;
  if (!match || actionBusy.value) {
    return false;
  }
  actionBusy.value = true;
  actionLabel.value = "Pulling…";
  actionBranch.value = branch || match.status?.branch || "";
  message.value = "";
  const groupId = match.group?.id ?? STANDALONE_GROUP_ID;
  const name =
    match.status?.name ??
    match.repo.path.split("/").filter(Boolean).pop() ??
    match.repo.path;
  try {
    const result = await api.pullRepo(groupId, match.repo.id, branch);
    presentActionResults(
      branch ? `Pull ${branch}` : "Pull",
      [result],
      {
        success: branch ? `Pulled ${branch} into ${name}.` : `Pulled ${name}.`,
        error: `Pull failed for ${name}.`,
      },
    );
    await loadRepo({ silent: !result.ok });
    await refreshRepoStatus(groupId, match.repo.id);
    if (!result.ok && conflictActive.value) {
      openChangesPane();
      message.value = "";
      const first = conflictedFiles.value[0];
      if (first) {
        await selectFile(first);
      }
    }
    return result.ok;
  } catch (err) {
    const text = String(err);
    message.value = text;
    showToast(text, "error");
    await loadRepo({ silent: true });
    await refreshRepoStatus(groupId, match.repo.id);
    if (conflictActive.value) {
      openChangesPane();
      message.value = "";
      const first = conflictedFiles.value[0];
      if (first) {
        await selectFile(first);
      }
    }
    return false;
  } finally {
    actionBusy.value = false;
    actionLabel.value = "";
    actionBranch.value = "";
  }
}

function fetchRepo() {
  const match = current.value;
  if (!match) {
    return;
  }
  const name = repoDisplayName(match.repo.id, match.repo.path);
  return runRepoAction("Fetching…", async () => {
    await api.refreshRepo(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id, true);
    return `Fetched ${name}.`;
  });
}

function pullRepo() {
  return runPull();
}

function openPullOptions() {
  if (actionBusy.value) {
    return;
  }
  pullSource.value = "current";
  specifyBranch.value = preferredMergeTarget() || "develop";
  pullingOptions.value = true;
}

function closePullOptions() {
  pullingOptions.value = false;
}

function confirmPull() {
  if (!canConfirmPull.value) {
    return;
  }
  const branch = pullBranch.value;
  closePullOptions();
  return runPull(branch || undefined);
}

async function pushRepo() {
  const match = current.value;
  if (!match || actionBusy.value) {
    return;
  }
  const branch = match.status?.branch ?? "";
  let rejected = false;
  await runRepoAction(
    "Pushing…",
    async () => {
      try {
        return await api.repoPush(match.repo.path);
      } catch (err) {
        rejected = String(err).startsWith(api.PUSH_REJECTED_PREFIX);
        throw err;
      }
    },
    branch,
  );
  if (!rejected) {
    return;
  }
  const behind = current.value?.status?.behind ?? 0;
  const incoming =
    behind === 1 ? "1 commit" : behind > 1 ? `${behind} commits` : "commits";
  const ok = await confirm(
    `The remote branch has ${incoming} you don't have yet, so the push was rejected. Pull them in first, then push again? If the changes conflict, you'll resolve them before anything is pushed.`,
    {
      title: "Push rejected",
      kind: "warning",
      okLabel: "Pull, then push",
      cancelLabel: "Cancel",
    },
  );
  if (!ok || !(await runPull()) || conflictActive.value) {
    return;
  }
  await runRepoAction("Pushing…", () => api.repoPush(match.repo.path), branch);
}

async function undoUnpushedCommits() {
  const match = current.value;
  const ahead = match?.status?.ahead ?? 0;
  if (!match || ahead < 1 || actionBusy.value) {
    return;
  }
  const countLabel = ahead === 1 ? "1 unpushed commit" : `${ahead} unpushed commits`;
  const ok = await confirm(
    `Undo ${countLabel} on this branch? The branch moves back to match the remote, and the changes stay staged. Nothing is removed from the remote.`,
    {
      title: "Undo unpushed commits",
      kind: "warning",
      okLabel: "Undo commits",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  openChangesPane();
  return runRepoAction("Undoing commits…", () => api.resetUnpushedCommits(match.repo.path));
}

async function checkoutBranch(branch: string) {
  const match = current.value;
  if (!match || actionBusy.value) {
    return;
  }
  const previous = match.status?.branch ?? "";
  if (previous === branch) {
    return;
  }
  markOverviewCurrent(branch);
  patchRepoStatus(match.repo.id, { branch });
  actionBusy.value = true;
  actionLabel.value = "Checking out…";
  actionBranch.value = branch;
  message.value = "";
  await nextTick();
  try {
    await api.checkoutLocalBranch(match.repo.path, branch);
  } catch (err) {
    markOverviewCurrent(previous);
    if (previous) {
      patchRepoStatus(match.repo.id, { branch: previous });
    }
    const text = String(err);
    message.value = text;
    showToast(text, "error");
    return;
  } finally {
    actionBusy.value = false;
    actionLabel.value = "";
    actionBranch.value = "";
  }
  graphStale.value = true;
  void loadRepo({
    overview: false,
    graph: !branchesView.value,
    silent: true,
  });
  void refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
}

function checkoutListedBranch(branch: LocalBranch) {
  if (branch.current) {
    return;
  }
  return checkoutBranch(branch.name);
}

async function openCreateBranch(start = "") {
  if (actionBusy.value) {
    return;
  }
  newBranchName.value = "";
  newBranchStart.value = start;
  if (!start.trim()) {
    await refreshBranches();
    baseBranch.value = checkedOutBranch.value || branches.value[0] || "";
  } else {
    baseBranch.value = "";
  }
  creatingBranch.value = true;
  await nextTick();
  newBranchInput.value?.focus();
}

function closeCreateBranch() {
  creatingBranch.value = false;
  newBranchName.value = "";
  baseBranch.value = "";
  newBranchStart.value = "";
}

function createBranch() {
  const match = current.value;
  const branch = newBranchName.value.trim();
  const start = newBranchStart.value.trim() || baseBranch.value.trim();
  if (!match || !branch || !start) {
    return;
  }
  closeCreateBranch();
  return runRepoAction(
    "Creating branch…",
    () => api.createAndCheckoutBranch(match.repo.path, branch, start),
    branch,
  );
}

async function openRenameBranch(branch: LocalBranch) {
  if (actionBusy.value) {
    return;
  }
  renamingBranch.value = branch;
  renameBranchName.value = branch.name;
  await nextTick();
  renameBranchInput.value?.focus();
  renameBranchInput.value?.select();
}

function closeRenameBranch() {
  renamingBranch.value = null;
  renameBranchName.value = "";
}

function pickMergeTarget(exclude = "") {
  const names = localBranchNames.value.filter((name) => name !== exclude);
  const preferred = [mergeTargetHint.value, "develop", "main", "master"];
  for (const name of preferred) {
    if (name && names.includes(name)) {
      return name;
    }
  }
  return names[0] ?? "";
}

function pickMergeSource(preferred: string, target: string) {
  const names = localBranchNames.value;
  if (preferred && preferred !== target && names.includes(preferred)) {
    return preferred;
  }
  const currentName = current.value?.status?.branch ?? "";
  if (currentName && currentName !== target && names.includes(currentName)) {
    return currentName;
  }
  return names.find((name) => name !== target) ?? "";
}

async function openMergeBranch(branch?: LocalBranch) {
  if (actionBusy.value || localBranchNames.value.length < 2) {
    return;
  }
  if (!overview.value) {
    await loadOverview().catch(() => undefined);
  }
  const preferredSource = branch?.name ?? current.value?.status?.branch ?? "";
  const target = pickMergeTarget();
  mergeTarget.value = target;
  mergeSource.value = pickMergeSource(preferredSource, target);
  mergingBranch.value = true;
}

function openMergeIntoCurrent(source: string) {
  const target = checkedOutBranch.value;
  if (actionBusy.value || !target || source === target) {
    return;
  }
  mergeTarget.value = target;
  mergeSource.value = source;
  mergingBranch.value = true;
}

async function pullLocalBranch(name: string) {
  const match = current.value;
  if (!match || actionBusy.value) {
    return;
  }
  if (name === checkedOutBranch.value) {
    await runPull();
    return;
  }
  const upstream = branchTracking.value.find((item) => item.name === name)?.upstream ?? "";
  const path = match.repo.path;
  if (!remotes.value.length) {
    await refreshRemoteList(path);
  }
  // Remote names may contain slashes, so match the longest known remote prefix.
  const remote = remotes.value
    .map((entry) => entry.name)
    .filter((entry) => upstream.startsWith(`${entry}/`))
    .sort((left, right) => right.length - left.length)[0];
  if (!remote) {
    showToast(`Couldn't tell which remote ${name} tracks.`, "error");
    return;
  }
  const remoteBranch = upstream.slice(remote.length + 1);
  await runRepoAction(
    "Pulling…",
    async () => {
      await api.fetchNamedRemote(path, remote);
      return api.mergeRemoteBranch(path, remote, remoteBranch, name, false);
    },
    name,
  );
}

watch(mergeSource, (source) => {
  if (source && source === mergeTarget.value) {
    mergeTarget.value = pickMergeTarget(source);
  }
});

function closeMergeBranch() {
  mergingBranch.value = false;
  mergeSource.value = "";
  mergeTarget.value = "";
}

function mergeLocalBranch() {
  const match = current.value;
  const source = mergeSource.value.trim();
  const target = mergeTarget.value.trim();
  if (!match || !canConfirmMerge.value) {
    return;
  }
  closeMergeBranch();
  return runRepoAction(
    "Merging…",
    () => api.mergeLocalBranch(match.repo.path, source, target),
    target,
  );
}

function renameBranch() {
  const match = current.value;
  const from = renamingBranch.value;
  const to = renameBranchName.value.trim();
  if (!match || !from || !to || to === from.name) {
    return;
  }
  closeRenameBranch();
  return runRepoAction("Renaming…", () => api.renameLocalBranch(match.repo.path, from.name, to));
}

function clipCommitTitle(value: string) {
  return [...value].slice(0, COMMIT_TITLE_MAX).join("");
}

type CommitDraft = { title: string; description: string };
const commitDrafts = new Map<string, CommitDraft>();
const hasCommitDraft = ref(false);

function repoDraftKey() {
  return current.value?.repo.path ?? "";
}

function isMeaningfulDraft(draft: CommitDraft | undefined) {
  return Boolean(draft && (draft.title.trim() || draft.description.trim()));
}

function refreshHasCommitDraft() {
  hasCommitDraft.value = isMeaningfulDraft(commitDrafts.get(repoDraftKey()));
}

function persistCommitDraft() {
  const key = repoDraftKey();
  if (!key) {
    return;
  }
  const title = amending.value ? draftTitle.value : commitTitle.value;
  const description = amending.value ? draftDescription.value : commitDescription.value;
  const draft = { title, description };
  if (isMeaningfulDraft(draft)) {
    commitDrafts.set(key, draft);
  } else {
    commitDrafts.delete(key);
  }
  refreshHasCommitDraft();
}

function loadCommitDraft() {
  const draft = commitDrafts.get(repoDraftKey());
  commitTitle.value = draft?.title ?? "";
  commitDescription.value = draft?.description ?? "";
  refreshHasCommitDraft();
}

function clearCommitDraft() {
  const key = repoDraftKey();
  if (key) {
    commitDrafts.delete(key);
  }
  amending.value = false;
  lastCommit.value = null;
  draftTitle.value = "";
  draftDescription.value = "";
  commitTitle.value = "";
  commitDescription.value = "";
  refreshHasCommitDraft();
}

function applyLastCommitMessage() {
  const last = lastCommit.value;
  if (!last) {
    return;
  }
  commitTitle.value = clipCommitTitle(last.title);
  commitDescription.value = last.description;
}

async function openCommit() {
  if (actionBusy.value || !files.value.length) {
    return;
  }
  if (operation.value && operation.value !== "merge") {
    return;
  }
  amending.value = false;
  lastCommit.value = null;
  loadCommitDraft();
  committing.value = true;
  const match = current.value;
  if (match) {
    lastCommit.value = await api.lastCommit(match.repo.path).catch(() => null);
  }
  await nextTick();
  commitTitleInput.value?.focus();
}

function closeCommit() {
  persistCommitDraft();
  amending.value = false;
  lastCommit.value = null;
  committing.value = false;
  loadCommitDraft();
}

function onAmendChange(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  if (checked) {
    draftTitle.value = commitTitle.value;
    draftDescription.value = commitDescription.value;
    amending.value = true;
    applyLastCommitMessage();
    return;
  }
  commitTitle.value = draftTitle.value;
  commitDescription.value = draftDescription.value;
  amending.value = false;
}

async function commitChanges(options?: { all?: boolean }) {
  const match = current.value;
  const title = commitTitle.value.trim();
  const all = Boolean(options?.all);
  if (!match || !title || !(all ? canCommitAll.value : canCommit.value)) {
    return;
  }
  const description = commitDescription.value;
  const amend = amending.value;
  if (amend && lastCommit.value?.published) {
    const ok = await confirm(
      "This commit is already on the remote. Amending rewrites history, and you will need to force-push.",
      {
        title: "Amend published commit",
        kind: "warning",
        okLabel: "Amend",
        cancelLabel: "Cancel",
      },
    );
    if (!ok) {
      return;
    }
  }
  closeCommit();
  return runRepoAction(amend ? "Amending…" : "Committing…", async () => {
    if (all) {
      await api.stageAll(match.repo.path);
    }
    const result = await api.commit(match.repo.path, title, description, amend);
    clearCommitDraft();
    return result;
  });
}

async function openStash() {
  if (actionBusy.value || !files.value.length) {
    return;
  }
  stashFileTargets.value = [];
  stashMessage.value = "";
  stashing.value = true;
  await nextTick();
  stashMessageInput.value?.focus();
}

function closeStash() {
  stashing.value = false;
  stashMessage.value = "";
  stashFileTargets.value = [];
}

function stashChanges() {
  const match = current.value;
  if (!match || !files.value.length) {
    return;
  }
  const message = stashMessage.value;
  const paths = stashFileTargets.value.map((file) => file.path);
  closeStash();
  closeDiff();
  if (paths.length) {
    return runRepoAction("Stashing…", () => api.stashFiles(match.repo.path, paths, message));
  }
  return runRepoAction("Stashing…", () => api.stashPush(match.repo.path, message));
}

function preferredMergeTarget() {
  return current.value?.group?.pullFromBranch?.trim() || undefined;
}

function removeOverviewBranches(names: string[]) {
  const gone = new Set(names);
  if (!overview.value || !gone.size) {
    return;
  }
  overview.value = {
    ...overview.value,
    branches: overview.value.branches.filter((branch) => !gone.has(branch.name)),
  };
}

async function selectMainTab(tab: RepoViewTab) {
  if (mainTab.value === tab) {
    return;
  }
  if (selectedStash.value) {
    closeCommitDetail();
  }
  mainTab.value = tab;
  if (tab === "terminal") {
    terminalStarted.value = true;
    return;
  }
  if (tab === "commits") {
    if (graphStale.value) {
      void loadRepo({ overview: false, silent: true });
    }
    return;
  }
  if (tab === "branches") {
    await nextTick();
    try {
      await loadOverview();
    } catch (err) {
      message.value = String(err);
    }
    return;
  }
  if (tab === "remotes") {
    const match = current.value;
    if (!match) {
      return;
    }
    await refreshRemoteList(match.repo.path);
    await loadRemoteOverview();
    void fetchSelectedRemote({ quiet: true });
  }
}

function openChangesPane() {
  historyOpen.value = false;
  closeHistoryFile();
  lastHistoryFile.value = "";
}

function openHistoryPane() {
  if (historyOpen.value) {
    return;
  }
  historyOpen.value = true;
  void loadRepoFiles();
}

function closeHistoryFile() {
  selectedHistoryFile.value = "";
  historyCommits.value = [];
  historyCommitsError.value = "";
  historyCommitsGeneration += 1;
  selectedHistoryCommit.value = null;
  if (!selectedFile.value && !selectedCommitFile.value) {
    diff.value = "";
  }
}

async function selectHistoryFile(path: string) {
  const match = current.value;
  if (!match) {
    return;
  }
  closeCommitDetail();
  selectedFile.value = null;
  selectedHistoryFile.value = path;
  lastHistoryFile.value = path;
  selectedHistoryCommit.value = null;
  diff.value = "";
  const generation = ++historyCommitsGeneration;
  historyCommitsLoading.value = true;
  historyCommitsError.value = "";
  try {
    const next = await api.fileLog(match.repo.path, path);
    if (generation !== historyCommitsGeneration) {
      return;
    }
    historyCommits.value = next;
  } catch (err) {
    if (generation !== historyCommitsGeneration) {
      return;
    }
    historyCommits.value = [];
    historyCommitsError.value = String(err);
  } finally {
    if (generation === historyCommitsGeneration) {
      historyCommitsLoading.value = false;
    }
  }
}

async function selectHistoryCommit(commit: CommitNode) {
  const match = current.value;
  const file = commit.path || selectedHistoryFile.value;
  if (!match || !file) {
    return;
  }
  if (selectedHistoryCommit.value?.hash === commit.hash) {
    closeDiff();
    return;
  }
  selectedFile.value = null;
  selectedCommitFile.value = null;
  selectedHistoryCommit.value = commit;
  try {
    diff.value = await api.commitFileDiff(match.repo.path, commit.hash, file);
  } catch (err) {
    diff.value = String(err);
  }
}

async function loadRepoFiles(options?: { silent?: boolean }) {
  const match = current.value;
  if (!match) {
    repoFiles.value = [];
    repoFilesError.value = "";
    return;
  }
  const generation = ++repoFilesGeneration;
  if (!options?.silent) {
    repoFilesLoading.value = true;
  }
  repoFilesError.value = "";
  try {
    const next = await api.repoFiles(match.repo.path);
    if (generation !== repoFilesGeneration) {
      return;
    }
    repoFiles.value = next;
  } catch (err) {
    if (generation !== repoFilesGeneration) {
      return;
    }
    if (!options?.silent) {
      repoFiles.value = [];
      repoFilesError.value = String(err);
    }
  } finally {
    if (generation === repoFilesGeneration) {
      repoFilesLoading.value = false;
    }
  }
}

function stashRef(index: number) {
  return `stash@{${index}}`;
}

async function applyStash(stash: StashEntry) {
  const match = current.value;
  if (!match) {
    return;
  }
  if (files.value.length) {
    const ok = await confirm(
      `You have uncommitted changes. Applying ${stashRef(stash.index)} may cause conflicts.`,
      {
        title: "Apply stash",
        kind: "warning",
        okLabel: "Apply",
        cancelLabel: "Cancel",
      },
    );
    if (!ok) {
      return;
    }
  }
  return runRepoAction("Applying stash…", () => api.stashApply(match.repo.path, stash.index));
}

async function popStash(stash: StashEntry) {
  const match = current.value;
  if (!match) {
    return;
  }
  const ok = await confirm(
    files.value.length
      ? `You have uncommitted changes. Pop ${stashRef(stash.index)} anyway? It will be removed if it applies cleanly.`
      : `Apply ${stashRef(stash.index)} and remove it from the stash list?`,
    {
      title: "Pop stash",
      kind: "warning",
      okLabel: "Pop",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  return runRepoAction("Popping stash…", () => api.stashPop(match.repo.path, stash.index));
}

async function dropStash(stash: StashEntry) {
  const match = current.value;
  if (!match) {
    return;
  }
  const ok = await confirm(
    `Permanently delete ${stashRef(stash.index)}? This cannot be undone.`,
    {
      title: "Drop stash",
      kind: "warning",
      okLabel: "Drop",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  return runRepoAction("Dropping stash…", () => api.stashDrop(match.repo.path, stash.index));
}

async function openCreateTag(target = "") {
  if (actionBusy.value) {
    return;
  }
  newTagName.value = "";
  newTagMessage.value = "";
    newTagTarget.value = target || selectedCommit.value?.hash.slice(0, 12) || "";
  creatingTag.value = true;
  await nextTick();
  newTagInput.value?.focus();
}

function openCommitMenu(commit: CommitNode, hashes: string[], x: number, y: number) {
  commitMenu.value = { commit, hashes, x, y };
}

function closeCommitMenu() {
  commitMenu.value = null;
}

function menuHashes(oldestFirst: boolean) {
  const hashes = commitMenu.value?.hashes ?? [];
  const index = new Map(commits.value.map((commit, i) => [commit.hash, i]));
  return [...hashes].sort((left, right) => {
    const a = index.get(left) ?? 0;
    const b = index.get(right) ?? 0;
    return oldestFirst ? b - a : a - b;
  });
}

function checkoutMenuCommit() {
  const match = current.value;
  const commit = commitMenu.value?.commit;
  closeCommitMenu();
  if (!match || !commit) {
    return;
  }
  return runRepoAction("Checking out…", () => api.checkoutCommit(match.repo.path, commit.hash));
}

function createBranchFromMenu() {
  const hash = commitMenu.value?.commit.hash ?? "";
  closeCommitMenu();
  if (!hash) {
    return;
  }
  return openCreateBranch(hash);
}

function cherryPickMenuCommits() {
  const match = current.value;
  const hashes = menuHashes(true);
  closeCommitMenu();
  if (!match || !hashes.length) {
    return;
  }
  return runRepoAction("Cherry-picking…", () => api.cherryPickCommits(match.repo.path, hashes));
}

function revertMenuCommits() {
  const match = current.value;
  const hashes = menuHashes(false);
  closeCommitMenu();
  if (!match || !hashes.length) {
    return;
  }
  return runRepoAction("Reverting…", () => api.revertCommits(match.repo.path, hashes));
}

async function copyMenuShas() {
  const hashes = commitMenu.value?.hashes ?? [];
  closeCommitMenu();
  if (!hashes.length) {
    return;
  }
  try {
    await navigator.clipboard.writeText(hashes.join("\n"));
    showToast(hashes.length === 1 ? "Copied commit SHA" : `Copied ${hashes.length} commit SHAs`);
  } catch (err) {
    showToast(String(err), "error");
  }
}

async function copyMenuLink() {
  const match = current.value;
  const hash = commitMenu.value?.commit.hash ?? "";
  closeCommitMenu();
  if (!match || !hash) {
    return;
  }
  try {
    const url = await api.commitRemoteUrl(match.repo.path, hash);
    await navigator.clipboard.writeText(url);
    showToast("Copied commit link");
  } catch (err) {
    showToast(String(err), "error");
  }
}

function createTagFromMenu() {
  const hash = commitMenu.value?.commit.hash ?? "";
  closeCommitMenu();
  if (!hash) {
    return;
  }
  return openCreateTag(hash);
}

function closeCreateTag() {
  creatingTag.value = false;
  newTagName.value = "";
  newTagMessage.value = "";
  newTagTarget.value = "";
}

function createTag() {
  const match = current.value;
  const name = newTagName.value.trim();
  if (!match || !name) {
    return;
  }
  const message = newTagMessage.value;
  const target = newTagTarget.value.trim();
  closeCreateTag();
  return runRepoAction("Creating tag…", () => api.createTag(match.repo.path, name, message, target));
}

async function deleteTag(tag: TagEntry) {
  const match = current.value;
  if (!match) {
    return;
  }
  const ok = await confirm(`Permanently delete tag ${tag.name}? This cannot be undone.`, {
    title: "Delete tag",
    kind: "warning",
    okLabel: "Delete",
    cancelLabel: "Cancel",
  });
  if (!ok) {
    return;
  }
  return runRepoAction("Deleting tag…", () => api.deleteTag(match.repo.path, tag.name));
}

function remoteStorageKey(path: string) {
  return `shipyard:remote:${path}`;
}

function rememberedRemote(path: string) {
  try {
    return localStorage.getItem(remoteStorageKey(path)) ?? "";
  } catch {
    return "";
  }
}

function rememberRemote(path: string, name: string) {
  try {
    localStorage.setItem(remoteStorageKey(path), name);
  } catch {
    /* selection just won't persist */
  }
}

function pickRemote(list: RemoteEntry[], path: string) {
  const names = list.map((remote) => remote.name);
  for (const name of [selectedRemote.value, rememberedRemote(path), "origin"]) {
    if (name && names.includes(name)) {
      return name;
    }
  }
  return names[0] ?? "";
}

async function refreshRemoteList(path: string) {
  try {
    const list = await api.listRemotes(path);
    if (current.value?.repo.path !== path) {
      return;
    }
    remotes.value = list;
    selectedRemote.value = pickRemote(list, path);
  } catch {
    /* keep the last successful list */
  }
}

async function loadRemoteOverview() {
  const match = current.value;
  const remote = selectedRemote.value;
  const generation = ++remoteGeneration;
  if (!match || !remote) {
    remoteOverview.value = null;
    remoteLoading.value = false;
    return;
  }
  remoteLoading.value = true;
  try {
    const next = await api.remoteBranches(match.repo.path, remote);
    if (generation === remoteGeneration) {
      remoteOverview.value = next;
    }
  } catch (err) {
    if (generation === remoteGeneration) {
      remoteOverview.value = null;
      message.value = String(err);
    }
  } finally {
    if (generation === remoteGeneration) {
      remoteLoading.value = false;
    }
  }
}

async function fetchSelectedRemote(options?: { quiet?: boolean }) {
  const match = current.value;
  const remote = selectedRemote.value;
  if (!match || !remote || remoteFetching.value) {
    return;
  }
  const path = match.repo.path;
  remoteFetching.value = true;
  try {
    const result = await api.fetchNamedRemote(path, remote);
    if (!options?.quiet) {
      showToast(result);
    }
  } catch (err) {
    showToast(String(err), "error");
  } finally {
    remoteFetching.value = false;
  }
  if (current.value?.repo.path !== path) {
    return;
  }
  graphStale.value = true;
  await refreshRemoteList(path);
  if (selectedRemote.value === remote) {
    await loadRemoteOverview();
  }
  void refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
}

function selectRemote(name: string) {
  const match = current.value;
  if (!match || name === selectedRemote.value) {
    return;
  }
  selectedRemote.value = name;
  rememberRemote(match.repo.path, name);
  void loadRemoteOverview().then(() => fetchSelectedRemote({ quiet: true }));
}

async function runRemoteAction(label: string, work: () => Promise<string>, branch = "") {
  const match = current.value;
  if (!match) {
    return;
  }
  await runRepoAction(label, work, branch);
  await refreshRemoteList(match.repo.path);
  await loadRemoteOverview();
}

async function openRemoteUrl(url: string) {
  try {
    await openUrl(url);
  } catch (err) {
    showToast(String(err), "error");
  }
}

function suggestedRemoteName() {
  const names = new Set(remotes.value.map((remote) => remote.name));
  if (!names.has("origin")) {
    return "origin";
  }
  return names.has("upstream") ? "" : "upstream";
}

async function openAddRemote() {
  if (actionBusy.value) {
    return;
  }
  remoteForm.value = { mode: "add", original: "" };
  remoteFormName.value = suggestedRemoteName();
  remoteFormUrl.value = "";
  await nextTick();
  remoteFormNameInput.value?.focus();
  remoteFormNameInput.value?.select();
}

async function openEditRemote(remote: RemoteEntry) {
  if (actionBusy.value) {
    return;
  }
  remoteForm.value = { mode: "edit", original: remote.name };
  remoteFormName.value = remote.name;
  remoteFormUrl.value = remote.fetchUrl;
  await nextTick();
  remoteFormNameInput.value?.focus();
}

function closeRemoteForm() {
  remoteForm.value = null;
  remoteFormName.value = "";
  remoteFormUrl.value = "";
}

const canSubmitRemoteForm = computed(() => {
  const name = remoteFormName.value.trim();
  const url = remoteFormUrl.value.trim();
  if (!name || !url) {
    return false;
  }
  const form = remoteForm.value;
  const taken = remotes.value.some((remote) => remote.name === name);
  return form?.mode === "edit" ? name === form.original || !taken : !taken;
});

function submitRemoteForm() {
  const match = current.value;
  const form = remoteForm.value;
  const name = remoteFormName.value.trim();
  const url = remoteFormUrl.value.trim();
  if (!match || !form || !canSubmitRemoteForm.value) {
    return;
  }
  closeRemoteForm();
  const path = match.repo.path;
  if (form.mode === "add") {
    return runRemoteAction("Adding remote…", async () => {
      const result = await api.addRemote(path, name, url);
      selectedRemote.value = name;
      rememberRemote(path, name);
      return result;
    });
  }
  return runRemoteAction("Updating remote…", async () => {
    const result = await api.updateRemote(path, form.original, name, url);
    if (selectedRemote.value === form.original) {
      selectedRemote.value = name;
      rememberRemote(path, name);
    }
    return result;
  });
}

async function removeRemote(remote: RemoteEntry) {
  const match = current.value;
  if (!match || actionBusy.value) {
    return;
  }
  const trackers =
    remoteOverview.value?.remote === remote.name
      ? remoteOverview.value.branches.filter((branch) => branch.tracked && branch.local)
      : [];
  const trackerNote = trackers.length
    ? ` Local branches that track it stop tracking anything: ${trackers.map((branch) => branch.local).join(", ")}.`
    : "";
  const ok = await confirm(
    `Remove ${remote.name} from this repository? Its remote-tracking branches are deleted here.${trackerNote} Nothing on the server changes, and you can add it again later.`,
    {
      title: "Remove remote",
      kind: "warning",
      okLabel: "Remove",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  return runRemoteAction("Removing remote…", () => api.removeRemote(match.repo.path, remote.name));
}

async function checkoutRemote(branch: RemoteBranch) {
  const match = current.value;
  if (!match || actionBusy.value || branch.current) {
    return;
  }
  if (branch.local && !branch.tracked) {
    namingCheckout.value = branch;
    checkoutLocalName.value = `${branch.remote}-${branch.name}`;
    await nextTick();
    checkoutLocalNameInput.value?.focus();
    checkoutLocalNameInput.value?.select();
    return;
  }
  return runRemoteAction(
    "Checking out…",
    () => api.checkoutRemoteBranch(match.repo.path, branch.remote, branch.name),
    branch.local ?? branch.name,
  );
}

function closeCheckoutNaming() {
  namingCheckout.value = null;
  checkoutLocalName.value = "";
}

const canConfirmCheckoutNaming = computed(() => {
  const name = checkoutLocalName.value.trim();
  return Boolean(name) && !branches.value.includes(name);
});

function confirmCheckoutNaming() {
  const match = current.value;
  const branch = namingCheckout.value;
  const localName = checkoutLocalName.value.trim();
  if (!match || !branch || !canConfirmCheckoutNaming.value) {
    return;
  }
  closeCheckoutNaming();
  return runRemoteAction(
    "Checking out…",
    () => api.checkoutRemoteBranch(match.repo.path, branch.remote, branch.name, localName),
    localName,
  );
}

const syncSourceLabel = computed(() => {
  const branch = syncingBranch.value;
  return branch ? `${branch.remote}/${branch.name}` : "";
});

const syncTargetUpstream = computed(
  () => branchTracking.value.find((item) => item.name === syncTarget.value)?.upstream ?? "",
);

const showSyncPush = computed(
  () => Boolean(syncTarget.value) && syncTargetUpstream.value !== syncSourceLabel.value,
);

const syncPushLabel = computed(() => {
  const upstream = syncTargetUpstream.value;
  return upstream
    ? `Push ${syncTarget.value} to ${upstream} afterward`
    : `Push ${syncTarget.value} to origin afterward and track it there`;
});

async function openSyncBranch(branch: RemoteBranch) {
  if (actionBusy.value) {
    return;
  }
  await refreshBranches();
  const names = branches.value;
  const preferred = [branch.local, checkedOutBranch.value, names[0]];
  syncTarget.value = preferred.find((name) => name && names.includes(name)) ?? "";
  syncAllowMerge.value = false;
  syncPush.value = true;
  syncingBranch.value = branch;
}

function closeSync() {
  syncingBranch.value = null;
  syncTarget.value = "";
}

async function confirmSync() {
  const match = current.value;
  const source = syncingBranch.value;
  const target = syncTarget.value.trim();
  if (!match || !source || !target) {
    return;
  }
  const path = match.repo.path;
  const push = showSyncPush.value && syncPush.value;
  let allowMerge = syncAllowMerge.value;
  closeSync();

  const attempt = async (): Promise<{ merged: boolean; needsMergeCommit: boolean }> => {
    let merged = false;
    let needsMergeCommit = false;
    await runRepoAction(
      allowMerge ? "Merging…" : "Syncing…",
      async () => {
        try {
          const result = await api.mergeRemoteBranch(path, source.remote, source.name, target, allowMerge);
          merged = true;
          return result;
        } catch (err) {
          needsMergeCommit = String(err).startsWith(api.NOT_FAST_FORWARD_PREFIX);
          throw err;
        }
      },
      target,
    );
    return { merged, needsMergeCommit };
  };

  let outcome = await attempt();
  if (!outcome.merged && outcome.needsMergeCommit && !allowMerge) {
    const ok = await confirm(
      `${target} has commits that aren't on ${source.remote}/${source.name}, so it can't just fast-forward. Merge anyway? That makes a merge commit on ${target}. If the changes conflict, you'll resolve them before anything is pushed.`,
      {
        title: "Can't fast-forward",
        kind: "warning",
        okLabel: "Merge",
        cancelLabel: "Cancel",
      },
    );
    if (ok) {
      allowMerge = true;
      outcome = await attempt();
    }
  }
  if (outcome.merged && push && !conflictActive.value) {
    await runRepoAction("Pushing…", () => api.pushLocalBranch(path, target), target);
  }
  await refreshRemoteList(path);
  await loadRemoteOverview();
}

async function deleteRemoteBranch(branch: RemoteBranch) {
  const match = current.value;
  if (!match || actionBusy.value || branch.isDefault) {
    return;
  }
  const ok = await confirm(
    `Delete ${branch.name} on ${branch.remote}? This removes the branch from the server for everyone who uses ${branch.remote}. Local branches are not touched.`,
    {
      title: "Delete remote branch",
      kind: "warning",
      okLabel: "Delete on server",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  return runRemoteAction(
    "Deleting remote branch…",
    () => api.deleteRemoteBranch(match.repo.path, branch.remote, branch.name),
  );
}

async function refreshBranches() {
  const match = current.value;
  if (!match) {
    return;
  }
  try {
    branches.value = await api.listLocalBranches(match.repo.path);
    rememberTrackingPath(match.repo.path);
    void loadBranchTracking(match.repo.path);
  } catch {
    /* keep the last successful list */
  }
}

function rememberTrackingPath(path: string) {
  if (trackingPath.value === path) {
    return;
  }
  trackingPath.value = path;
  trackingGeneration += 1;
  branchTracking.value = [];
}

async function loadBranchTracking(path: string) {
  const generation = ++trackingGeneration;
  try {
    const next = await api.listBranchTracking(path);
    if (generation !== trackingGeneration) {
      return;
    }
    branchTracking.value = next;
  } catch {
    /* keep the last successful list */
  }
}

function notFullyMerged(err: unknown) {
  return String(err).toLowerCase().includes("not fully merged");
}

async function deleteBranch(branch: LocalBranch) {
  const match = current.value;
  if (!match || branch.current) {
    return;
  }
  const target = overview.value?.mergeTarget ?? "the integration branch";
  const force = !branch.merged;
  const ok = await confirm(
    force
      ? branch.partial
        ? `${branch.name} is only partially merged into ${target}. Some commits are still unique. Delete this local branch anyway?`
        : `${branch.name} is not fully merged into ${target}. Delete this local branch anyway?`
      : branch.protected
        ? `Delete local branch ${branch.name}? This is a protected integration branch.`
        : `Delete local branch ${branch.name}? It is already merged into ${target}.`,
    {
      title: force
        ? branch.partial
          ? "Delete partial branch"
          : "Delete unmerged branch"
        : "Delete branch",
      kind: "warning",
      okLabel: "Delete",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  return runRepoAction("Deleting…", async () => {
    try {
      return await api.deleteLocalBranch(match.repo.path, branch.name, force);
    } catch (err) {
      if (
        force ||
        !notFullyMerged(err) ||
        !(await confirm(
          `${branch.name} is marked merged into ${target}, but git will not delete it safely. Force delete this local branch?`,
          {
            title: "Force delete branch",
            kind: "warning",
            okLabel: "Force delete",
            cancelLabel: "Keep",
          },
        ))
      ) {
        throw err;
      }
      return api.deleteLocalBranch(match.repo.path, branch.name, true);
    }
  });
}

function selectedDeleteCopy(branches: LocalBranch[], target: string) {
  const merged = branches.filter((branch) => branch.merged).length;
  const partial = branches.filter((branch) => !branch.merged && branch.partial).length;
  const unique = branches.filter((branch) => !branch.merged && !branch.partial).length;
  const bits: string[] = [];
  if (merged) {
    bits.push(
      `${merged} ${merged === 1 ? "is" : "are"} already merged into ${target}`,
    );
  }
  if (partial) {
    bits.push(`${partial} ${partial === 1 ? "is" : "are"} only partially merged`);
  }
  if (unique) {
    bits.push(`${unique} ${unique === 1 ? "has" : "have"} unique work`);
  }
  const noun = branches.length === 1 ? "branch" : "branches";
  return `Delete ${branches.length} local ${noun}?${bits.length ? ` ${bits.join(". ")}.` : ""}`;
}

async function deleteSelectedBranches(branches: LocalBranch[]) {
  const match = current.value;
  const victims = branches.filter((branch) => !branch.current && !branch.protected);
  if (!match || !victims.length) {
    return;
  }
  const target = overview.value?.mergeTarget ?? "the integration branch";
  const ok = await confirm(selectedDeleteCopy(victims, target), {
    title: "Delete selected branches",
    kind: "warning",
    okLabel: "Delete",
    cancelLabel: "Cancel",
  });
  if (!ok) {
    return;
  }
  if (actionBusy.value) {
    return;
  }
  actionBusy.value = true;
  actionLabel.value = "Deleting selected branches…";
  message.value = "";
  try {
    const safe = victims.filter((branch) => branch.merged).map((branch) => branch.name);
    const forced = victims.filter((branch) => !branch.merged).map((branch) => branch.name);
    const deleted: string[] = [];
    const errors: string[] = [];
    const notes: string[] = [];

    if (safe.length) {
      let result = await api.deleteMergedBranches(
        match.repo.path,
        preferredMergeTarget(),
        false,
        safe,
      );
      deleted.push(...result.deleted);
      errors.push(...result.errors);
      if (result.message) {
        notes.push(result.message);
      }
      if (result.refused.length) {
        const refused = result.refused.join(", ");
        const forceOk = await confirm(
          result.deleted.length
            ? `Deleted ${result.deleted.length}. Git would not safely delete ${refused}. Those branches are already contained in ${target}, but not fully merged into the branch you're on (squash merges and unmerged remotes do this). Force delete them?`
            : `Git would not safely delete ${refused}. Those leftover branches are already contained in ${target}, but not fully merged into the branch you're on (squash merges and unmerged remotes do this). Force delete them?`,
          {
            title: "Force delete leftover branches",
            kind: "warning",
            okLabel: "Force delete",
            cancelLabel: "Keep",
          },
        );
        if (forceOk) {
          const extra = await api.deleteMergedBranches(
            match.repo.path,
            preferredMergeTarget(),
            true,
            result.refused,
          );
          deleted.push(...extra.deleted);
          errors.push(...extra.errors);
          if (extra.message) {
            notes.push(extra.message);
          }
        }
      }
    }

    if (forced.length) {
      const result = await api.deleteMergedBranches(
        match.repo.path,
        preferredMergeTarget(),
        true,
        forced,
      );
      deleted.push(...result.deleted);
      errors.push(...result.errors);
      if (result.message) {
        notes.push(result.message);
      }
    }

    const text = notes.filter(Boolean).join(" ") || `Deleted ${deleted.length} local branches.`;
    const failed = deleted.length === 0 && errors.length > 0;
    if (failed) {
      message.value = text;
    }
    showToast(text, failed ? "error" : "success");
    overviewGeneration += 1;
    removeOverviewBranches(deleted);
    await loadRepo({ overview: false, graph: !branchesView.value, silent: true });
    if (branchesView.value && overview.value?.branches.some((branch) => branch.pending)) {
      void loadOverview();
    }
    await refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
  } catch (err) {
    const text = String(err);
    message.value = text;
    showToast(text, "error");
  } finally {
    actionBusy.value = false;
    actionLabel.value = "";
  }
}

async function deleteMerged() {
  const match = current.value;
  const count =
    overview.value?.branches.filter(
      (branch) => branch.merged && !branch.current && !branch.protected && !branch.pending,
    ).length ?? 0;
  if (!match || count === 0) {
    return;
  }
  const target = overview.value?.mergeTarget ?? "the integration branch";
  const ok = await confirm(
    `Delete ${count} leftover local ${count === 1 ? "branch" : "branches"} already merged into ${target}? Partial and unique branches stay. This never deletes develop, main, master, or the current branch.`,
    {
      title: "Delete merged branches",
      kind: "warning",
      okLabel: "Delete",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  if (actionBusy.value) {
    return;
  }
  actionBusy.value = true;
  actionLabel.value = "Deleting merged branches…";
  message.value = "";
  try {
    const victims =
      overview.value?.branches
        .filter((branch) => branch.merged && !branch.current && !branch.protected && !branch.pending)
        .map((branch) => branch.name) ?? [];
    let result = await api.deleteMergedBranches(
      match.repo.path,
      preferredMergeTarget(),
      false,
      victims,
    );
    if (result.refused.length) {
      const refused = result.refused.join(", ");
      const forceOk = await confirm(
        result.deleted.length
          ? `Deleted ${result.deleted.length}. Git would not safely delete ${refused}. Those branches are already contained in ${target}, but not fully merged into the branch you're on (squash merges and unmerged remotes do this). Force delete them?`
          : `Git would not safely delete ${refused}. Those leftover branches are already contained in ${target}, but not fully merged into the branch you're on (squash merges and unmerged remotes do this). Force delete them?`,
        {
          title: "Force delete leftover branches",
          kind: "warning",
          okLabel: "Force delete",
          cancelLabel: "Keep",
        },
      );
      if (forceOk) {
        const forced = await api.deleteMergedBranches(
          match.repo.path,
          preferredMergeTarget(),
          true,
          result.refused,
        );
        result = {
          deleted: [...result.deleted, ...forced.deleted],
          refused: forced.refused,
          errors: [...result.errors, ...forced.errors],
          message: [result.message, forced.message].filter(Boolean).join(" "),
        };
      }
    }
    const failed = result.deleted.length === 0 && result.errors.length > 0;
    const text = result.message;
    if (failed) {
      message.value = text;
    }
    showToast(text, failed ? "error" : "success");
    overviewGeneration += 1;
    removeOverviewBranches(result.deleted);
    await loadRepo({ overview: false, graph: !branchesView.value, silent: true });
    if (branchesView.value && overview.value?.branches.some((branch) => branch.pending)) {
      void loadOverview();
    }
    await refreshRepoStatus(match.group?.id ?? STANDALONE_GROUP_ID, match.repo.id);
  } catch (err) {
    const text = String(err);
    message.value = text;
    showToast(text, "error");
  } finally {
    actionBusy.value = false;
    actionLabel.value = "";
  }
}

async function openInEditor(file: WorkingTreeFile) {
  const match = current.value;
  if (!match) {
    return;
  }
  try {
    await api.openInEditor(match.repo.path, file.path);
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  }
}

async function ignoreFiles(targets: WorkingTreeFile[], kind: IgnoreKind) {
  const match = current.value;
  if (!match || !targets.length) {
    return;
  }
  try {
    await runWorktreeMutation(() =>
      forEachFile(targets, async (file) => {
        await api.ignoreWorkingTreePath(match.repo.path, file.path, kind);
        if (selectedFile.value?.path === file.path) {
          closeDiff();
        }
      }),
    );
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  }
}

async function stashFiles(targets: WorkingTreeFile[]) {
  if (!current.value || actionBusy.value || !targets.length) {
    return;
  }
  stashFileTargets.value = targets;
  stashMessage.value = targets.length === 1 ? targets[0].path : "";
  stashing.value = true;
  await nextTick();
  stashMessageInput.value?.focus();
  stashMessageInput.value?.select();
}

async function revealFile(file: WorkingTreeFile) {
  const match = current.value;
  if (!match) {
    return;
  }
  try {
    await api.revealFileInFinder(match.repo.path, file.path);
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  }
}

async function copyFilePath(file: WorkingTreeFile) {
  const match = current.value;
  if (!match) {
    return;
  }
  try {
    await navigator.clipboard.writeText(absoluteFilePath(match.repo.path, file.path));
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  }
}

function discardPrompt(targets: WorkingTreeFile[]) {
  if (targets.length > 1) {
    const deletes = targets.some(
      (file) => file.untracked || (file.staged && file.status === "Added"),
    );
    const note = deletes ? " New and untracked files among them will be deleted." : "";
    return `Discard changes in ${targets.length} files? This cannot be undone.${note}\n\n${fileListPreview(targets)}`;
  }
  const file = targets[0];
  const name = fileBasename(file.path);
  if (file.untracked) {
    return `Discard ${name}? This untracked file will be deleted.`;
  }
  if (file.staged && file.status === "Added") {
    return `Discard ${name}? This new file will be deleted.`;
  }
  if (file.staged && file.status === "Renamed") {
    return `Discard the rename of ${name}? It will go back to its original name and its staged edits will be lost.`;
  }
  return `Discard changes to ${name}? This cannot be undone.`;
}

async function discardFiles(targets: WorkingTreeFile[]) {
  const match = current.value;
  if (!match || !targets.length) {
    return;
  }
  const ok = await confirm(discardPrompt(targets), {
    title: "Discard changes",
    kind: "warning",
    okLabel: "Discard",
    cancelLabel: "Cancel",
  });
  if (!ok) {
    return;
  }
  try {
    await runWorktreeMutation(() =>
      forEachFile(targets, (file) =>
        api.discardFileChanges(match.repo.path, file.path, file.staged),
      ),
    );
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  }
}

async function deleteFiles(targets: WorkingTreeFile[]) {
  const match = current.value;
  if (!match || !targets.length) {
    return;
  }
  const prompt =
    targets.length === 1
      ? `Delete ${fileBasename(targets[0].path)}? This cannot be undone.`
      : `Delete ${targets.length} files? This cannot be undone.\n\n${fileListPreview(targets)}`;
  const ok = await confirm(prompt, {
    title: "Delete file",
    kind: "warning",
    okLabel: "Delete",
    cancelLabel: "Cancel",
  });
  if (!ok) {
    return;
  }
  try {
    await runWorktreeMutation(() =>
      forEachFile(targets, async (file) => {
        await api.deleteWorkingTreeFile(match.repo.path, file.path);
        if (selectedFile.value?.path === file.path) {
          closeDiff();
        }
      }),
    );
  } catch (err) {
    message.value = String(err);
    showToast(String(err), "error");
  }
}

async function abortCurrentOperation() {
  const match = current.value;
  if (!match || !operation.value || actionBusy.value) {
    return;
  }
  const noun = operationNoun(operation.value);
  const ok = await confirm(
    `Abort this ${noun}? The repository returns to the state before the ${noun} started.`,
    {
      title: abortLabel(operation.value),
      kind: "warning",
      okLabel: abortLabel(operation.value),
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  closeDiff();
  return runRepoAction("Aborting…", () => api.abortOperation(match.repo.path));
}

async function continueCurrentOperation() {
  const match = current.value;
  if (!match || !operation.value || actionBusy.value || conflictedCount.value) {
    return;
  }
  closeDiff();
  return runRepoAction("Continuing…", () => api.continueOperation(match.repo.path));
}

async function discardAll() {
  const match = current.value;
  if (!match || !files.value.length) {
    return;
  }
  const ok = await confirm(
    "Discard all uncommitted changes? Tracked files will be reset and untracked files will be deleted.",
    {
      title: "Discard all changes",
      kind: "warning",
      okLabel: "Discard",
      cancelLabel: "Cancel",
    },
  );
  if (!ok) {
    return;
  }
  try {
    await runWorktreeMutation(async () => {
      await api.discardAllChanges(match.repo.path);
      closeDiff();
    });
  } catch (err) {
    message.value = String(err);
  }
}

watch(
  () => props.repoId,
  () => {
    mainTab.value = "commits";
    remoteGeneration += 1;
    remotes.value = [];
    selectedRemote.value = "";
    remoteOverview.value = null;
    remoteLoading.value = false;
    remoteFetching.value = false;
    historyOpen.value = false;
    repoFiles.value = [];
    repoFilesError.value = "";
    repoFilesGeneration += 1;
    closeHistoryFile();
    lastHistoryFile.value = "";
    terminalStarted.value = false;
    graphStale.value = false;
    overviewGeneration += 1;
    overview.value = null;
    trackingGeneration += 1;
    branchTracking.value = [];
    trackingPath.value = "";
    closeCommitDetail();
    closeDiff();
    closeCommitMenu();
  },
);

watch(
  () => [props.repoId, groups.value, standaloneRepos.value, loaded.value],
  () => {
    void loadRepo();
  },
  { immediate: true },
);

watch(
  () => current.value?.repo.path ?? "",
  (path) => {
    void startWatching(path);
    refreshHasCommitDraft();
  },
  { immediate: true },
);

watch(
  () => files.value.length,
  (count, previous) => {
    if (count === 0 && (previous ?? 0) > 0 && !committing.value) {
      clearCommitDraft();
    }
  },
);

void listen<RepoFilesChanged>("repo-files-changed", (event) => {
  void onRepoFilesChanged(event.payload);
}).then((unlisten) => {
  if (watchClosed) {
    unlisten();
    return;
  }
  stopWatch = unlisten;
});
</script>

<template>
  <div class="repo-pane">
  <div
    v-if="current"
    class="repo-view"
    :class="{ resizing }"
    :style="{ '--files-pane-width': `${filesPaneWidth}px` }"
  >
    <section v-show="!showingDiff" class="graph-pane">
      <RepoToolbar
        :repo-id="current.repo.id"
        :name="current.status?.name ?? current.repo.path"
        :branch="current.status?.branch ?? ''"
        :path="current.repo.path"
        :branches="branches"
        :branch-tracking="branchTracking"
        :busy="actionBusy"
        :busy-label="actionLabel || (loading ? 'Loading…' : '')"
        :busy-branch="actionBranch"
        :checked-out-branch="checkedOutBranch"
        @fetch="fetchRepo"
        @pull="pullRepo"
        @pull-options="openPullOptions"
        @push="pushRepo"
        @undo-unpushed="undoUnpushedCommits"
        @checkout="checkoutBranch"
        @create="openCreateBranch"
        @merge="openMergeBranch()"
        @pull-branch="pullLocalBranch"
        @merge-into-current="openMergeIntoCurrent"
        @refresh-branches="refreshBranches"
      />
      <RepoViewTabs
        :active="mainTab"
        :busy="actionBusy"
        :branch-count="branches.length"
        :remote-count="remotes.length"
        :tag-count="tags.length"
        :stash-count="stashes.length"
        @select="selectMainTab"
      />
      <div v-if="conflictActive" class="conflict-banner">
        <div class="conflict-banner-copy">
          <strong>{{ conflictHeading }}</strong>
          <p class="tiny">{{ conflictCopy }}</p>
        </div>
        <div class="conflict-banner-actions">
          <button
            class="ghost tiny danger"
            type="button"
            :disabled="actionBusy || !operation"
            @click="abortCurrentOperation"
          >
            {{ abortLabel(operation) }}
          </button>
          <button
            class="ghost tiny commit"
            type="button"
            :disabled="actionBusy || !operation || conflictedCount > 0"
            @click="continueCurrentOperation"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4.5 12.75l6 6 9-13.5" />
            </svg>
            {{ continueLabel(operation) }}
          </button>
        </div>
      </div>
      <p v-if="message" class="banner">{{ message }}</p>
      <div class="graph-body">
        <TerminalPane
          v-if="terminalStarted"
          v-show="mainTab === 'terminal'"
          :cwd="current.repo.path"
          :active="mainTab === 'terminal'"
        />
        <BranchList
          v-if="branchesView"
          :overview="overview"
          :busy="actionBusy"
          @checkout="checkoutListedBranch"
          @merge="openMergeBranch"
          @rename="openRenameBranch"
          @delete="deleteBranch"
          @delete-merged="deleteMerged"
          @delete-selected="deleteSelectedBranches"
        />
        <RemoteList
          v-else-if="remotesView"
          :remotes="remotes"
          :selected="selectedRemote"
          :overview="remoteOverview"
          :busy="actionBusy"
          :loading="remoteLoading"
          :fetching="remoteFetching"
          @select="selectRemote"
          @fetch="fetchSelectedRemote()"
          @add="openAddRemote"
          @edit="openEditRemote"
          @remove="removeRemote"
          @open="openRemoteUrl"
          @checkout="checkoutRemote"
          @merge="openSyncBranch"
          @delete="deleteRemoteBranch"
        />
        <TagList
          v-else-if="tagView"
          :tags="tags"
          :busy="actionBusy"
          @create="openCreateTag"
          @delete="deleteTag"
        />
        <StashList
          v-else-if="stashView"
          :stashes="stashes"
          :selected-hash="selectedStash?.hash ?? ''"
          :busy="actionBusy"
          :can-stash="files.length > 0"
          @select="selectStash"
          @apply="applyStash"
          @pop="popStash"
          @drop="dropStash"
          @push="openStash"
        />
        <div v-else-if="mainTab === 'commits'" class="graph-scroll">
          <CommitGraph
            :commits="commits"
            :selected-hash="selectedCommit?.hash ?? ''"
            @select="selectCommit"
            @menu="openCommitMenu"
          />
        </div>
      </div>
    </section>
    <section v-if="showingDiff" class="diff-main">
      <div class="pane-header">
        <div class="diff-heading">
          <button class="ghost tiny" type="button" @click="closeDiff">← Back</button>
          <PathLabel
            class="diff-path"
            :path="selectedFile?.path ?? selectedCommitFile?.path ?? selectedHistoryCommit?.path ?? selectedHistoryFile"
          />
          <span class="muted tiny">{{
            selectedFile
              ? isConflicted(selectedFile)
                ? "Conflicted"
                : selectedFile.staged
                  ? "Staged"
                  : "Unstaged"
              : selectedCommitFile
                ? `${selectedCommitFile.status} · ${commitDetail?.label ?? ""}`
                : selectedHistoryCommit
                  ? selectedHistoryCommit.status
                    ? `${selectedHistoryCommit.status} · ${selectedHistoryCommit.hash.slice(0, 7)}`
                    : `${selectedHistoryCommit.hash.slice(0, 7)}`
                  : ""
          }}</span>
          <template v-if="selectedFile && isConflicted(selectedFile)">
            <button class="ghost tiny" type="button" @click="openInEditor(selectedFile)">
              {{ openEditorLabel }}
            </button>
            <button class="ghost tiny stage" type="button" @click="stageFiles([selectedFile])">
              Mark resolved
            </button>
          </template>
          <button
            v-if="operation"
            class="ghost tiny danger"
            type="button"
            :disabled="actionBusy"
            @click="abortCurrentOperation"
          >
            {{ abortLabel(operation) }}
          </button>
        </div>
        <div class="pane-header-end">
          <div class="segmented" role="group" aria-label="Diff layout">
            <button
              type="button"
              :class="{ active: diffMode === 'inline' }"
              :aria-pressed="diffMode === 'inline'"
              @click="saveDiffMode('inline')"
            >
              Inline
            </button>
            <button
              type="button"
              :class="{ active: diffMode === 'split' }"
              :aria-pressed="diffMode === 'split'"
              @click="saveDiffMode('split')"
            >
              Side by side
            </button>
          </div>
        </div>
      </div>
      <div class="diff-scroll">
        <DiffViewer
          :raw="diff"
          :mode="diffMode"
          :repo-path="current.repo.path"
          :file="blameFile"
          :rev="blameRev"
          :staged="blameStaged"
          :old-path="blameOldPath"
        />
      </div>
    </section>
    <aside class="changes-pane">
      <button
        class="pane-resize"
        type="button"
        aria-label="Resize files panel"
        @pointerdown="startResize"
      />
      <ChangesPanelTabs
        :history-open="historyOpen"
        :unstaged="unstagedCount"
        :staged="stagedCount"
        :conflicted="conflictedCount"
        @changes="openChangesPane"
        @history="openHistoryPane"
      />
      <div v-if="historyOpen" class="file-history-host">
        <FileTree
          class="file-history-tree"
          :class="{ parked: Boolean(selectedHistoryFile) }"
          :files="repoFiles"
          :loading="repoFilesLoading"
          :error="repoFilesError"
          :parked="Boolean(selectedHistoryFile)"
          :selected-path="lastHistoryFile"
          @select="selectHistoryFile"
        />
        <FileHistoryList
          v-if="selectedHistoryFile"
          :file="selectedHistoryFile"
          :commits="historyCommits"
          :selected-hash="selectedHistoryCommit?.hash ?? ''"
          :loading="historyCommitsLoading"
          :error="historyCommitsError"
          @select="selectHistoryCommit"
          @close="closeHistoryFile"
        />
      </div>
      <CommitFiles
        v-else-if="commitDetail"
        :title="commitDetail.title"
        :meta="commitDetail.meta"
        :files="commitFiles"
        :selected-path="selectedCommitFile?.path ?? ''"
        :loading="commitFilesLoading"
        @select="selectCommitFile"
        @close="closeCommitDetail"
      />
      <WorkingTree
        v-else
        :files="files"
        :selected-path="selectedFile?.path ?? ''"
        :selected-staged="selectedFile?.staged ?? false"
        :operation="operation"
        :has-draft="hasCommitDraft"
        @select="selectFile"
        @stage="stageFiles"
        @unstage="unstageFiles"
        @stage-all="stageAll"
        @unstage-all="unstageAll"
        @discard="discardAll"
        @stash="openStash"
        @stash-files="stashFiles"
        @ignore="ignoreFiles"
        @reveal="revealFile"
        @copy-path="copyFilePath"
        @discard-files="discardFiles"
        @delete-files="deleteFiles"
        @commit="openCommit"
        @open-editor="openInEditor"
      />
    </aside>
  </div>
  <div v-else class="empty-home">
    <p class="muted">{{ loaded ? "Repository not found." : "Loading…" }}</p>
  </div>
  <Modal v-if="pullingOptions" title="Pull from remote" @close="closePullOptions">
    <p class="pull-summary">
      Merges <code>origin/{{ pullRemoteLabel }}</code> into the currently checked-out branch.
      Checkout does not change.
    </p>
    <fieldset class="radio-list">
      <legend class="muted tiny">Remote branch</legend>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="current" />
        origin/current-branch
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="develop" />
        origin/develop
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="master" />
        origin/master
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="main" />
        origin/main
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="specify" />
        Specify
      </label>
      <input
        v-if="pullSource === 'specify'"
        v-model="specifyBranch"
        type="text"
        placeholder="branch name"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        autofocus
        @keydown.enter="confirmPull"
      />
    </fieldset>
    <p class="muted tiny pull-hint">{{ pullHint }}</p>
    <template #actions>
      <button class="ghost" type="button" @click="closePullOptions">Cancel</button>
      <button class="primary" type="button" :disabled="!canConfirmPull" @click="confirmPull">
        Pull
      </button>
    </template>
  </Modal>
  <Modal v-if="committing" :title="amending ? 'Amend last commit' : 'Commit'" medium @close="closeCommit">
    <label class="modal-label">
      <span class="modal-label-row">
        <span class="muted tiny">Title</span>
        <span
          class="muted tiny char-count"
          :class="{ warn: commitTitleLeft <= 12, bad: commitTitleLeft === 0 }"
        >
          {{ commitTitleLength }}/{{ COMMIT_TITLE_MAX }} · {{ commitTitleLeft }} left
        </span>
      </span>
      <input
        ref="commitTitleInput"
        v-model="commitTitle"
        type="text"
        :maxlength="COMMIT_TITLE_MAX"
        placeholder="Short summary of the change"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter.prevent="commitChanges({ all: !stagedCount && commitAllAvailable })"
      />
    </label>
    <label class="modal-label">
      <span class="muted tiny">Description</span>
      <textarea
        v-model="commitDescription"
        rows="5"
        placeholder="Optional details"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
      />
    </label>
    <label v-if="lastCommit" class="radio-option">
      <input type="checkbox" :checked="amending" @change="onAmendChange" />
      Amend last commit
    </label>
    <p v-if="amending && lastCommit?.published" class="muted tiny">
      This commit is already on the remote. Amending rewrites it, and you will need to force-push.
    </p>
    <p v-if="!stagedCount && commitAllAvailable" class="muted tiny">
      Nothing is staged. {{ amending ? "Amend all" : "Commit all" }} stages every changed file,
      including untracked files.
    </p>
    <p v-else-if="!stagedCount" class="muted tiny">Stage a file to commit.</p>
    <template #actions>
      <button class="ghost" type="button" @click="closeCommit">Close</button>
      <button
        v-if="commitAllAvailable"
        class="ghost commit"
        type="button"
        :disabled="!canCommitAll"
        title="Stage every changed file, including untracked files, and commit."
        @click="commitChanges({ all: true })"
      >
        {{ amending ? "Amend all" : "Commit all" }}
      </button>
      <button
        v-if="stagedCount || !commitAllAvailable"
        class="ghost commit"
        type="button"
        :disabled="!canCommit"
        @click="commitChanges()"
      >
        {{ amending ? "Amend" : "Commit" }}
      </button>
    </template>
  </Modal>
  <Modal
    v-if="stashing"
    :title="
      stashFileTargets.length > 1
        ? `Stash ${stashFileTargets.length} files`
        : stashFileTargets.length
          ? 'Stash file'
          : 'Stash changes'
    "
    @close="closeStash"
  >
    <label class="modal-label">
      <span class="muted tiny">Message</span>
      <input
        ref="stashMessageInput"
        v-model="stashMessage"
        type="text"
        placeholder="Optional summary of this work"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter.prevent="stashChanges"
      />
    </label>
    <p v-if="stashFileTargets.length > 1" class="muted tiny">
      Saves the changes to {{ stashFileTargets.length }} files in one stash, then removes them from the working tree. Other files stay as they are.
    </p>
    <p v-else-if="stashFileTargets.length" class="muted tiny">
      Saves the changes to {{ stashFileTargets[0].path }}, then removes them from the working tree. Other files stay as they are.
    </p>
    <p v-else class="muted tiny">Saves staged, unstaged, and untracked files, then clears the working tree.</p>
    <template #actions>
      <button class="ghost" type="button" @click="closeStash">Cancel</button>
      <button class="primary" type="button" :disabled="!files.length" @click="stashChanges">
        Stash
      </button>
    </template>
  </Modal>
  <Modal v-if="creatingTag" title="New tag" @close="closeCreateTag">
    <label class="modal-label">
      <span class="muted tiny">Tag name</span>
      <input
        ref="newTagInput"
        v-model="newTagName"
        type="text"
        placeholder="v1.0.0"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter.prevent="createTag"
      />
    </label>
    <label class="modal-label">
      <span class="muted tiny">Message</span>
      <input
        v-model="newTagMessage"
        type="text"
        placeholder="Optional. Makes an annotated tag"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
      />
    </label>
    <label class="modal-label">
      <span class="muted tiny">Commit</span>
      <input
        v-model="newTagTarget"
        type="text"
        placeholder="Current commit (HEAD)"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter.prevent="createTag"
      />
    </label>
    <p class="muted tiny">Leave commit blank to tag HEAD. A message makes an annotated tag.</p>
    <template #actions>
      <button class="ghost" type="button" @click="closeCreateTag">Cancel</button>
      <button class="primary" type="button" :disabled="!canCreateTag" @click="createTag">
        Create tag
      </button>
    </template>
  </Modal>
  <Modal v-if="creatingBranch" title="New branch" @close="closeCreateBranch">
    <label class="modal-label">
      <span class="muted tiny">Branch name</span>
      <input
        ref="newBranchInput"
        v-model="newBranchName"
        type="text"
        placeholder="feature/JIRA-123"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter="createBranch"
      />
    </label>
    <div v-if="!newBranchStart" class="modal-label">
      <span class="muted tiny">Base branch</span>
      <BranchSelect
        v-model="baseBranch"
        label="Base branch"
        :branches="baseBranchOptions"
        :branch-tracking="branchTracking"
        :current="checkedOutBranch"
      />
    </div>
    <p v-if="newBranchStartShort" class="muted tiny">Starts at {{ newBranchStartShort }}.</p>
    <p v-else class="muted tiny">The new branch starts at the tip of the base branch, then checks it out.</p>
    <template #actions>
      <button class="ghost" type="button" @click="closeCreateBranch">Cancel</button>
      <button class="primary" type="button" :disabled="!canCreateBranch" @click="createBranch">
        Create and switch
      </button>
    </template>
  </Modal>
  <Modal v-if="renamingBranch" title="Rename branch" @close="closeRenameBranch">
    <label class="modal-label">
      <span class="muted tiny">Branch name</span>
      <input
        ref="renameBranchInput"
        v-model="renameBranchName"
        type="text"
        placeholder="feature/JIRA-123"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter="renameBranch"
      />
    </label>
    <template #actions>
      <button class="ghost" type="button" @click="closeRenameBranch">Cancel</button>
      <button class="primary" type="button" :disabled="!canRenameBranch" @click="renameBranch">
        Rename
      </button>
    </template>
  </Modal>
  <Modal
    v-if="remoteForm"
    :title="remoteForm.mode === 'add' ? 'Add remote' : `Edit ${remoteForm.original}`"
    @close="closeRemoteForm"
  >
    <label class="modal-label">
      <span class="muted tiny">Name</span>
      <input
        ref="remoteFormNameInput"
        v-model="remoteFormName"
        type="text"
        placeholder="upstream"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter.prevent="submitRemoteForm"
      />
    </label>
    <label class="modal-label">
      <span class="muted tiny">URL</span>
      <input
        v-model="remoteFormUrl"
        type="text"
        placeholder="git@github.com:owner/repo.git"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter.prevent="submitRemoteForm"
      />
    </label>
    <p v-if="remoteForm.mode === 'add'" class="muted tiny">
      For a fork, name the original project <strong>upstream</strong>. Shipyard fetches its
      branches after adding it.
    </p>
    <p v-else class="muted tiny">
      Renaming keeps every local branch that tracks this remote pointed at it.
    </p>
    <template #actions>
      <button class="ghost" type="button" @click="closeRemoteForm">Cancel</button>
      <button
        class="primary"
        type="button"
        :disabled="!canSubmitRemoteForm"
        @click="submitRemoteForm"
      >
        {{ remoteForm.mode === "add" ? "Add remote" : "Save" }}
      </button>
    </template>
  </Modal>
  <Modal v-if="namingCheckout" title="Check out remote branch" @close="closeCheckoutNaming">
    <p class="pull-summary">
      You already have a local <code>{{ namingCheckout.local }}</code> that doesn't track
      <code>{{ namingCheckout.remote }}/{{ namingCheckout.name }}</code>. Pick a name for a new local
      branch that does.
    </p>
    <label class="modal-label">
      <span class="muted tiny">Local branch name</span>
      <input
        ref="checkoutLocalNameInput"
        v-model="checkoutLocalName"
        type="text"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        @keydown.enter.prevent="confirmCheckoutNaming"
      />
    </label>
    <p v-if="branches.includes(checkoutLocalName.trim())" class="muted tiny">
      A local branch with that name already exists.
    </p>
    <template #actions>
      <button class="ghost" type="button" @click="closeCheckoutNaming">Cancel</button>
      <button
        class="primary"
        type="button"
        :disabled="!canConfirmCheckoutNaming"
        @click="confirmCheckoutNaming"
      >
        Create and switch
      </button>
    </template>
  </Modal>
  <Modal v-if="syncingBranch" :title="`Merge ${syncSourceLabel}`" @close="closeSync">
    <p class="pull-summary">
      Brings <code>{{ syncSourceLabel }}</code> into your local
      <code>{{ syncTarget || "…" }}</code>.
    </p>
    <div class="modal-label">
      <span class="muted tiny">Into</span>
      <BranchSelect
        v-model="syncTarget"
        label="Into"
        :branches="branches"
        :branch-tracking="branchTracking"
        :current="checkedOutBranch"
      />
    </div>
    <fieldset class="radio-list">
      <legend class="muted tiny">How</legend>
      <label class="radio-option">
        <input v-model="syncAllowMerge" type="radio" :value="false" />
        Fast-forward only (recommended)
      </label>
      <label class="radio-option">
        <input v-model="syncAllowMerge" type="radio" :value="true" />
        Allow a merge commit
      </label>
    </fieldset>
    <label v-if="showSyncPush" class="radio-option">
      <input v-model="syncPush" type="checkbox" />
      {{ syncPushLabel }}
    </label>
    <p class="muted tiny pull-hint">
      <template v-if="!syncAllowMerge">
        Fast-forward moves {{ syncTarget || "the branch" }} up to {{ syncSourceLabel }} without a
        merge commit. If {{ syncTarget || "it" }} has its own commits, Shipyard asks before merging.
      </template>
      <template v-else>
        Checks out {{ syncTarget || "the branch" }} if needed. Conflicts appear in the files list
        so you can resolve them or abort, and nothing is pushed until they're done.
      </template>
    </p>
    <template #actions>
      <button class="ghost" type="button" @click="closeSync">Cancel</button>
      <button class="primary" type="button" :disabled="!syncTarget" @click="confirmSync">
        {{ syncAllowMerge ? "Merge" : "Sync" }}
      </button>
    </template>
  </Modal>
  <Modal v-if="mergingBranch" title="Merge local branch" @close="closeMergeBranch">
    <p class="pull-summary">
      Merges <code>{{ mergeSource || "…" }}</code> into <code>{{ mergeTarget || "…" }}</code>.
      Checkout switches to the target first if needed.
    </p>
    <div class="modal-label">
      <span class="muted tiny">From</span>
      <BranchSelect
        v-model="mergeSource"
        label="From"
        :branches="localBranchNames"
        :branch-tracking="branchTracking"
        :current="checkedOutBranch"
      />
    </div>
    <div class="modal-label">
      <span class="muted tiny">Into</span>
      <BranchSelect
        v-model="mergeTarget"
        label="Into"
        :branches="mergeTargetOptions"
        :branch-tracking="branchTracking"
        :current="checkedOutBranch"
      />
    </div>
    <p class="muted tiny pull-hint">
      Conflicts appear in the files list so you can open them, mark them resolved, or abort.
    </p>
    <template #actions>
      <button class="ghost" type="button" @click="closeMergeBranch">Cancel</button>
      <button class="primary" type="button" :disabled="!canConfirmMerge" @click="mergeLocalBranch">
        Merge
      </button>
    </template>
  </Modal>
  <CommitContextMenu
    v-if="commitMenu"
    :commit="commitMenu.commit"
    :hashes="commitMenu.hashes"
    :x="commitMenu.x"
    :y="commitMenu.y"
    :busy="actionBusy"
    :operation="operation"
    :has-merge="commitMenuHasMerge"
    @checkout="checkoutMenuCommit"
    @create-branch="createBranchFromMenu"
    @cherry-pick="cherryPickMenuCommits"
    @revert="revertMenuCommits"
    @copy-sha="copyMenuShas"
    @copy-link="copyMenuLink"
    @create-tag="createTagFromMenu"
    @close="closeCommitMenu"
  />
  </div>
</template>
