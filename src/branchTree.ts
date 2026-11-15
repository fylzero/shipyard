export interface BranchFolder<T> {
  kind: "folder";
  path: string;
  name: string;
  children: BranchTreeNode<T>[];
  branches: T[];
}

export interface BranchLeaf<T> {
  kind: "branch";
  path: string;
  name: string;
  branch: T;
}

export type BranchTreeNode<T> = BranchFolder<T> | BranchLeaf<T>;

export type BranchRow<T> =
  | { kind: "folder"; path: string; name: string; depth: number; open: boolean; branches: T[] }
  | { kind: "branch"; name: string; depth: number; branch: T };

function sortNodes<T>(nodes: BranchTreeNode<T>[]) {
  const folders = nodes
    .filter((node): node is BranchFolder<T> => node.kind === "folder")
    .sort((left, right) => left.name.localeCompare(right.name));
  const leaves = nodes.filter((node) => node.kind === "branch");
  return [...folders, ...leaves];
}

/**
 * A folder holding a single branch is shown as that branch with its remaining
 * path, and a folder whose only child is another folder is merged into it.
 */
function compact<T>(nodes: BranchTreeNode<T>[], prefix: string): BranchTreeNode<T>[] {
  return sortNodes(
    nodes.map((node): BranchTreeNode<T> => {
      if (node.kind === "branch") {
        return node;
      }
      if (node.branches.length === 1) {
        const branch = node.branches[0] as T & { name: string };
        return { kind: "branch", path: branch.name, name: branch.name.slice(prefix.length), branch };
      }
      let folder = node;
      while (folder.children.length === 1 && folder.children[0].kind === "folder") {
        const child: BranchFolder<T> = folder.children[0];
        folder = { ...child, name: `${folder.name}/${child.name}` };
      }
      return { ...folder, children: compact(folder.children, `${folder.path}/`) };
    }),
  );
}

export function buildBranchTree<T extends { name: string }>(branches: T[]): BranchTreeNode<T>[] {
  const root: BranchTreeNode<T>[] = [];
  const folders = new Map<string, BranchFolder<T>>();

  for (const branch of branches) {
    const segments = branch.name.split("/");
    let siblings = root;
    let path = "";
    for (const segment of segments.slice(0, -1)) {
      path = path ? `${path}/${segment}` : segment;
      let folder = folders.get(path);
      if (!folder) {
        folder = { kind: "folder", path, name: segment, children: [], branches: [] };
        folders.set(path, folder);
        siblings.push(folder);
      }
      folder.branches.push(branch);
      siblings = folder.children;
    }
    siblings.push({
      kind: "branch",
      path: branch.name,
      name: segments[segments.length - 1],
      branch,
    });
  }

  return compact(root, "");
}

export function flattenBranchTree<T>(
  nodes: BranchTreeNode<T>[],
  expanded: Set<string>,
  depth = 0,
  rows: BranchRow<T>[] = [],
): BranchRow<T>[] {
  for (const node of nodes) {
    if (node.kind === "branch") {
      rows.push({ kind: "branch", name: node.name, depth, branch: node.branch });
      continue;
    }
    const open = expanded.has(node.path);
    rows.push({ kind: "folder", path: node.path, name: node.name, depth, open, branches: node.branches });
    if (open) {
      flattenBranchTree(node.children, expanded, depth + 1, rows);
    }
  }
  return rows;
}

function foldersStorageKey(kind: "local" | "remote", path: string) {
  return `shipyard:branch-folders:${kind}:${path}`;
}

export function loadOpenFolders(kind: "local" | "remote", path: string): Set<string> {
  try {
    const raw = JSON.parse(localStorage.getItem(foldersStorageKey(kind, path)) ?? "[]");
    return new Set(Array.isArray(raw) ? raw.filter((item) => typeof item === "string") : []);
  } catch {
    return new Set();
  }
}

export function saveOpenFolders(kind: "local" | "remote", path: string, open: Set<string>) {
  try {
    localStorage.setItem(foldersStorageKey(kind, path), JSON.stringify([...open]));
  } catch {
    /* folders just won't stay open */
  }
}
