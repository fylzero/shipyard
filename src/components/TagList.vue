<script lang="ts">
export function filterTags<T extends { name: string; hash: string; message?: string }>(
  tags: T[],
  query: string,
) {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return tags;
  }
  return tags.filter(
    (tag) =>
      tag.name.toLowerCase().includes(needle) ||
      tag.hash.toLowerCase().startsWith(needle) ||
      (tag.message ?? "").toLowerCase().includes(needle),
  );
}
</script>

<script setup lang="ts">
import { computed, ref } from "vue";
import BranchContextMenu from "./BranchContextMenu.vue";
import RemotePicker from "./RemotePicker.vue";
import type { BranchesView } from "./BranchesHeader.vue";
import type { RemoteEntry, RemoteTag, RemoteTagOverview, TagEntry } from "../types";
import { formatCommitDate } from "../graphLayout";

const props = defineProps<{
  view: BranchesView;
  tags: TagEntry[];
  remotes: RemoteEntry[];
  selectedRemote: string;
  remoteTags: RemoteTagOverview | null;
  remoteLoading: boolean;
  remoteError: string;
  busy: boolean;
  query: string;
}>();

const emit = defineEmits<{
  create: [];
  delete: [tag: TagEntry];
  publish: [tag: TagEntry];
  selectRemote: [name: string];
  addRemote: [];
  refreshRemote: [];
  fetchRemote: [tag: RemoteTag];
  deleteRemote: [tag: RemoteTag];
}>();

const nameMenu = ref<{ name: string; x: number; y: number } | null>(null);

const remote = computed(() => props.selectedRemote);
const remoteEntry = computed(
  () => props.remotes.find((entry) => entry.name === props.selectedRemote) ?? null,
);
const remoteList = computed(() =>
  props.remoteTags?.remote === props.selectedRemote ? props.remoteTags.tags : null,
);
const remoteByName = computed(
  () => new Map((remoteList.value ?? []).map((tag) => [tag.name, tag])),
);
const localByName = computed(() => new Map(props.tags.map((tag) => [tag.name, tag])));

const visibleTags = computed(() => filterTags(props.tags, props.query));
const visibleRemoteTags = computed(() =>
  filterTags(
    (remoteList.value ?? []).map((tag) => ({
      ...tag,
      message: localByName.value.get(tag.name)?.message ?? "",
    })),
    props.query,
  ),
);

type LocalSync = "unknown" | "published" | "unpublished" | "differs";

function localSync(tag: TagEntry): LocalSync {
  if (!remoteList.value) {
    return "unknown";
  }
  const match = remoteByName.value.get(tag.name);
  if (!match) {
    return "unpublished";
  }
  return match.sync === "same" ? "published" : "differs";
}

const unpublishedCount = computed(
  () => props.tags.filter((tag) => localSync(tag) === "unpublished").length,
);

const localSummary = computed(() => {
  if (!props.remotes.length || !remote.value) {
    return "Tags mark a commit. Annotated tags keep a message; lightweight tags are just a name.";
  }
  if (props.remoteLoading && !remoteList.value) {
    return `Checking which tags are on ${remote.value}…`;
  }
  if (!remoteList.value) {
    return `Couldn't reach ${remote.value} to check which tags are published.`;
  }
  if (!props.tags.length) {
    return "Tags mark a commit. Annotated tags keep a message; lightweight tags are just a name.";
  }
  const count = unpublishedCount.value;
  if (!count) {
    return `Every local tag is published to ${remote.value}.`;
  }
  return `${count} ${count === 1 ? "tag is" : "tags are"} only on this machine. Publish to share with ${remote.value}.`;
});

function publishTitle(tag: TagEntry) {
  const sync = localSync(tag);
  if (sync === "published") {
    return `${tag.name} is already on ${remote.value}`;
  }
  if (sync === "differs") {
    return `${remote.value} has a different ${tag.name}. Delete it there first to publish yours.`;
  }
  return `Push ${tag.name} to ${remote.value}`;
}

function keepNameUnselected(event: MouseEvent) {
  if (event.button === 2) {
    event.preventDefault();
  }
}

function openNameMenu(event: MouseEvent, name: string) {
  event.preventDefault();
  event.stopPropagation();
  window.getSelection()?.removeAllRanges();
  nameMenu.value = { name, x: event.clientX, y: event.clientY };
}
</script>

