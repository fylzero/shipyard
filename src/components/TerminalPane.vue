<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal, type ITheme } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import * as api from "../api";
import { useApp } from "../composables/useApp";
import { fontFaceName, resolveFontStack } from "../fonts";
import { resolvedTheme } from "../theme";
import type { ResolvedTheme } from "../types";

const TERMINAL_THEMES: Record<ResolvedTheme, ITheme> = {
  dark: {
    background: "#0d1016",
    foreground: "#e8edf5",
    cursor: "#118777",
    cursorAccent: "#0d1016",
    selectionBackground: "#2a3344",
    black: "#12151b",
    red: "#f07178",
    green: "#3dd68c",
    yellow: "#e6c07b",
    blue: "#7aa2f7",
    magenta: "#c678dd",
    cyan: "#56b6c2",
    white: "#e8edf5",
    brightBlack: "#8b95a8",
    brightRed: "#f07178",
    brightGreen: "#3dd68c",
    brightYellow: "#e6c07b",
    brightBlue: "#7aa2f7",
    brightMagenta: "#c678dd",
    brightCyan: "#56b6c2",
    brightWhite: "#ffffff",
  },
  light: {
    background: "#ffffff",
    foreground: "#1c2330",
    cursor: "#118777",
    cursorAccent: "#ffffff",
    selectionBackground: "#cfe0f7",
    black: "#1c2330",
    red: "#c9303a",
    green: "#15803d",
    yellow: "#a16207",
    blue: "#2c5fcf",
    magenta: "#9333ea",
    cyan: "#0e7490",
    white: "#6e7781",
    brightBlack: "#5f6b7d",
    brightRed: "#d1434b",
    brightGreen: "#178a52",
    brightYellow: "#b07a12",
    brightBlue: "#2f6fdb",
    brightMagenta: "#a855f7",
    brightCyan: "#0891b2",
    brightWhite: "#8c959f",
  },
};

const props = defineProps<{
  cwd: string;
  active: boolean;
}>();

const { terminalFontFamily, terminalFontSize } = useApp();

const host = ref<HTMLDivElement | null>(null);
const message = ref("");

let term: Terminal | null = null;
let fit: FitAddon | null = null;
let sessionId = "";
let stopData: (() => void) | undefined;
let stopExit: (() => void) | undefined;
let resizeObserver: ResizeObserver | null = null;
let starting = false;

function disposeListeners() {
  stopData?.();
  stopExit?.();
  stopData = undefined;
  stopExit = undefined;
}

async function closeSession() {
  const id = sessionId;
  sessionId = "";
  disposeListeners();
  if (id) {
    try {
      await api.closeTerminal(id);
    } catch {
      /* session may already be gone */
    }
  }
}

function writeMessage(text: string) {
  term?.writeln(`\r\n${text}`);
}

async function startSession() {
  if (!term || !fit || starting) {
    return;
  }
  starting = true;
  message.value = "";
  await closeSession();
  try {
    fit.fit();
    const id = await api.openTerminal(props.cwd, term.cols, term.rows);
    sessionId = id;
    stopData = await listen<{ id: string; data: string }>("terminal-data", (event) => {
      if (event.payload.id === sessionId) {
        term?.write(event.payload.data);
      }
    });
    stopExit = await listen<{ id: string }>("terminal-exit", (event) => {
      if (event.payload.id === sessionId) {
        sessionId = "";
        writeMessage("[Process exited]");
      }
    });
    term.focus();
  } catch (err) {
    message.value = String(err);
    writeMessage(String(err));
  } finally {
    starting = false;
  }
}

function syncSize() {
  if (!term || !fit || !sessionId) {
    fit?.fit();
    return;
  }
  fit.fit();
  void api.resizeTerminal(sessionId, term.cols, term.rows).catch(() => {
    /* closed while resizing */
  });
}

async function preloadTerminalFont() {
  const stack = resolveFontStack(terminalFontFamily.value);
  const face = fontFaceName(stack);
  try {
    if (face) {
      await document.fonts.load(`${terminalFontSize.value}px "${face}"`);
    }
    await document.fonts.ready;
  } catch {
    /* use the fallback stack if the face is not available */
  }
}

function applyTerminalFont() {
  if (!term) {
    return;
  }
  term.options.fontFamily = resolveFontStack(terminalFontFamily.value);
  term.options.fontSize = terminalFontSize.value;
}

onMounted(async () => {
  const node = host.value;
  if (!node) {
    return;
  }
  await preloadTerminalFont();
  term = new Terminal({
    cursorBlink: true,
    fontFamily: resolveFontStack(terminalFontFamily.value),
    fontSize: terminalFontSize.value,
    fontWeight: "400",
    fontWeightBold: "700",
    lineHeight: 1.35,
    scrollback: 5000,
    theme: TERMINAL_THEMES[resolvedTheme.value],
    macOptionIsMeta: true,
  });
  fit = new FitAddon();
  term.loadAddon(fit);
  term.open(node);
  term.onData((data) => {
    if (!sessionId) {
      return;
    }
    void api.writeTerminal(sessionId, data).catch(() => {
      /* closed while typing */
    });
  });
  resizeObserver = new ResizeObserver(() => {
    syncSize();
  });
  resizeObserver.observe(node);
  await nextTick();
  await new Promise<void>((resolve) => {
    if (node.clientWidth > 0 && node.clientHeight > 0) {
      resolve();
      return;
    }
    const ready = new ResizeObserver(() => {
      if (node.clientWidth > 0 && node.clientHeight > 0) {
        ready.disconnect();
        resolve();
      }
    });
    ready.observe(node);
    window.setTimeout(() => {
      ready.disconnect();
      resolve();
    }, 250);
  });
  await startSession();
});

onUnmounted(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
  void closeSession();
  term?.dispose();
  term = null;
  fit = null;
});

watch(
  () => props.cwd,
  () => {
    if (term) {
      void startSession();
    }
  },
);

watch(
  () => props.active,
  async (active) => {
    if (!active || !term) {
      return;
    }
    await nextTick();
    syncSize();
    term.focus();
  },
);

watch([terminalFontFamily, terminalFontSize], async () => {
  if (!term) {
    return;
  }
  await preloadTerminalFont();
  applyTerminalFont();
  syncSize();
});

watch(resolvedTheme, (theme) => {
  if (term) {
    term.options.theme = TERMINAL_THEMES[theme];
  }
});
</script>

<template>
  <section class="repo-terminal">
    <div class="repo-terminal-bar">
      <span class="repo-terminal-path" :title="cwd">{{ cwd }}</span>
    </div>
    <div ref="host" class="repo-terminal-host" />
    <p v-if="message" class="repo-terminal-error">{{ message }}</p>
  </section>
</template>
