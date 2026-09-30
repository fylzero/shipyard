<script setup lang="ts">
import type { StashEntry } from "../types";
import { formatCommitDate } from "../graphLayout";

defineProps<{
  stashes: StashEntry[];
  selectedHash: string;
  busy: boolean;
  canStash: boolean;
}>();

const emit = defineEmits<{
  select: [stash: StashEntry];
  apply: [stash: StashEntry];
  pop: [stash: StashEntry];
  drop: [stash: StashEntry];
  push: [];
}>();

function stashRef(index: number) {
  return `stash@{${index}}`;
}
</script>

<template>
  <div class="branch-pane">
    <div class="graph-scroll branch-list">
      <p class="muted tiny branch-list-hint">
        Click a stash to preview its changes. Apply keeps the stash. Pop applies it and removes it.
        Drop deletes it without applying.
      </p>
      <p v-if="!stashes.length" class="muted tiny empty-files">No stashes.</p>
      <div
        v-for="stash in stashes"
        :key="stash.hash || stash.index"
        class="branch-row stash-row"
        :class="{ selected: selectedHash !== '' && selectedHash === stash.hash }"
        :aria-selected="selectedHash !== '' && selectedHash === stash.hash"
        @click="emit('select', stash)"
      >
        <span class="branch-pill">{{ stashRef(stash.index) }}</span>
        <span class="branch-row-name" :title="stash.message">{{ stash.message }}</span>
        <span class="stash-row-date">{{ formatCommitDate(stash.date) }}</span>
        <div class="stash-row-actions">
          <button
            class="ghost tiny"
            type="button"
            :disabled="busy"
            :title="`Apply ${stashRef(stash.index)}`"
            @click.stop="emit('apply', stash)"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M9 8.25H7.5a2.25 2.25 0 00-2.25 2.25v9a2.25 2.25 0 002.25 2.25h9a2.25 2.25 0 002.25-2.25v-9a2.25 2.25 0 00-2.25-2.25H15M9 12l3 3m0 0l3-3m-3 3V2.25"
              />
            </svg>
            Apply
          </button>
          <button
            class="ghost tiny"
            type="button"
            :disabled="busy"
            :title="`Pop ${stashRef(stash.index)}`"
            @click.stop="emit('pop', stash)"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M9 8.25H7.5a2.25 2.25 0 00-2.25 2.25v9a2.25 2.25 0 002.25 2.25h9a2.25 2.25 0 002.25-2.25v-9a2.25 2.25 0 00-2.25-2.25H15m0-3l-3-3m0 0l-3 3m3-3V15"
              />
            </svg>
            Pop
          </button>
          <button
            class="ghost tiny danger"
            type="button"
            :disabled="busy"
            :title="`Drop ${stashRef(stash.index)}`"
            @click.stop="emit('drop', stash)"
          >
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 18 18 6M6 6l12 12" />
            </svg>
            Drop
          </button>
        </div>
      </div>
    </div>
    <div class="branch-footer">
      <button
        class="ghost tiny"
        type="button"
        :disabled="busy || !canStash"
        @click="emit('push')"
      >
        <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M20.25 7.5l-.625 10.632a2.25 2.25 0 01-2.247 2.118H6.622a2.25 2.25 0 01-2.247-2.118L3.75 7.5M10 11.25h4M3.375 7.5h17.25c.621 0 1.125-.504 1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125Z"
          />
        </svg>
        Stash changes
      </button>
      <p class="muted tiny branch-footer-hint">
        Saves uncommitted work, including untracked files, and clears the working tree.
      </p>
    </div>
  </div>
</template>