<template>
  <div class="branch-pane">
    <template v-if="view === 'local'">
      <div class="remote-header">
        <p class="muted tiny tag-header-copy" :title="localSummary">
          <span
            v-if="remoteLoading && !remoteList && remotes.length"
            class="spinner"
            aria-hidden="true"
          />
          {{ localSummary }}
        </p>
        <div class="remote-header-actions">
          <button class="ghost tiny" type="button" :disabled="busy" @click="emit('create')">
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3Z"
              />
              <path d="M6 6h.01" />
            </svg>
            New tag
          </button>
        </div>
      </div>
      <div class="graph-scroll branch-list">
        <p v-if="!tags.length" class="muted tiny empty-files">No tags.</p>
        <p v-else-if="!visibleTags.length" class="muted tiny empty-files">
          No tags match that search.
        </p>
        <div
          v-for="tag in visibleTags"
          :key="tag.name"
          class="branch-row tag-row"
          @mousedown="keepNameUnselected"
          @contextmenu="openNameMenu($event, tag.name)"
        >
          <svg class="button-icon tag-row-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3Z"
            />
            <path d="M6 6h.01" />
          </svg>
          <span class="branch-row-name" :title="tag.name">{{ tag.name }}</span>
          <span v-if="tag.annotated" class="branch-pill annotated">Annotated</span>
          <span
            v-if="localSync(tag) === 'unpublished'"
            class="branch-pill partial"
            :title="`${tag.name} is not on ${remote}`"
          >
            Not published
          </span>
          <span
            v-else-if="localSync(tag) === 'differs'"
            class="branch-pill differs"
            :title="`${remote} has a tag named ${tag.name} that points somewhere else`"
          >
            Differs on {{ remote }}
          </span>
          <span v-if="tag.message" class="tag-row-message" :title="tag.message">{{ tag.message }}</span>
          <span v-if="tag.hash" class="tag-row-hash" :title="tag.hash">{{ tag.hash }}</span>
          <span v-if="tag.date" class="stash-row-date">{{ formatCommitDate(tag.date) }}</span>
          <div class="stash-row-actions">
            <button
              v-if="remotes.length && remote"
              class="ghost tiny"
              type="button"
              :disabled="busy || localSync(tag) === 'published' || localSync(tag) === 'differs'"
              :title="publishTitle(tag)"
              @click="emit('publish', tag)"
            >
              <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 19.5v-15m0 0-6.75 6.75M12 4.5l6.75 6.75" />
              </svg>
              Publish
            </button>
            <button
              class="ghost tiny danger"
              type="button"
              :disabled="busy"
              :title="`Delete ${tag.name} on this machine`"
              @click="emit('delete', tag)"
            >
              <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M6 18 18 6M6 6l12 12" />
              </svg>
              Delete
            </button>
          </div>
        </div>
      </div>
    </template>
    <template v-else>
      <div v-if="!remotes.length" class="tag-pane-header">
        <p class="muted tiny">
          This repository has no remotes. Add one to publish tags, like
          <strong>origin</strong> for where you push your work.
        </p>
        <button class="ghost tiny commit" type="button" :disabled="busy" @click="emit('addRemote')">
          Add remote
        </button>
      </div>
      <template v-else>
        <div class="remote-header">
          <RemotePicker
            :remotes="remotes"
            :selected="selectedRemote"
            :busy="busy"
            :show-counts="false"
            @select="emit('selectRemote', $event)"
            @add="emit('addRemote')"
          />
          <span v-if="remoteEntry" class="remote-url muted tiny" :title="remoteEntry.pushUrl">
            {{ remoteEntry.pushUrl }}
          </span>
          <div class="remote-header-actions">
            <button
              class="ghost tiny"
              type="button"
              :disabled="busy || remoteLoading"
              :title="`Check the tags on ${selectedRemote} again`"
              @click="emit('refreshRemote')"
            >
              Refresh
              <span v-if="remoteLoading" class="spinner" aria-hidden="true" />
            </button>
          </div>
        </div>
        <div class="graph-scroll branch-list">
          <p v-if="remoteLoading && !remoteList" class="muted tiny branch-list-hint">
            <span class="spinner" aria-hidden="true" />
            <span>Loading tags on {{ selectedRemote }}…</span>
          </p>
          <p v-else-if="!remoteList" class="muted tiny empty-files">
            {{ remoteError || `Couldn't load tags on ${selectedRemote}.` }}
          </p>
          <p v-else-if="!remoteList.length" class="muted tiny empty-files">
            No tags on {{ selectedRemote }}.
          </p>
          <p v-else-if="!visibleRemoteTags.length" class="muted tiny empty-files">
            No tags match that search.
          </p>
          <div
            v-for="tag in visibleRemoteTags"
            :key="tag.name"
            class="branch-row tag-row remote-row"
            @mousedown="keepNameUnselected"
            @contextmenu="openNameMenu($event, tag.name)"
          >
            <svg class="button-icon tag-row-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M9.568 3H5.25A2.25 2.25 0 003 5.25v4.318c0 .597.237 1.17.659 1.591l9.581 9.581c.699.699 1.78.872 2.607.33a18.095 18.095 0 005.223-5.223c.542-.827.369-1.908-.33-2.607L11.16 3.66A2.25 2.25 0 009.568 3Z"
              />
              <path d="M6 6h.01" />
            </svg>
            <span class="branch-row-name" :title="`${selectedRemote} tag ${tag.name}`">
              {{ tag.name }}
            </span>
            <span v-if="tag.annotated" class="branch-pill annotated">Annotated</span>
            <span
              v-if="tag.sync === 'remoteOnly'"
              class="branch-pill"
              :title="`${tag.name} is on ${selectedRemote} but not on this machine`"
            >
              Not fetched
            </span>
            <span
              v-else-if="tag.sync === 'differs'"
              class="branch-pill differs"
              :title="`Your local ${tag.name} points somewhere else`"
            >
              Differs from local
            </span>
            <span v-if="tag.message" class="tag-row-message" :title="tag.message">
              {{ tag.message }}
            </span>
            <span class="tag-row-hash" :title="tag.hash">{{ tag.hash.slice(0, 7) }}</span>
            <div class="branch-row-actions">
              <button
                class="ghost tiny"
                type="button"
                :disabled="busy || tag.sync !== 'remoteOnly'"
                :title="
                  tag.sync === 'remoteOnly'
                    ? `Fetch ${tag.name} from ${selectedRemote}`
                    : `You already have a local tag named ${tag.name}`
                "
                @click="emit('fetchRemote', tag)"
              >
                Fetch
              </button>
              <button
                class="ghost tiny danger"
                type="button"
                :disabled="busy"
                :title="`Delete ${tag.name} on ${selectedRemote}`"
                @click="emit('deleteRemote', tag)"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      </template>
    </template>
    <BranchContextMenu
      v-if="nameMenu"
      :branch="nameMenu.name"
      :x="nameMenu.x"
      :y="nameMenu.y"
      copy-label="Copy tag name"
      copied-message="Copied tag name"
      @close="nameMenu = null"
    />
  </div>
</template>
