<script setup lang="ts">
import { computed, defineAsyncComponent, h } from "vue";
import {
  SETTINGS_SECTIONS,
  type SettingsSection,
  useTabs,
} from "../composables/useTabs";
import GeneralSettingsView from "./settings/GeneralSettingsView.vue";
import ScheduleSettingsView from "./settings/ScheduleSettingsView.vue";
import WindowSettingsView from "./settings/WindowSettingsView.vue";

const GitSettingsView = defineAsyncComponent({
  loader: () => import("./settings/GitSettingsView.vue"),
  errorComponent: {
    setup() {
      return () =>
        h("p", { class: "settings-error settings-pane-error" }, "Could not open Git settings.");
    },
  },
});

const JsonSettingsView = defineAsyncComponent({
  loader: () => import("./settings/JsonSettingsView.vue"),
  errorComponent: {
    setup() {
      return () =>
        h("p", { class: "settings-error settings-pane-error" }, "Could not open the JSON editor.");
    },
  },
});

const { settingsSection, openSettings } = useTabs();

const preferenceSections: { id: SettingsSection; label: string }[] = [
  { id: "general", label: "General" },
  { id: "git", label: "Git" },
  { id: "schedule", label: "Schedule" },
  { id: "window", label: "Window" },
  { id: "json", label: "JSON" },
];

const current = computed(() =>
  SETTINGS_SECTIONS.includes(settingsSection.value) ? settingsSection.value : "general",
);
</script>

<template>
  <div class="settings-layout">
    <nav class="settings-nav" aria-label="Settings">
      <button
        v-for="item in preferenceSections"
        :key="item.id"
        class="settings-nav-item"
        type="button"
        :class="{ active: current === item.id }"
        :aria-current="current === item.id ? 'page' : undefined"
        @click="openSettings(item.id)"
      >
        <svg v-if="item.id === 'general'" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 6h16" />
          <path d="M4 12h10" />
          <path d="M4 18h13" />
        </svg>
        <svg v-else-if="item.id === 'git'" viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="6" cy="5" r="2.25" />
          <circle cx="6" cy="19" r="2.25" />
          <circle cx="18" cy="12" r="2.25" />
          <path d="M6 7.25v9.5" />
          <path d="M8.1 5.6h5.4A4.5 4.5 0 0 1 18 10.1V12" />
        </svg>
        <svg v-else-if="item.id === 'schedule'" viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="12" cy="12" r="8" />
          <path d="M12 8v4.5L15 14" />
        </svg>
        <svg v-else-if="item.id === 'window'" viewBox="0 0 24 24" aria-hidden="true">
          <rect x="3.5" y="5.5" width="17" height="13" rx="1.5" />
          <path d="M3.5 9h17" />
        </svg>
        <svg v-else viewBox="0 0 24 24" aria-hidden="true">
          <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
          <path d="M14 2v4a2 2 0 0 0 2 2h4" />
        </svg>
        {{ item.label }}
      </button>
    </nav>
    <div class="settings-content" :class="{ fill: current === 'json' || current === 'git' }">
      <GeneralSettingsView v-if="current === 'general'" />
      <GitSettingsView v-if="current === 'git'" />
      <ScheduleSettingsView v-if="current === 'schedule'" />
      <WindowSettingsView v-if="current === 'window'" />
      <JsonSettingsView v-if="current === 'json'" />
    </div>
  </div>
</template>
