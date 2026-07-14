<script setup lang="ts">
import { computed, ref, watch, watchEffect } from "vue";
import {
  NConfigProvider,
  NGlobalStyle,
  NMessageProvider,
  darkTheme,
  type GlobalTheme,
  type GlobalThemeOverrides,
} from "naive-ui";

import PortKillWorkbench from "./components/PortKillWorkbench.vue";
import {
  darkThemeOverrides,
  lightThemeOverrides,
} from "./styles/naiveTheme";

const THEME_STORAGE_KEY = "port-kill.theme";

function loadInitialTheme() {
  if (typeof window === "undefined") {
    return true;
  }

  const savedTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (savedTheme === "dark" || savedTheme === "light") {
    return savedTheme === "dark";
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

const isDarkTheme = ref(loadInitialTheme());

const currentTheme = computed<GlobalTheme | null>(() =>
  isDarkTheme.value ? darkTheme : null,
);
const currentThemeOverrides = computed<GlobalThemeOverrides>(() =>
  isDarkTheme.value ? darkThemeOverrides : lightThemeOverrides,
);

watchEffect(() => {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.dataset.theme = isDarkTheme.value
    ? "dark"
    : "light";
});

watch(isDarkTheme, (isDark) => {
  window.localStorage.setItem(THEME_STORAGE_KEY, isDark ? "dark" : "light");
});

function toggleTheme() {
  isDarkTheme.value = !isDarkTheme.value;
}
</script>

<template>
  <n-config-provider
    :theme="currentTheme"
    :theme-overrides="currentThemeOverrides"
  >
    <n-message-provider placement="top" :duration="5000">
      <n-global-style />

      <main class="app-shell" :data-theme="isDarkTheme ? 'dark' : 'light'">
        <div
          class="window-drag-strip"
          data-tauri-drag-region
          aria-hidden="true"
        />
        <PortKillWorkbench
          :is-dark-theme="isDarkTheme"
          @toggle-theme="toggleTheme"
        />
      </main>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
.app-shell {
  position: relative;
  height: 100vh;
  padding: 30px 14px 14px;
  overflow: hidden;
  color: var(--app-text);
  background: var(--app-bg);
  transition:
    color 0.3s ease,
    background-color 0.3s ease;
}

.window-drag-strip {
  position: absolute;
  top: 0;
  right: 8px;
  left: 76px;
  z-index: 10;
  height: 30px;
  user-select: none;
}

@media (max-width: 720px) {
  .app-shell {
    padding: 30px 10px 10px;
  }

  .window-drag-strip {
    right: 6px;
    left: 76px;
  }
}
</style>
