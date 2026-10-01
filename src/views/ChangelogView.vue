<script setup lang="ts">
import { onMounted } from "vue";
import { CHANGELOG } from "../changelog";
import { useApp } from "../composables/useApp";
import { useUpdater } from "../composables/useUpdater";

const { showToast } = useApp();
const {
  status,
  statusText,
  currentVersion,
  availableVersion,
  busy,
  updateReady,
  ensureCurrentVersion,
  checkForUpdates,
  showPrompt,
} = useUpdater();

onMounted(() => {
  void ensureCurrentVersion();
});

async function onCheckForUpdates() {
  if (updateReady.value) {
    showPrompt();
    return;
  }
  await checkForUpdates({ prompt: true });
  if (status.value === "available") {
    showPrompt();
  } else if (status.value === "up-to-date") {
    showToast("You're on the latest version.");
  } else if (status.value === "error") {
    showToast(statusText.value, "error");
  }
}
</script>

<template>
  <div class="settings-page changelog-page">
    <div class="settings-inner">
      <div class="settings-header">
        <div>
          <div class="brand">Change Log</div>
          <p class="muted tiny">What shipped in each Shipyard release.</p>
        </div>
        <div class="settings-header-actions">
          <button class="ghost" type="button" :disabled="busy" @click="onCheckForUpdates">
            <template v-if="status === 'checking'">Checking…</template>
            <template v-else-if="updateReady && availableVersion">
              Update to v{{ availableVersion }}
            </template>
            <template v-else>Check for Updates</template>
          </button>
        </div>
      </div>
      <div class="changelog-scroll">
        <article
          v-for="release in CHANGELOG"
          :key="release.version"
          class="changelog-release"
        >
          <header class="changelog-release-head">
            <h2 class="changelog-version">{{ release.version }}</h2>
            <span v-if="release.version === currentVersion" class="changelog-current">
              Current
            </span>
            <p class="muted tiny">{{ release.date }}</p>
          </header>
          <ul class="changelog-notes">
            <li v-for="note in release.notes" :key="note">{{ note }}</li>
          </ul>
        </article>
      </div>
    </div>
  </div>
</template>
