<script setup lang="ts">
import { computed, ref } from "vue";
import {
  NConfigProvider,
  NGlobalStyle,
  NMessageProvider,
  darkTheme,
  type GlobalTheme,
  type GlobalThemeOverrides,
} from "naive-ui";

import PortKillWorkbench from "./components/PortKillWorkbench.vue";

const isDarkTheme = ref(true);

const darkThemeOverrides: GlobalThemeOverrides = {
  common: {
    bodyColor: "#07111f",
    cardColor: "#0c1828",
    modalColor: "#0f1d31",
    popoverColor: "#0f1d31",
    tableColor: "#091423",
    textColorBase: "#eaf1fb",
    textColor1: "#eaf1fb",
    textColor2: "#93a9c5",
    textColor3: "#6f86a2",
    borderColor: "rgba(91, 119, 149, 0.18)",
    primaryColor: "#4b7fff",
    primaryColorHover: "#5f90ff",
    primaryColorPressed: "#3866d0",
    successColor: "#2bbf8a",
    warningColor: "#f0a441",
    errorColor: "#ff6b78",
    fontFamily:
      '"IBM Plex Sans", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    fontFamilyMono:
      '"SF Mono", "JetBrains Mono", "IBM Plex Mono", ui-monospace, monospace',
  },
  DataTable: {
    thColor: "rgba(10, 21, 36, 0.94)",
    tdColor: "transparent",
    tdColorHover: "rgba(31, 52, 81, 0.75)",
    borderColor: "rgba(91, 119, 149, 0.16)",
  },
  Input: {
    color: "rgba(11, 23, 39, 0.92)",
    colorFocus: "rgba(11, 23, 39, 0.96)",
    borderHover: "1px solid rgba(92, 128, 175, 0.5)",
    boxShadowFocus: "0 0 0 2px rgba(75, 127, 255, 0.16)",
  },
  Card: {
    colorEmbedded: "rgba(10, 20, 34, 0.8)",
    borderColor: "rgba(91, 119, 149, 0.14)",
  },
};

const lightThemeOverrides: GlobalThemeOverrides = {
  common: {
    bodyColor: "#f3f7fc",
    cardColor: "#ffffff",
    modalColor: "#ffffff",
    popoverColor: "#ffffff",
    tableColor: "#ffffff",
    textColorBase: "#11233a",
    textColor1: "#11233a",
    textColor2: "#51657d",
    textColor3: "#6f8095",
    borderColor: "rgba(70, 98, 129, 0.16)",
    primaryColor: "#356dff",
    primaryColorHover: "#5887ff",
    primaryColorPressed: "#2452c9",
    successColor: "#21a97a",
    warningColor: "#e0932b",
    errorColor: "#d95a65",
    fontFamily:
      '"IBM Plex Sans", "SF Pro Display", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    fontFamilyMono:
      '"SF Mono", "JetBrains Mono", "IBM Plex Mono", ui-monospace, monospace',
  },
  DataTable: {
    thColor: "rgba(241, 246, 252, 0.98)",
    tdColor: "transparent",
    tdColorHover: "rgba(226, 236, 248, 0.78)",
    borderColor: "rgba(84, 112, 145, 0.14)",
  },
  Input: {
    color: "#ffffff",
    colorFocus: "#ffffff",
    borderHover: "1px solid rgba(65, 110, 205, 0.38)",
    boxShadowFocus: "0 0 0 2px rgba(53, 109, 255, 0.14)",
  },
  Card: {
    colorEmbedded: "rgba(248, 251, 255, 0.95)",
    borderColor: "rgba(84, 112, 145, 0.12)",
  },
};

const currentTheme = computed<GlobalTheme | null>(() => (isDarkTheme.value ? darkTheme : null));
const currentThemeOverrides = computed<GlobalThemeOverrides>(() =>
  isDarkTheme.value ? darkThemeOverrides : lightThemeOverrides,
);

function toggleTheme() {
  isDarkTheme.value = !isDarkTheme.value;
}
</script>

<template>
  <n-config-provider :theme="currentTheme" :theme-overrides="currentThemeOverrides">
    <n-message-provider placement="bottom-right">
      <n-global-style />

      <main :class="['app-shell', { 'app-shell--light': !isDarkTheme }]">
        <PortKillWorkbench :is-dark-theme="isDarkTheme" @toggle-theme="toggleTheme" />
      </main>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
:root {
  color-scheme: dark light;
}

.app-shell {
  height: 100vh;
  padding: 14px;
  overflow: hidden;
  color: #eaf1fb;
  background:
    radial-gradient(circle at top, rgba(61, 100, 214, 0.18), transparent 28%),
    linear-gradient(180deg, #07111f 0%, #050b14 100%);
  transition:
    color 0.3s ease,
    background-color 0.3s ease,
    background-image 0.3s ease;
}

.app-shell--light {
  color: #14273d;
  background:
    radial-gradient(circle at top, rgba(82, 127, 240, 0.18), transparent 28%),
    linear-gradient(180deg, #eff5fd 0%, #e4edf8 100%);
}

@media (max-width: 720px) {
  .app-shell {
    padding: 10px;
  }
}
</style>
