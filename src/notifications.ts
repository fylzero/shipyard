import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type { NotificationMode } from "./types";

export const NOTIFICATION_MODES: { id: NotificationMode; label: string }[] = [
  { id: "background", label: "In background" },
  { id: "always", label: "Always" },
  { id: "off", label: "Never" },
];

export const windowFocused = ref(typeof document === "undefined" ? true : document.hasFocus());

let watchingFocus = false;
let permissionRequest: Promise<boolean> | null = null;

export function watchWindowFocus() {
  if (watchingFocus) {
    return;
  }
  watchingFocus = true;
  const appWindow = getCurrentWindow();
  void appWindow
    .isFocused()
    .then((focused) => {
      windowFocused.value = focused;
    })
    .catch(() => {
      /* keep the document focus guess */
    });
  void appWindow
    .onFocusChanged(({ payload }) => {
      windowFocused.value = payload;
    })
    .catch(() => {
      watchingFocus = false;
    });
}

export function sanitizeNotificationMode(value: unknown): NotificationMode {
  return value === "always" || value === "off" ? value : "background";
}

export function prefersSystemNotification(mode: NotificationMode) {
  return mode === "always" || (mode === "background" && !windowFocused.value);
}

/**
 * Only a granted answer is cached, so turning notifications on in System
 * Settings takes effect without restarting Shipyard.
 */
export function ensureNotificationPermission(): Promise<boolean> {
  permissionRequest ??= (async () => {
    if (await isPermissionGranted()) {
      return true;
    }
    return (await requestPermission()) === "granted";
  })()
    .catch(() => false)
    .then((granted) => {
      if (!granted) {
        permissionRequest = null;
      }
      return granted;
    });
  return permissionRequest;
}

export async function sendSystemNotification(body: string, kind: "success" | "error") {
  if (!(await ensureNotificationPermission())) {
    return false;
  }
  try {
    sendNotification({ title: kind === "error" ? "Shipyard: action failed" : "Shipyard", body });
    return true;
  } catch {
    return false;
  }
}
