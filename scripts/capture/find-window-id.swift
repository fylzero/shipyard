#!/usr/bin/swift

import Cocoa

// Find the window ID of a running application by bundle identifier
// Usage: ./find-window-id.swift <bundle-identifier>
// Returns: window ID on stdout, or exits with error

guard CommandLine.arguments.count > 1 else {
    fputs("Usage: find-window-id.swift <bundle-identifier>\n", stderr)
    exit(1)
}

let bundleIdentifier = CommandLine.arguments[1]

// Get all running applications
let workspace = NSWorkspace.shared
let runningApps = workspace.runningApplications

// Find the target app
guard let targetApp = runningApps.first(where: { $0.bundleIdentifier == bundleIdentifier }) else {
    fputs("Application with bundle identifier '\(bundleIdentifier)' is not running\n", stderr)
    exit(1)
}

guard let pid = targetApp.processIdentifier as pid_t? else {
    fputs("Could not get process ID for application\n", stderr)
    exit(1)
}

// Use CGWindowList to find windows for this process
let options = CGWindowListOption(arrayLiteral: .optionOnScreenOnly, .excludeDesktopElements)
guard let windowList = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
    fputs("Could not get window list\n", stderr)
    exit(1)
}

// Filter windows by owner PID and find a visible window with bounds
for window in windowList {
    guard let windowPID = window[kCGWindowOwnerPID as String] as? Int32,
          windowPID == pid,
          let windowNumber = window[kCGWindowNumber as String] as? Int,
          let bounds = window[kCGWindowBounds as String] as? [String: CGFloat],
          let width = bounds["Width"],
          let height = bounds["Height"],
          width > 100 && height > 100 else {
        continue
    }
    
    // Found a suitable window
    print(windowNumber)
    exit(0)
}

fputs("No visible window found for application\n", stderr)
exit(1)
