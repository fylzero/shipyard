<script setup lang="ts">
import type { CommitFile } from "../types";
import FileStatusIcon from "./FileStatusIcon.vue";
import PathLabel from "./PathLabel.vue";

defineProps<{
  title: string;
  meta: string;
  files: CommitFile[];
  selectedPath: string;
  loading?: boolean;
}>();

const emit = defineEmits<{
  select: [file: CommitFile];
  close: [];
}>();

function fileTitle(file: CommitFile) {
  return file.oldPath ? `${file.oldPath} → ${file.path}` : file.path;
}
</script>

<template>
  <div class="file-pane">
    <section class="file-section">
      <div class="pane-header commit-files-header">
        <div class="commit-files-title">
          <div class="file-heading">
            <strong :title="title">{{ title }}</strong>
            <span class="file-count-badge">{{ files.length }}</span>
          </div>
          <p class="commit-files-meta muted tiny">{{ meta }}</p>
        </div>
        <button class="ghost tiny" type="button" @click="emit('close')">Close</button>
      </div>
      <div class="file-list">
        <p v-if="loading" class="muted tiny empty-files">Loading files…</p>
        <p v-else-if="!files.length" class="muted tiny empty-files">No files changed.</p>
        <div
          v-for="file in files"
          :key="file.path"
          class="file-item"
          :class="{ active: selectedPath === file.path }"
        >
          <button
            class="file-item-main"
            type="button"
            :title="fileTitle(file)"
            @click="emit('select', file)"
          >
            <FileStatusIcon :status="file.status" />
            <PathLabel class="file-item-path" :path="file.path" />
          </button>
        </div>
      </div>
    </section>
  </div>
</template>
