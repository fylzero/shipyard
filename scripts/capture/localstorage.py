#!/usr/bin/env python3
"""Remove demo-only keys from Shipyard's webview localStorage.

Shipyard's webview keeps per-repository UI state (for example
"shipyard:branches-view:<repo path>") in the real user's WebKit storage under
~/Library/WebKit/com.shipyard.app. Setting HOME for the capture does not
redirect it, so opening the demo repositories would leave keys for them behind
in the user's real app storage. capture.sh calls this before and after a run to
delete only the keys that mention the demo directory and leaves everything else
alone.

Usage:
  localstorage.py purge-demo
"""
import glob, os, sqlite3, sys

PATTERN = os.path.expanduser(
    "~/Library/WebKit/com.shipyard.app/WebsiteData/Default/*/*/LocalStorage/localstorage.sqlite3")
DEMO_MARKER = "shipyard-demo-capture"

def main():
    if len(sys.argv) != 2 or sys.argv[1] != "purge-demo":
        sys.exit(__doc__)
    for path in glob.glob(PATTERN):
        con = sqlite3.connect(path)
        con.execute("DELETE FROM ItemTable WHERE key LIKE ?", (f"%{DEMO_MARKER}%",))
        con.commit()
        con.close()

if __name__ == "__main__":
    main()
