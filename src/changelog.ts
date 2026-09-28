export interface ChangelogRelease {
  version: string;
  date: string;
  notes: string[];
}

export const CHANGELOG: ChangelogRelease[] = [
  {
    version: "1.6.2",
    date: "September 28, 2026",
    notes: [
      "Shipyard is now licensed under the Functional Source License (FSL-1.1-MIT). You can use, change, and share it freely, including at work, but not sell it as a competing product. Each release becomes MIT two years after it comes out.",
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
