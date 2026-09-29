import { computed, onUnmounted, ref } from "vue";

export interface DashboardLayout {
  /** Group ids and ungrouped item ids, in display order. */
  root: string[];
  groups: Record<string, string[]>;
}

type DragKind = "group" | "item";

interface Drag {
  kind: DragKind;
  id: string;
  /** The group the item started in, or null for the top level. */
  from: string | null;
}

interface DashboardDragOptions {
  layout: () => DashboardLayout;
  itemSelector: string;
  itemKey: string;
  isCollapsed: (groupId: string) => boolean;
  canMove?: (from: string | null, to: string | null) => boolean;
  commitRoot: (ids: string[]) => Promise<void>;
  commitGroup: (itemId: string, groupId: string, ids: string[]) => Promise<void>;
  onError: (err: unknown) => void;
}

const BODY_CLASSES: Record<DragKind, string> = {
  group: "reordering-groups",
  item: "reordering-repos",
};

function sameIds(left: string[] = [], right: string[] = []) {
  return left.join("\0") === right.join("\0");
}

function locate(layout: DashboardLayout, id: string): string | null | undefined {
  if (layout.root.includes(id)) {
    return null;
  }
  return Object.keys(layout.groups).find((key) => layout.groups[key].includes(id));
}

function without(layout: DashboardLayout, id: string): DashboardLayout {
  return {
    root: layout.root.filter((item) => item !== id),
    groups: Object.fromEntries(
      Object.entries(layout.groups).map(([key, ids]) => [key, ids.filter((item) => item !== id)]),
    ),
  };
}

function midpoint(element: Element) {
  const rect = element.getBoundingClientRect();
  return rect.top + rect.height / 2;
}

function headerOf(element: Element) {
  return element.querySelector(":scope > .group-header") ?? element;
}

/**
 * Drag and drop for the dashboard, where groups and ungrouped items share one
 * top-level list. Expects `[data-dashboard-root] > [data-dashboard-id]` for
 * top-level entries and `[data-group-id]` sections with a `.group-header`.
 */
export function useDashboardDrag(options: DashboardDragOptions) {
  const drag = ref<Drag | null>(null);
  const draft = ref<DashboardLayout | null>(null);

  const draggingGroupId = computed(() => (drag.value?.kind === "group" ? drag.value.id : null));
  const draggingItemId = computed(() => (drag.value?.kind === "item" ? drag.value.id : null));

  const dropGroupId = computed(() => {
    if (!drag.value || !draft.value || drag.value.kind !== "item") {
      return null;
    }
    const location = locate(draft.value, drag.value.id);
    return typeof location === "string" && location !== drag.value.from ? location : null;
  });

  const collapsedDrop = computed(
    () => dropGroupId.value !== null && options.isCollapsed(dropGroupId.value),
  );

  function update(next: DashboardLayout) {
    const current = draft.value;
    const unchanged =
      current &&
      sameIds(next.root, current.root) &&
      Object.keys(next.groups).every((key) => sameIds(next.groups[key], current.groups[key]));
    if (!unchanged) {
      draft.value = next;
    }
  }

  function allowed(to: string | null) {
    const from = drag.value?.from ?? null;
    return from === to || !options.canMove || options.canMove(from, to);
  }

  function placeInRoot(beforeId: string | null) {
    const current = drag.value;
    if (!current || !draft.value || (current.kind === "item" && !allowed(null))) {
      return;
    }
    const next = without(draft.value, current.id);
    const index = beforeId ? next.root.indexOf(beforeId) : -1;
    next.root.splice(index === -1 ? next.root.length : index, 0, current.id);
    update(next);
  }

  function placeInGroup(groupId: string, targetId: string | null, before: boolean) {
    const current = drag.value;
    if (
      !current ||
      !draft.value ||
      !(groupId in draft.value.groups) ||
      targetId === current.id ||
      !allowed(groupId)
    ) {
      return;
    }
    const next = without(draft.value, current.id);
    const ids = next.groups[groupId];
    let index = targetId ? ids.indexOf(targetId) : before ? 0 : ids.length;
    if (index === -1) {
      return;
    }
    if (targetId && !before) {
      index += 1;
    }
    ids.splice(index, 0, current.id);
    update(next);
  }

  function rootBefore(event: PointerEvent) {
    const entries = document.querySelectorAll<HTMLElement>(
      "[data-dashboard-root] > [data-dashboard-id]",
    );
    for (const entry of entries) {
      const id = entry.dataset.dashboardId;
      if (id && id !== drag.value?.id && event.clientY < midpoint(headerOf(entry))) {
        return id;
      }
    }
    return null;
  }

  /**
   * Items dropped on the lower half of a group header, or anywhere in its
   * body, go into the group. Everywhere else places them at the top level.
   */
  function onMove(event: PointerEvent) {
    const current = drag.value;
    const layout = draft.value;
    if (!current || !layout) {
      return;
    }
    if (current.kind === "item") {
      const node = document.elementFromPoint(event.clientX, event.clientY);
      const group = node?.closest<HTMLElement>("[data-group-id]");
      const groupId = group?.dataset.groupId;
      if (node && group && groupId && groupId in layout.groups) {
        const header = headerOf(group);
        if (event.clientY >= midpoint(header)) {
          const row = node.closest<HTMLElement>(options.itemSelector);
          const rowId = row?.dataset[options.itemKey];
          if (row && rowId && group.contains(row)) {
            placeInGroup(groupId, rowId, event.clientY < midpoint(row));
          } else if (locate(layout, current.id) !== groupId) {
            placeInGroup(groupId, null, event.clientY <= header.getBoundingClientRect().bottom);
          }
          return;
        }
      }
    }
    placeInRoot(rootBefore(event));
  }

  function detach() {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", finish);
    window.removeEventListener("pointercancel", finish);
    document.body.classList.remove(...Object.values(BODY_CLASSES));
  }

  async function finish() {
    detach();
    const current = drag.value;
    const layout = draft.value;
    drag.value = null;
    draft.value = null;
    if (!current || !layout) {
      return;
    }
    const saved = options.layout();
    const target = locate(layout, current.id);
    try {
      if (target === null) {
        if (current.from !== null || !sameIds(layout.root, saved.root)) {
          await options.commitRoot(layout.root);
        }
      } else if (
        target !== undefined &&
        (target !== current.from || !sameIds(layout.groups[target], saved.groups[target]))
      ) {
        await options.commitGroup(current.id, target, layout.groups[target]);
      }
    } catch (err) {
      options.onError(err);
    }
  }

  function start(event: PointerEvent, kind: DragKind, id: string) {
    if (event.button !== 0) {
      return;
    }
    const layout = options.layout();
    const from = locate(layout, id);
    if (from === undefined) {
      return;
    }
    event.preventDefault();
    drag.value = { kind, id, from };
    draft.value = layout;
    document.body.classList.add(BODY_CLASSES[kind]);
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", finish);
  }

  /*
   * While hovering a collapsed group, the source list keeps its saved layout,
   * otherwise the group's header shifts out from under the pointer.
   */
  function rootIds(saved: string[]) {
    if (!draft.value || (collapsedDrop.value && drag.value?.from === null)) {
      return saved;
    }
    return draft.value.root;
  }

  function groupItemIds(groupId: string) {
    if (!draft.value || (collapsedDrop.value && drag.value?.from === groupId)) {
      return null;
    }
    return draft.value.groups[groupId] ?? null;
  }

  onUnmounted(detach);

  return { draggingGroupId, draggingItemId, dropGroupId, start, rootIds, groupItemIds };
}
