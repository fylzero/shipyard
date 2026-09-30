<script setup lang="ts">
import { computed, ref } from "vue";
import { useApp } from "../composables/useApp";
import type { RepoEntry } from "../types";
import BranchIcon from "./BranchIcon.vue";
import Modal from "./Modal.vue";
import SplitAction from "./SplitAction.vue";

type LastFallback = "develop" | "master" | "main";

const props = defineProps<{
  repo: RepoEntry;
}>();

const {
  statuses,
  isRepoRefreshing,
  refreshingAll,
  pullingAll,
  refreshStandaloneRepo,
  pullStandaloneRepo,
  checkoutStandaloneRepo,
} = useApp();

const modal = ref<"pull" | "checkout" | null>(null);
const pullSource = ref<"current" | "develop" | "master" | "main" | "specify">("current");
const specifyBranch = ref("");
const checkoutSource = ref<"develop" | "specify" | "master" | "main">("develop");
const checkoutTarget = ref("");
const extraFallbacks = ref<string[]>([]);
const lastFallback = ref<LastFallback>("develop");
const action = ref<"pull" | "checkout" | "refresh" | null>(null);
const actionBranch = ref("");

const progress = computed(() => {
  if (action.value) {
    return action.value;
  }
  if (pullingAll.value) {
    return "pull";
  }
  if (refreshingAll.value && isRepoRefreshing(props.repo.id)) {
    return "refresh";
  }
  return null;
});

const busy = computed(() => Boolean(progress.value));

const currentBranch = computed(() => statuses.value[props.repo.id]?.branch || "");
const pullTitle = computed(() =>
  currentBranch.value ? `Pull from ${currentBranch.value}` : "Pull from current branch",
);

const pullBranch = computed(() => {
  if (pullSource.value === "current") {
    return "";
  }
  if (pullSource.value === "specify") {
    return specifyBranch.value.trim();
  }
  return pullSource.value;
});

const canConfirmPull = computed(
  () => pullSource.value === "current" || Boolean(pullBranch.value),
);

const pullRemoteLabel = computed(() => {
  if (pullSource.value === "current") {
    return "current-branch";
  }
  return pullBranch.value || "…";
});

const pullHint = computed(() =>
  pullSource.value === "current"
    ? "Use this to pick up others’ commits on the same branch."
    : "Brings that remote branch into this checkout. Conflicts appear in the files list so you can open them, mark them resolved, or abort.",
);

const checkoutBranch = computed(() => {
  if (checkoutSource.value === "specify") {
    return checkoutTarget.value.trim();
  }
  return checkoutSource.value;
});

const canConfirmCheckout = computed(() => Boolean(checkoutBranch.value));

function fallbackList() {
  return [
    ...extraFallbacks.value.map((item) => item.trim()).filter(Boolean),
    lastFallback.value,
  ];
}

function openPull() {
  pullSource.value = "current";
  specifyBranch.value = "develop";
  modal.value = "pull";
}

function pullCurrent() {
  return runAction("pull", statuses.value[props.repo.id]?.branch || "", () =>
    pullStandaloneRepo(props.repo.id),
  );
}

function openCheckout() {
  checkoutSource.value = "develop";
  checkoutTarget.value = "";
  extraFallbacks.value = [];
  lastFallback.value = "develop";
  modal.value = "checkout";
}

function closeModal() {
  modal.value = null;
}

async function runAction(
  kind: "pull" | "checkout" | "refresh",
  branch: string,
  work: () => Promise<unknown>,
) {
  if (busy.value) {
    return;
  }
  action.value = kind;
  actionBranch.value = branch;
  try {
    await work();
  } finally {
    action.value = null;
    actionBranch.value = "";
  }
}

async function confirmPull() {
  if (!canConfirmPull.value) {
    return;
  }
  const branch = pullBranch.value;
  modal.value = null;
  return runAction(
    "pull",
    branch || statuses.value[props.repo.id]?.branch || "",
    () => pullStandaloneRepo(props.repo.id, branch || undefined),
  );
}

async function confirmCheckout() {
  if (!canConfirmCheckout.value) {
    return;
  }
  const target = checkoutBranch.value;
  const fallbacks = checkoutSource.value === "specify" ? fallbackList() : [];
  modal.value = null;
  return runAction("checkout", target, () =>
    checkoutStandaloneRepo(props.repo.id, target, fallbacks),
  );
}

function runRefresh() {
  return runAction("refresh", "", () => refreshStandaloneRepo(props.repo.id));
}

function addFallback() {
  extraFallbacks.value = [...extraFallbacks.value, ""];
}

function removeFallback() {
  extraFallbacks.value = extraFallbacks.value.slice(0, -1);
}
</script>

