export const isMac =
  typeof navigator === "undefined" || /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

export const isWindows = typeof navigator !== "undefined" && /Win/.test(navigator.platform || navigator.userAgent);

export const commandKeyName = isMac ? "Command" : "Ctrl";

export const fileManagerName = isMac ? "Finder" : isWindows ? "File Explorer" : "File Manager";

/** The last part of a folder or file path, which on Windows may use backslashes. */
export function baseName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

export const systemNotificationsTarget = isMac
  ? "to macOS Notification Center"
  : "as desktop notifications";

export const defaultAppSource = isMac
  ? "the macOS file association"
  : "your desktop's default app for the file type";
