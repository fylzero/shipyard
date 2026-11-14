import { computed, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

export type UpdaterStatus =
  | "idle"
  | "checking"
  | "available"
  | "up-to-date"
  | "downloading"
  | "installing"
  | "error";

const status = ref<UpdaterStatus>("idle");
const currentVersion = ref("");
const availableVersion = ref("");
const errorMessage = ref("");
const downloaded = ref(0);
const contentLength = ref(0);
const promptOpen = ref(false);

let pendingUpdate: Update | null = null;
let dismissedVersion = "";

export function useUpdater() {
  const busy = computed(
    () =>
      status.value === "checking" ||
      status.value === "downloading" ||
      status.value === "installing",
  );

  const updateReady = computed(
    () =>
      status.value === "available" ||
      status.value === "downloading" ||
      status.value === "installing",
  );

  const progressPercent = computed(() => {
    if (!contentLength.value) {
      return 0;
    }
    return Math.min(100, Math.round((downloaded.value / contentLength.value) * 100));
  });

  const statusText = computed(() => {
    switch (status.value) {
      case "checking":
        return "Checking GitHub for a newer build…";
      case "up-to-date":
        return "You're on the latest version.";
      case "available":
        return `Version ${availableVersion.value} is available.`;
      case "downloading":
        return contentLength.value
          ? `Downloading… ${progressPercent.value}%`
          : "Downloading…";
      case "installing":
        return "Installing… Shipyard will restart.";
      case "error":
        return errorMessage.value;
      default:
        return "";
    }
  });

  async function ensureCurrentVersion() {
    if (currentVersion.value) {
      return;
    }
    try {
      currentVersion.value = await getVersion();
    } catch {
      currentVersion.value = "1.4.0";
    }
  }

  async function checkForUpdates(options?: { prompt?: boolean; silent?: boolean }) {
    if (busy.value) {
      return null;
    }
    await ensureCurrentVersion();
    status.value = "checking";
    errorMessage.value = "";
    if (pendingUpdate) {
      await pendingUpdate.close().catch(() => undefined);
      pendingUpdate = null;
    }
    try {
      const update = await check();
      if (!update) {
        availableVersion.value = "";
        promptOpen.value = false;
        status.value = "up-to-date";
        return null;
      }
      pendingUpdate = update;
      currentVersion.value = update.currentVersion || currentVersion.value;
      availableVersion.value = update.version;
      status.value = "available";
      if (options?.prompt !== false && dismissedVersion !== update.version) {
        promptOpen.value = true;
      }
      return update;
    } catch (err) {
      pendingUpdate = null;
      availableVersion.value = "";
      promptOpen.value = false;
      if (options?.silent) {
        status.value = "idle";
        errorMessage.value = "";
        return null;
      }
      status.value = "error";
      errorMessage.value = formatUpdaterError(err);
      return null;
    }
  }

  async function installUpdate() {
    if (!pendingUpdate || (busy.value && status.value !== "available")) {
      return;
    }
    promptOpen.value = true;
    status.value = "downloading";
    downloaded.value = 0;
    contentLength.value = 0;
    try {
      await pendingUpdate.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") {
          contentLength.value = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded.value += event.data.chunkLength;
        } else if (event.event === "Finished") {
          status.value = "installing";
        }
      });
      await relaunch();
    } catch (err) {
      status.value = "error";
      errorMessage.value = formatUpdaterError(err);
    }
  }

  function dismissPrompt() {
    promptOpen.value = false;
    if (availableVersion.value) {
      dismissedVersion = availableVersion.value;
    }
  }

  function showPrompt() {
    if (updateReady.value) {
      promptOpen.value = true;
    }
  }

  return {
    status,
    statusText,
    currentVersion,
    availableVersion,
    errorMessage,
    progressPercent,
    promptOpen,
    busy,
    updateReady,
    ensureCurrentVersion,
    checkForUpdates,
    installUpdate,
    dismissPrompt,
    showPrompt,
  };
}

function formatUpdaterError(err: unknown) {
  const text = String(err).replace(/^Error:\s*/i, "").trim();
  if (/404|not found|latest\.json/i.test(text)) {
    return "No update feed on GitHub yet. This starts working after the next release.";
  }
  if (/fetch|network|request|dns|timed out|connection/i.test(text)) {
    return "Could not reach GitHub to check for updates.";
  }
  return text || "Could not check for updates.";
}
