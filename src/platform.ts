export const isMac =
  typeof navigator === "undefined" || /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

export const commandKeyName = isMac ? "Command" : "Ctrl";

export const fileManagerName = isMac ? "Finder" : "File Manager";

export const systemNotificationsTarget = isMac
  ? "to macOS Notification Center"
  : "as desktop notifications";

export const defaultAppSource = isMac
  ? "the macOS file association"
  : "your desktop's default app for the file type";