<template>
  <div class="repo-row-actions">
    <span v-if="progress === 'pull'" class="action-progress">
      Pulling
      <span
        v-if="actionBranch || currentBranch"
        class="action-branch-badge"
        :title="actionBranch || currentBranch"
      >
        <BranchIcon />
        <span class="action-branch-name">{{ actionBranch || currentBranch }}</span>
      </span>
      <span class="spinner" aria-hidden="true" />
    </span>
    <span v-else-if="progress === 'checkout'" class="action-progress">
      Checking out
      <span v-if="actionBranch" class="action-branch-badge" :title="actionBranch">
        <BranchIcon />
        <span class="action-branch-name">{{ actionBranch }}</span>
      </span>
      <span class="spinner" aria-hidden="true" />
    </span>
    <span v-else-if="progress === 'refresh'" class="action-progress">
      Fetching
      <span class="spinner" aria-hidden="true" />
    </span>
    <template v-else>
      <button class="ghost tiny" type="button" @click.stop="runRefresh">
        <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M12 9.75v6.75m0 0-3-3m3 3 3-3m-8.25 6a4.5 4.5 0 0 1-1.41-8.775 5.25 5.25 0 0 1 10.233-2.33 3 3 0 0 1 3.758 3.848A3.752 3.752 0 0 1 18 19.5H6.75Z"
          />
        </svg>
        Fetch
      </button>
      <SplitAction
        :primary-title="pullTitle"
        more-title="Pull from another branch"
        @primary="pullCurrent"
        @more="openPull"
      >
        <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5M16.5 12 12 16.5m0 0L7.5 12m4.5 4.5V3"
          />
        </svg>
        Pull
      </SplitAction>
      <button class="ghost tiny" type="button" @click.stop="openCheckout">
        <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"
          />
        </svg>
        Checkout
      </button>
    </template>
  </div>

  <Modal v-if="modal === 'pull'" title="Pull from remote" @close="closeModal">
    <p class="pull-summary">
      Merges <code>origin/{{ pullRemoteLabel }}</code> into the currently checked-out branch.
      Checkout does not change.
    </p>
    <fieldset class="radio-list">
      <legend class="muted tiny">Remote branch</legend>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="current" />
        origin/current-branch
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="develop" />
        origin/develop
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="master" />
        origin/master
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="main" />
        origin/main
      </label>
      <label class="radio-option">
        <input v-model="pullSource" type="radio" value="specify" />
        Specify
      </label>
      <input
        v-if="pullSource === 'specify'"
        v-model="specifyBranch"
        type="text"
        placeholder="branch name"
        autocapitalize="off"
        autocorrect="off"
        autocomplete="off"
        spellcheck="false"
        autofocus
        @keydown.enter="confirmPull"
      />
    </fieldset>
    <p class="muted tiny pull-hint">{{ pullHint }}</p>
    <template #actions>
      <button class="ghost" type="button" @click="closeModal">Cancel</button>
      <button class="primary" type="button" :disabled="!canConfirmPull" @click="confirmPull">
        Pull
      </button>
    </template>
  </Modal>

  <Modal v-if="modal === 'checkout'" title="Checkout" @close="closeModal">
    <fieldset class="radio-list">
      <legend class="muted tiny">Branch</legend>
      <label class="radio-option">
        <input v-model="checkoutSource" type="radio" value="develop" />
        develop
      </label>
      <label class="radio-option">
        <input v-model="checkoutSource" type="radio" value="specify" />
        Specify
      </label>
      <div v-if="checkoutSource === 'specify'" class="specify-panel">
        <label class="modal-label">
          <span class="muted tiny">Check out this branch</span>
          <input
            v-model="checkoutTarget"
            type="text"
            placeholder="feature/JIRA-123"
            autocapitalize="off"
            autocorrect="off"
            autocomplete="off"
            spellcheck="false"
            autofocus
            @keydown.enter="confirmCheckout"
          />
        </label>
        <div class="specify-fallbacks">
          <p class="muted tiny specify-fallbacks-title">Fallbacks</p>
          <div v-if="extraFallbacks.length" class="fallback-editor">
            <input
              v-for="(_fallback, index) in extraFallbacks"
              :key="index"
              v-model="extraFallbacks[index]"
              type="text"
              placeholder="branch name"
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck="false"
            />
          </div>
          <div class="fallback-editor-actions">
            <button class="ghost tiny" type="button" @click="addFallback">
              {{ extraFallbacks.length ? "Add another fallback" : "Add a fallback" }}
            </button>
            <button
              v-if="extraFallbacks.length"
              class="ghost tiny"
              type="button"
              @click="removeFallback"
            >
              Remove
            </button>
          </div>
          <p class="muted tiny specify-fallbacks-title">Final fallback</p>
          <div class="specify-last-fallback">
            <label class="radio-option">
              <input v-model="lastFallback" type="radio" value="develop" />
              develop
            </label>
            <label class="radio-option">
              <input v-model="lastFallback" type="radio" value="master" />
              master
            </label>
            <label class="radio-option">
              <input v-model="lastFallback" type="radio" value="main" />
              main
            </label>
          </div>
        </div>
      </div>
      <label class="radio-option">
        <input v-model="checkoutSource" type="radio" value="master" />
        master
      </label>
      <label class="radio-option">
        <input v-model="checkoutSource" type="radio" value="main" />
        main
      </label>
    </fieldset>
    <template #actions>
      <button class="ghost" type="button" @click="closeModal">Cancel</button>
      <button
        class="primary"
        type="button"
        :disabled="!canConfirmCheckout"
        @click="confirmCheckout"
      >
        Checkout
      </button>
    </template>
  </Modal>
</template>
