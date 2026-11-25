import { computed, ref, watch } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

/*
 * This file is shared by Dispatch, Lockshot, Recon, and Shipyard. Keep it the
 * same in all four apart from APP_NAME, so a fix in one can be copied over.
 */
const APP_NAME = "Shipyard";
/** How long the automatic check waits between looks at GitHub. */
const AUTO_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;
/** How often the timer wakes to see if a check is due, so one still runs soon after the Mac wakes from sleep. */
const AUTO_CHECK_TICK_MS = 10 * 60 * 1000;
/** After Later, an automatic check waits this long before asking about the same version again. */
const REMIND_AFTER_MS = 24 * 60 * 60 * 1000;
const STORAGE_KEY = "updater";

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
/** An automatic check found an update while the window was in the background: ask when it comes forward. */
let promptOnFocus = false;
let lastAutoCheck = 0;

/** Survives restarts, so a version is only announced once and Later is remembered. */
interface StoredUpdaterState {
  notifiedVersion?: string;
  dismissedVersion?: string;
  dismissedAt?: number;
}

function readStored(): StoredUpdaterState {
  try {
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function writeStored(patch: StoredUpdaterState) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...readStored(), ...patch }));
  } catch {
    /* Only costs a repeat reminder. */
  }
}

function recentlyDismissed(version: string) {
  const stored = readStored();
  return stored.dismissedVersion === version && Date.now() - (stored.dismissedAt ?? 0) < REMIND_AFTER_MS;
}

/** Visible, not minimized, and focused: somewhere the update prompt will actually be seen. */
async function windowInFront() {
  try {
    const appWindow = getCurrentWindow();
    return (await appWindow.isVisible()) && !(await appWindow.isMinimized()) && (await appWindow.isFocused());
  } catch {
    return document.visibilityState === "visible" && document.hasFocus();
  }
}

async function notifyUpdate(version: string) {
  if (readStored().notifiedVersion === version) {
    return;
  }
  writeStored({ notifiedVersion: version });
  try {
    const granted = (await isPermissionGranted()) || (await requestPermission()) === "granted";
    if (granted) {
      sendNotification({
        title: `${APP_NAME} ${version} is available`,
        body: `Open ${APP_NAME} to install it.`,
      });
    }
  } catch {
    /* The update icon in the tab bar still shows it. */
  }
}

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
        return `Installing… ${APP_NAME} will restart.`;
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
      currentVersion.value = "";
    }
  }

  /**
   * `silent` is an automatic check: errors stay quiet, and the prompt only
   * opens when the window is in front (otherwise a system notification says
   * so, and the prompt waits for the window to come forward). Without it, the
   * person asked, so the prompt opens even after Later.
   */
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
      if (options?.prompt === false) {
        return update;
      }
      if (!options?.silent) {
        promptOpen.value = true;
      } else if (await windowInFront()) {
        if (!recentlyDismissed(update.version)) {
          promptOpen.value = true;
        }
      } else {
        promptOnFocus = true;
        void notifyUpdate(update.version);
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
    promptOnFocus = false;
    if (availableVersion.value) {
      writeStored({ dismissedVersion: availableVersion.value, dismissedAt: Date.now() });
    }
  }

  /**
   * Checks at launch and then every few hours while `enabled` is on, and when
   * the window comes forward. Returns a function that stops it.
   */
  function startAutoChecks(enabled: () => boolean) {
    if (import.meta.env.DEV) {
      return () => undefined;
    }
    const tick = () => {
      /* An update already waiting is shown by the tab bar icon; checking again would only reset it. */
      if (!enabled() || busy.value || updateReady.value || Date.now() - lastAutoCheck < AUTO_CHECK_INTERVAL_MS) {
        return;
      }
      lastAutoCheck = Date.now();
      void checkForUpdates({ silent: true });
    };
    const onFocus = () => {
      if (promptOnFocus && status.value === "available" && availableVersion.value) {
        promptOnFocus = false;
        if (!recentlyDismissed(availableVersion.value)) {
          promptOpen.value = true;
        }
      }
      tick();
    };
    const timer = window.setInterval(tick, AUTO_CHECK_TICK_MS);
    const stopWatching = watch(enabled, (on) => on && tick());
    let stopFocus: (() => void) | undefined;
    let stopped = false;
    void getCurrentWindow()
      .onFocusChanged(({ payload }) => payload && onFocus())
      .then((unlisten) => (stopped ? unlisten() : (stopFocus = unlisten)))
      .catch(() => undefined);
    tick();
    return () => {
      stopped = true;
      window.clearInterval(timer);
      stopWatching();
      stopFocus?.();
    };
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
    startAutoChecks,
  };
}

function formatUpdaterError(err: unknown) {
  const text = String(err).replace(/^Error:\s*/i, "").trim();
  /* No release (or no latest.json in it) yet: GitHub answered, there is just nothing to read. */
  if (/404|not found|latest\.json|valid release json/i.test(text)) {
    return "No update feed on GitHub yet. This starts working after the next release.";
  }
  if (/fetch|network|request|dns|timed out|connection/i.test(text)) {
    return "Could not reach GitHub to check for updates.";
  }
  return text || "Could not check for updates.";
}
