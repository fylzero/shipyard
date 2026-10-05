export interface ChangelogRelease {
  version: string;
  date: string;
  notes: string[];
}

export const CHANGELOG: ChangelogRelease[] = [
  {
    version: "1.12.0",
    date: "October 5, 2026",
    notes: [
      "Right-click a branch in the branch dropdown and choose Create pull request… to open your host's new pull request page in the browser. It works with GitHub, GitLab, and Bitbucket, and doesn't need a sign-in or token.",
      "Create pull request… shows for branches that exist on origin. Branches marked Local need to be pushed first.",
      "Add repository is now the first button on the dashboard. New group moved after Clone repository.",
      "Branches and Remotes are now one Branches tab. Switch between Local and Remote at the top. Each repo remembers which one you used last.",
      "The branch filter is shared between Local and Remote, so you can type a name once and flip between them. Command-F still jumps to it.",
    ],
  },
  {
    version: "1.11.0",
    date: "October 3, 2026",
    notes: [
      "Right-click a branch in the branch dropdown for Pull and Merge into current branch.",
      "Pull shows when the branch is behind its remote. On a branch you don't have checked out, it fetches and fast-forwards the branch without switching to it. If that branch also has local commits, Pull is turned off and explains why.",
      "Merge into current branch opens Merge local branch with the branch you right-clicked as From and the branch you're on as Into.",
      "Shift-F10 opens the menu for the highlighted branch. Escape closes it and leaves the branch dropdown open.",
      "The branch pickers in Merge local branch, Sync, and New branch are now searchable dropdowns like the one in the toolbar. They show the Current, Local, and ahead or behind badges, and you can start typing to filter.",
      "Toasts moved to the bottom-left corner, above the status bar, so they no longer cover the toolbar. Hovering over a toast keeps it open. Clicking the toast itself no longer does anything, so it can't catch a click meant for a button underneath. Use View details or the close button instead.",
      "Checking out a branch, fetching a single repo, and picking the branch you're already on no longer show a toast, since the branch name and status already update. Failures still show one.",
      "New System notifications setting in General. The default, In background, sends results to macOS Notification Center when Shipyard isn't the active app, and the toast waits until you switch back. You can also choose Always or Never.",
      "Repos inside a group now have Edit repository in their row menu, so you can give them a label. Their color still comes from the group.",
    ],
  },
  {
    version: "1.10.1",
    date: "September 30, 2026",
    notes: [
      "Check for Updates moved from Settings to the top-right of the Change Log. It shows Update to vX.Y.Z once a newer build is found.",
      "The release you're running is marked Current in the Change Log.",
    ],
  },
  {
    version: "1.10.0",
    date: "September 30, 2026",
    notes: [
      "You can select several files in the unstaged or staged list. Cmd-click adds or removes a file, Shift-click selects a range, and Escape clears the selection. Clicking a file on its own still opens its diff.",
      "Right-click a selection to stage or unstage, ignore, stash, discard, or delete all of those files at once. Stashing several files puts them in one stash instead of one per file.",
      "Discard and delete ask once for the whole selection and list the files. If one file can't be discarded or deleted, Shipyard stops there and says how many were already done.",
      "The unstaged and staged lists each keep their own selection, so a file with both staged and unstaged changes is only acted on in the list you picked it from.",
      "Click a stash to preview it without applying it. Its files show in the right panel, and clicking one opens its diff. Untracked files saved in the stash are listed too.",
    ],
  },
  {
    version: "1.9.1",
    date: "September 30, 2026",
    notes: [
      "Text fields no longer capitalize the first letter as you type. Commit titles and descriptions, tag names and messages, branch names, stash messages, remotes, clone paths, and search fields keep the case you type.",
      "Git name and email, group names, repo labels, and custom font names do the same.",
      "Stash file in the right-click menu now asks for a name, like stashing all changes does. The name starts as the file's path, so you can tell file stashes apart instead of seeing the same \"WIP on\" name for each one.",
    ],
  },
  {
    version: "1.9.0",
    date: "September 29, 2026",
    notes: [
      "Groups and ungrouped repos now share one list on the dashboard, so you can drag them into any order, like a repo between two groups.",
      "To take a repo out of a group, drop it in the gap between entries or on the top half of a group's header. Dropping it lower on a group puts it inside. The separate drop area at the top is gone.",
      "Sort A–Z on the dashboard sorts groups and ungrouped repos together by name.",
      "New groups appear at the top of the dashboard.",
    ],
  },
  {
    version: "1.8.3",
    date: "September 29, 2026",
    notes: [
      "Shipyard is now open source under the MIT license. You can use, change, share, and sell it with no restrictions beyond keeping the copyright notice.",
      "Building Shipyard from source now uses Bun instead of npm. The README has the updated setup steps.",
    ],
  },
  {
    version: "1.8.2",
    date: "September 28, 2026",
    notes: [
      "Shipyard is now licensed under the Functional Source License (FSL-1.1-MIT). You can use, change, and share it freely, including at work, but not sell it as a competing product. Each release becomes MIT two years after it comes out.",
    ],
  },
  {
    version: "1.8.1",
    date: "September 26, 2026",
    notes: [
      "Clone repository accepts SSH addresses like git@github.com:owner/repo.git as well as HTTPS URLs.",
      "Cloning over SSH no longer fails the first time you clone from a host. Shipyard trusts the host and remembers it, but still refuses if a known host's key has changed.",
      "If the host rejects your SSH key, the error explains how to fix it: load the key into your SSH agent with ssh-add and add it to your account on the host.",
    ],
  },
  {
    version: "1.8.0",
    date: "September 26, 2026",
    notes: [
      "Repos can be dragged into, out of, and between groups. Drop a repo on another group's list or its header to move it there, or drag it to the top of the dashboard to take it out of its group.",
      "Collapsed and empty groups accept drops too. When there are no ungrouped repos, a drop area appears at the top while you drag.",
      "Repos can't be moved between groups while a fetch, pull, or checkout is running on either one. Reordering inside a group still works.",
      "An empty group shows No repositories yet with an Add one link instead of just its header.",
    ],
  },
  {
    version: "1.7.0",
    date: "September 26, 2026",
    notes: [
      "The dashboard has a Clone repository button next to Add repository. Paste a URL, pick where to put it, and Shipyard clones the repo and adds it to your list.",
      "The folder name fills in from the URL, and the location starts next to the repo you added most recently, or your home folder. Both can be changed before cloning.",
      "If the clone fails, git's error is shown in the dialog so you can fix the URL or folder and try again.",
    ],
  },
  {
    version: "1.6.2",
    date: "September 25, 2026",
    notes: [
      "The diff viewer hides git's header lines like diff --git, index, and the @@ ranges. Chunks of a file are separated by a thin bar instead.",
      "Renames, file mode changes, and binary files are shown as a short note above the diff, since those details used to live in the hidden header lines.",
      "Deleted or added lines that start with -- or ++, like SQL comments, now show up correctly in the diff instead of being mistaken for header lines.",
    ],
  },
  {
    version: "1.6.1",
    date: "September 24, 2026",
    notes: [
      "The Tags page has a header bar like the Remotes page, with New tag on the right.",
      "File History shows its filter field right away instead of a File History title and a search button.",
    ],
  },
  {
    version: "1.6.0",
    date: "September 24, 2026",
    notes: [
      "Repo views are now tabs: Commits, Branches, Remotes, Tags, and Stashes, with Terminal at the far right. Commits opens by default.",
      "Fetch, Pull, and Push moved to the top right, next to the repo name, so the header takes up one row instead of two.",
      "The terminal fills the whole area like the other tabs and keeps running when you switch away. It no longer has a Close button.",
      "The right panel always stays open, with Changes and File History as tabs at the top. Changes is selected by default.",
      "The Terminal height setting is gone now that the terminal fills its tab.",
    ],
  },
  {
    version: "1.5.0",
    date: "September 24, 2026",
    notes: [
      "A new Remotes page lists every remote on a repo, like origin and upstream. Pick one to see its branches, and add, rename, change the URL of, or remove remotes from the same page.",
      "Keep a fork up to date: pick a branch on upstream, choose Merge into, and Shipyard fast-forwards your local branch, then pushes it to your fork. If your branch has its own commits, it asks before making a merge commit.",
      "Remote branches show which local branch tracks them and how many commits each side is ahead or behind. Checkout makes a local tracking branch, and Delete removes a branch from the server after you confirm. The remote's default branch can't be deleted.",
      "Fetch now fetches every remote, not just origin, so upstream branches stay current too.",
      "The dashboard updates as soon as git changes a repo outside Shipyard. Checking out a branch, committing, tagging, or fetching from the terminal shows up right away instead of at the next auto-fetch.",
      "Tags and branch switches made outside Shipyard show up in an open repo tab again. Repos added through a symlinked path, like /tmp, were missing every change.",
      "Linked git worktrees pick up branch, tag, and commit changes. Their HEAD and refs live in the main repo's git folder, which Shipyard was not watching.",
      "The Branches page has a search field. Type part of a name to filter the list, or press Command-F to jump to it. Delete selected only removes branches that match the search.",
      "A pull where every repo succeeds no longer pops open the output window. The toast has a View details button when there is output to read. The window still opens by itself when a pull fails.",
      "The commit window has a Commit all button that stages every changed file, including untracked files, and commits in one step. With nothing staged it replaces the old Stage a file to commit dead end. It is hidden while any file has conflicts.",
      "Dragging to select text in a dialog or the output window no longer closes it when the mouse is released outside. Clicking outside still closes it.",
      "Pull works on a branch that was pushed without tracking set up. Shipyard links it to the matching origin branch instead of failing with There is no tracking information for the current branch.",
      "Pushing a new branch sets up tracking, so later pulls and pushes go to the same origin branch.",
      "When a push is rejected because the remote has commits you don't have, Shipyard explains why and offers Pull, then push. It never force pushes.",
      "Pulling a branch that has diverged from the remote merges the remote commits when pull.rebase and pull.ff are not set, instead of stopping with Need to specify how to reconcile divergent branches.",
    ],
  },
  {
    version: "1.4.0",
    date: "September 24, 2026",
    notes: [
      "Discard changes is in the right-click menu for unstaged and staged files. Discarding a staged file keeps its unstaged edits, and Shipyard refuses instead of overwriting edits that sit next to the staged ones.",
      "Right-clicking a file in the unstaged or staged list selects it and opens its diff, without closing a diff that is already open.",
      "Right-clicking a file name no longer highlights the word under the cursor.",
    ],
  },
  {
    version: "1.3.1",
    date: "September 23, 2026",
    notes: [
      "macOS downloads are signed with a Developer ID and notarized by Apple, so installing Shipyard no longer shows the unidentified developer warning.",
    ],
  },
  {
    version: "1.3.0",
    date: "September 22, 2026",
    notes: [
      "The branch switcher always shows a search field, so the menu stays the same shape and long branch lists can be filtered.",
      "Auto-fetch interval stays in Settings. The status bar shows the next fetch, and hovering it shows the last fetch.",
      "Check for updates is back in Settings. A newer build uses the same install confirmation as launch.",
    ],
  },
  {
    version: "1.2.0",
    date: "September 22, 2026",
    notes: [
      "New branch asks which local branch to start from. The current branch is selected by default, and the new branch is checked out from that tip.",
      "Branch action buttons on the current or selected row use a stronger outline so they stay visible.",
    ],
  },
  {
    version: "1.1.2",
    date: "September 21, 2026",
    notes: [
      "Stage, unstage, and discard no longer freeze the window. File watching was reloading the commit graph for index updates, and status was writing the index again, so each click ran that work twice.",
    ],
  },
  {
    version: "1.1.1",
    date: "September 19, 2026",
    notes: [
      "Branches view no longer marks a brand-new local branch as Merged just because it still points at origin/main. Delete merged uses that same leftover list.",
    ],
  },
  {
    version: "1.1.0",
    date: "September 19, 2026",
    notes: [
      "1.0.1 through 1.0.8 were feature releases, not patches. From here on we follow semantic versioning: patch for fixes, minor for features, major for breaking changes.",
      "General settings pick the font and size for file diffs and the repository terminal, including a custom family name.",
      "History can pause recording so new git commands stay off the log until you resume.",
      "Open-repo fetch, pull, and other progress sits on the name row so the toolbar buttons stay put.",
      "Right-click a commit in the graph to check it out, create a branch or tag there, cherry-pick, revert, or copy its SHA or remote link. Command-click or Shift-click selects several commits for cherry-pick or revert.",
      "Merge a local branch into another from the branch switcher or a row in Branches. Checkout switches to the target first when needed, and conflicts use the existing resolve, continue, and abort flow.",
      "New branch and Merge into live in the branch switcher so the action bar stays Fetch, Pull, Push, and the view toggles.",
      "The branch switcher marks local-only branches and shows ahead or behind counts when a branch is out of date with its remote.",
    ],
  },
  {
    version: "1.0.8",
    date: "September 18, 2026",
    notes: [
      "Refresh is now Fetch on repository and group actions, with Auto-fetch and Last fetch matching that wording. The button uses a cloud-down icon and sits before Pull. Open repositories have Fetch on the toolbar too.",
      "Repository rows can open the origin remote in a browser or open the folder in Finder.",
      "Open repositories watch the working tree and git metadata, so staged, unstaged, and File History lists update when files change in another app.",
      "File History no longer keeps a renamed or deleted file under its old name.",
      "Tags view on the repo toolbar lists local tags. New tag creates a lightweight or annotated tag on HEAD or a commit you specify; Delete removes it.",
      "Hover a diff line in Changes or File History to highlight it and see git blame for that line.",
      "Commit opens when the working tree has changes, even if nothing is staged. Close keeps the title and description for that repository until you commit; a mark on the button shows when a draft is waiting.",
      "Right-click or Control-click a staged or unstaged file to stage or unstage it, ignore it, stash it, open it in the chosen editor, show it in Finder, copy its path, or delete it.",
      "Hover a commit date in the graph to see the full date and time.",
      "Command-click or Shift-click branches to select them, then Delete selected in the footer. Current, develop, main, and master stay unselected.",
    ],
  },
  {
    version: "1.0.7",
    date: "September 17, 2026",
    notes: [
      "When a branch is ahead of its remote, Undo unpushed on the repo toolbar soft-resets those local commits and keeps the changes staged.",
    ],
  },
  {
    version: "1.0.6",
    date: "September 17, 2026",
    notes: [
      "File History on the repo toolbar opens a project file tree in the same right-hand pane as Changes. Folders expand on click, with search and collapse-all.",
      "File tree search treats spaces as wildcards.",
      "Gitignored files and folders appear muted. Ignored directories are listed without expanding their contents.",
      "Click a file to see its commit history. Click a commit to open that file’s diff on the left.",
      "File history diffs follow a file through renames, so commits from before a move still show their changes. A small colored marker sits below the first commit when the file was added, and between commits when it was moved, renamed, or copied.",
    ],
  },
  {
    version: "1.0.5",
    date: "September 17, 2026",
    notes: [
      "Pull toasts stay short. Git output opens in the scrollable command window only when there is more than a one-line result; Already up to date stays toast-only.",
    ],
  },
  {
    version: "1.0.4",
    date: "September 16, 2026",
    notes: [
      "Commit can amend the last commit. The checkbox fills in that message; a warning appears if it is already on the remote.",
      "Repository rows sit the branch icon closer to the branch name.",
      "Push and pull progress on an open repo sits after Push, with the branch badge and a spinner.",
      "When a pull leaves merge or rebase conflicts, the files list shows them with Open in the chosen editor, Mark resolved, Continue, and Abort. Repository rows show a conflict count.",
      "General settings pick the editor used on conflicted files, or the system default. The button uses that name. The editor list is shorter and sorted A–Z after System default.",
      "Git settings edit your global git config: name, email, default branch, pull.rebase, checkout.defaultRemote, and the config file itself.",
      "The repository terminal pane can be resized vertically. The height is saved in settings.",
      "Reset buttons restore the default files pane width and terminal height in General settings.",
      "File list paths stay left-aligned; only the directory portion truncates when space is tight.",
      "Expand and Collapse in the repositories toolbar are hidden until at least one group exists.",
    ],
  },
  {
    version: "1.0.3",
    date: "September 16, 2026",
    notes: [
      "Optional active hours for auto-refresh in Schedule settings. Off by default; Business (8am–6pm) and Personal (6am–11pm) presets, or custom times. Automatic fetches pause outside the window.",
      "Pull next to Refresh pulls the current branch for every standalone and grouped repository.",
      "Terminal on the open-repo toolbar opens a shell in the bottom half of the graph, branches, or stashes view, started in that repository.",
    ],
  },
  {
    version: "1.0.2",
    date: "September 15, 2026",
    notes: [
      "Branches view lists names immediately, then fills Merged/Partial in place. The list stays in the same order, and a spinner shows while leftover work is still being checked.",
      "Delete merged reuses that list and removes leftovers in one git call.",
    ],
  },
  {
    version: "1.0.1",
    date: "September 15, 2026",
    notes: [
      "Check for Updates in Settings, the Shipyard menu, or automatically on launch. Newer builds download from GitHub Releases and install in place.",
      "A green update button appears in the tab bar when a newer build is available.",
      "Settings has a left-hand menu: Updates, then General, Window, and the JSON file. The settings icon opens General.",
    ],
  },
  {
    version: "1.0.0",
    date: "September 15, 2026",
    notes: [
      "GitHub icon in the status bar opens the public repository.",
      "First official release.",
      "Click a commit in the graph to see the files it changed and open a diff.",
      "Stashes view on the repo toolbar to apply, pop, or drop saved changes. Stash working-tree changes from the files pane or that view.",
      "Branches view has checkout and rename buttons on each local branch.",
      "Settings tab has preferences for auto-refresh, diff layout, files pane width, and window size and position. Open settings.json from the top-right icon to edit, export, or import the full file.",
      "App data lives in settings.json. An existing groups.json is migrated on launch.",
      "Drag handles reorder group repos, standalone repos, or the groups themselves. Sort A–Z in the repositories toolbar sorts groups and standalone repos; the group menu sorts that group’s repos.",
      "Standalone repositories are spaced like groups, with a color and optional label from the row menu. Each row has pull, checkout, and refresh.",
      "Pull on groups, standalone repos, and the open-repo toolbar runs the current branch. The caret opens the existing branch-options modal.",
      "Repository rows and open repo tabs show a repo icon.",
      "Checking out a branch from the Branches view updates the current branch immediately.",
      "Open-repo toolbar is two lines: identity on top, git actions and view toggles below.",
      "Branches view marks leftover work as Merged or Partial, and delete-merged asks before force-deleting branches git will not remove safely.",
    ],
  },
  {
    version: "0.2.0",
    date: "September 14, 2026",
    notes: [
      "Branches view for leftover local work: merged vs still unique, per-branch delete, and delete-merged that never force-deletes or removes develop, main, master, or the current branch.",
      "History tab logs every git command Shipyard runs, with a commands/full-detail toggle, hide-status filter, and a clear action.",
      "Settings can be exported to a JSON file from the editor.",
      "Group headers show one active action at a time (refresh, pull, or checkout) with per-repo progress, the real branch name, and a cancel control.",
      "Refresh runs a bounded pool of fetches so a group updates in waves instead of one repo at a time. All row spinners appear together.",
      "Commit graph stays in a capped left rail and tightens lane spacing on busy histories, so subjects and authors stay readable.",
      "Branches page opens immediately. Merge checks run off the UI thread and use one --merged scan plus cherry only for leftover branches.",
      "Change Log tab, opened from the version number in the status bar.",
    ],
  },
  {
    version: "0.1.0",
    date: "September 11, 2026",
    notes: [
      "First release: a local-first macOS Git client that shells out to the system git binary. No in-app login.",
      "Repository groups and standalone repos, persisted at groups.json.",
      "Live branch, ahead/behind, and working-tree counts on each repo row.",
      "Pull and checkout across a group, with a pull-from branch and checkout fallbacks.",
      "Repo tabs with a commit graph, working tree (stage, unstage, discard), inline or side-by-side diffs, and commit.",
      "Repo toolbar: branch switcher, new branch, pull, push, and a Changes toggle.",
      "JSON settings editor, window position restore, group header colors on tabs, and keyboard tab close.",
    ],
  },
];
