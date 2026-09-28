export type DiffKind = "meta" | "hunk" | "add" | "del" | "ctx";

export interface DiffLine {
  kind: DiffKind;
  text: string;
  oldNo?: number;
  newNo?: number;
}

export interface SplitRow {
  leftKind: DiffKind;
  rightKind: DiffKind;
  leftText: string;
  rightText: string;
  leftNo?: number;
  rightNo?: number;
}

export function parseDiff(raw: string): DiffLine[] {
  const lines: DiffLine[] = [];
  let oldNo = 0;
  let newNo = 0;
  let inHeader = false;

  for (const line of raw.split(/\r?\n/)) {
    if (line.startsWith("diff ")) {
      inHeader = true;
      lines.push({ kind: "meta", text: line });
      continue;
    }

    if (line.startsWith("@@")) {
      inHeader = false;
      const match = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
      if (match) {
        oldNo = Number(match[1]);
        newNo = Number(match[2]);
      }
      lines.push({ kind: "hunk", text: line });
      continue;
    }

    if (inHeader) {
      if (line !== "") {
        lines.push({ kind: "meta", text: line });
      }
      continue;
    }

    if (line.startsWith("+")) {
      lines.push({ kind: "add", text: line.slice(1), newNo });
      newNo += 1;
      continue;
    }

    if (line.startsWith("-")) {
      lines.push({ kind: "del", text: line.slice(1), oldNo });
      oldNo += 1;
      continue;
    }

    const text = line.startsWith(" ") ? line.slice(1) : line;
    lines.push({ kind: "ctx", text, oldNo, newNo });
    if (line !== "") {
      oldNo += 1;
      newNo += 1;
    }
  }

  return lines;
}

export function diffNotes(lines: DiffLine[]): string[] {
  const meta = lines.filter((line) => line.kind === "meta").map((line) => line.text);
  const hasContent = lines.some((line) => line.kind !== "meta");
  const value = (prefix: string) => meta.find((text) => text.startsWith(prefix))?.slice(prefix.length);
  const notes: string[] = [];

  const renameFrom = value("rename from ");
  const renameTo = value("rename to ");
  if (renameFrom && renameTo) {
    notes.push(`Renamed from ${renameFrom} to ${renameTo}`);
  }

  const copyFrom = value("copy from ");
  if (copyFrom) {
    notes.push(`Copied from ${copyFrom}`);
  }

  const oldMode = value("old mode ");
  const newMode = value("new mode ");
  if (oldMode && newMode) {
    notes.push(`File mode changed from ${oldMode} to ${newMode}`);
  }

  if (meta.some((text) => text.startsWith("Binary files ") || text === "GIT binary patch")) {
    notes.push("Binary file changed");
  } else if (!hasContent && value("new file mode ")) {
    notes.push("Empty file added");
  } else if (!hasContent && value("deleted file mode ")) {
    notes.push("Empty file deleted");
  }

  if (!hasContent && meta.length > 0 && notes.length === 0) {
    notes.push("No content changes");
  }

  return notes;
}

export function toSplitRows(lines: DiffLine[]): SplitRow[] {
  const rows: SplitRow[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];
    if (line.kind === "meta" || line.kind === "hunk") {
      rows.push({
        leftKind: line.kind,
        rightKind: line.kind,
        leftText: line.text,
        rightText: line.text,
      });
      index += 1;
      continue;
    }

    if (line.kind === "ctx") {
      rows.push({
        leftKind: "ctx",
        rightKind: "ctx",
        leftText: line.text,
        rightText: line.text,
        leftNo: line.oldNo,
        rightNo: line.newNo,
      });
      index += 1;
      continue;
    }

    const dels: DiffLine[] = [];
    const adds: DiffLine[] = [];
    while (index < lines.length && lines[index].kind === "del") {
      dels.push(lines[index]);
      index += 1;
    }
    while (index < lines.length && lines[index].kind === "add") {
      adds.push(lines[index]);
      index += 1;
    }

    const count = Math.max(dels.length, adds.length);
    for (let i = 0; i < count; i += 1) {
      const del = dels[i];
      const add = adds[i];
      rows.push({
        leftKind: del ? "del" : "ctx",
        rightKind: add ? "add" : "ctx",
        leftText: del?.text ?? "",
        rightText: add?.text ?? "",
        leftNo: del?.oldNo,
        rightNo: add?.newNo,
      });
    }
  }

  return rows;
}
