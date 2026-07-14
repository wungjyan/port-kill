<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NButton,
  NEmpty,
  NModal,
  NRadioButton,
  NRadioGroup,
  NSpin,
  NSwitch,
  useMessage,
} from "naive-ui";

import {
  AlertTriangleIcon,
  MoonIcon,
  SettingsIcon,
  SunIcon,
} from "@lucide/vue";
import PortTable from "./PortTable.vue";
import PortToolbar from "./PortToolbar.vue";
import type {
  KillResult,
  PortListResponse,
  PortProcess,
  ReboundCheckResult,
  SortKey,
  SortOrder,
} from "../types";

type WorkbenchSettings = {
  autoRefreshEnabled: boolean;
  refreshIntervalMs: number;
  confirmBeforeTerminate: boolean;
};

defineEmits<{
  toggleTheme: [];
}>();

const message = useMessage();
const { isDarkTheme } = defineProps<{
  isDarkTheme: boolean;
}>();

const SETTINGS_STORAGE_KEY = "port-kill.workbench-settings";
const SLOW_SCAN_NOTICE_MS = 2_000;
const REBOUND_CHECK_DELAYS_MS = [800, 2_000, 5_000];
const DEFAULT_REFRESH_INTERVAL_MS = 10_000;
const DEFAULT_SETTINGS: WorkbenchSettings = {
  autoRefreshEnabled: true,
  refreshIntervalMs: DEFAULT_REFRESH_INTERVAL_MS,
  confirmBeforeTerminate: true,
};
const refreshIntervalOptions = [
  { label: "3 秒", value: 3_000 },
  { label: "5 秒", value: 5_000 },
  { label: "10 秒", value: 10_000 },
  { label: "15 秒", value: 15_000 },
  { label: "30 秒", value: 30_000 },
  { label: "60 秒", value: 60_000 },
];
const refreshIntervalValues = new Set(
  refreshIntervalOptions.map((option) => option.value),
);

const initialLoading = ref(true);
const refreshing = ref(false);
const settingsOpen = ref(false);
const settings = ref<WorkbenchSettings>(loadSettings());
const query = ref("");
const currentUser = ref("");
const items = ref<PortProcess[]>([]);
const sortKey = ref<SortKey>("recent");
const sortOrder = ref<SortOrder>("descend");
const activeKillPids = ref<number[]>([]);
const loadError = ref("");
const refreshError = ref("");
const lastLoadTime = ref<number>(0);
const lastUpdatedAt = ref<Date | null>(null);
let refreshTimer: number | null = null;
let currentLoadPromise: Promise<boolean> | null = null;
const reboundCheckTimers = new Set<number>();

const filteredItems = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) {
    return items.value;
  }

  // Keep the search broad enough for scanning, but prioritize the fields from the API doc.
  return items.value.filter((item) => {
    const haystack = [
      String(item.port),
      String(item.pid),
      item.processName,
      item.command,
      item.cwd ?? "",
      item.hostSummary,
    ]
      .join("\n")
      .toLowerCase();

    return haystack.includes(keyword);
  });
});

const sortedItems = computed(() => {
  const nextItems = [...filteredItems.value];

  const compare = (left: PortProcess, right: PortProcess) => {
    if (sortKey.value === "port") {
      return left.port - right.port || left.pid - right.pid;
    }

    if (sortKey.value === "process") {
      return (
        left.processName.localeCompare(right.processName) ||
        left.port - right.port ||
        left.pid - right.pid
      );
    }

    return (
      (left.startedAtTs ?? 0) - (right.startedAtTs ?? 0) ||
      left.port - right.port ||
      left.pid - right.pid
    );
  };

  nextItems.sort((left, right) => {
    const baseResult = compare(left, right);
    return sortOrder.value === "ascend" ? baseResult : -baseResult;
  });

  return nextItems;
});

const visibleSummary = computed(() => {
  const resultCount = filteredItems.value.length;
  const totalCount = items.value.length;

  if (query.value) {
    return `${resultCount} / ${totalCount} 个端口 · ${autoRefreshSummary.value}`;
  }

  return `${totalCount} 个端口 · ${autoRefreshSummary.value}`;
});

const autoRefreshSummary = computed(() =>
  settings.value.autoRefreshEnabled
    ? `自动刷新 ${refreshIntervalLabel.value}`
    : "自动刷新已关闭",
);

const relatedPortsByPid = computed<Record<number, number[]>>(() => {
  const result: Record<number, number[]> = {};

  for (const item of items.value) {
    const ports = result[item.pid] ?? [];
    if (!ports.includes(item.port)) {
      ports.push(item.port);
      ports.sort((left, right) => left - right);
    }
    result[item.pid] = ports;
  }

  return result;
});

const scanStatus = computed(() => {
  if (refreshing.value) {
    return "扫描中";
  }

  if (refreshError.value) {
    return "刷新失败";
  }

  if (lastUpdatedAt.value && lastLoadTime.value > 0) {
    return `${lastUpdatedAt.value.toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    })} · ${lastLoadTime.value}ms`;
  }

  return "等待中";
});

const refreshIntervalLabel = computed(
  () =>
    refreshIntervalOptions.find(
      (option) => option.value === settings.value.refreshIntervalMs,
    )?.label ?? `${Math.round(settings.value.refreshIntervalMs / 1_000)} 秒`,
);

function normalizeRefreshInterval(value: unknown) {
  if (value === 8_000) {
    return DEFAULT_REFRESH_INTERVAL_MS;
  }

  return typeof value === "number" && refreshIntervalValues.has(value)
    ? value
    : DEFAULT_REFRESH_INTERVAL_MS;
}

function loadSettings(): WorkbenchSettings {
  if (typeof window === "undefined") {
    return { ...DEFAULT_SETTINGS };
  }

  try {
    const rawValue = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (!rawValue) {
      return { ...DEFAULT_SETTINGS };
    }

    const parsed = JSON.parse(rawValue) as Partial<WorkbenchSettings>;
    const refreshIntervalMs = normalizeRefreshInterval(
      parsed.refreshIntervalMs,
    );

    return {
      autoRefreshEnabled:
        typeof parsed.autoRefreshEnabled === "boolean"
          ? parsed.autoRefreshEnabled
          : DEFAULT_SETTINGS.autoRefreshEnabled,
      refreshIntervalMs,
      confirmBeforeTerminate:
        typeof parsed.confirmBeforeTerminate === "boolean"
          ? parsed.confirmBeforeTerminate
          : DEFAULT_SETTINGS.confirmBeforeTerminate,
    };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

function persistSettings() {
  window.localStorage.setItem(
    SETTINGS_STORAGE_KEY,
    JSON.stringify(settings.value),
  );
}

function scheduleAutoRefresh() {
  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer);
    refreshTimer = null;
  }

  if (!settings.value.autoRefreshEnabled) {
    return;
  }

  refreshTimer = window.setTimeout(() => {
    void loadPorts({ silent: true });
  }, settings.value.refreshIntervalMs);
}

async function loadPorts(options: { silent?: boolean } = {}) {
  if (currentLoadPromise) {
    return currentLoadPromise;
  }

  const silent = options.silent ?? false;
  const hasExistingData = items.value.length > 0;

  if (!hasExistingData) {
    loadError.value = "";
  }
  refreshError.value = "";

  if (!hasExistingData && (!silent || initialLoading.value)) {
    initialLoading.value = true;
  } else {
    refreshing.value = true;
  }

  currentLoadPromise = (async () => {
    const startTime = performance.now();
    const slowNoticeId = setTimeout(() => {
      if (initialLoading.value && !silent) {
        message.warning("端口扫描耗时较长，请稍候...");
      }
    }, SLOW_SCAN_NOTICE_MS);

    try {
      const response = await invoke<PortListResponse>("list_ports");
      currentUser.value = response.currentUser;
      items.value = response.items;
      clearTimeout(slowNoticeId);
      loadError.value = "";
      refreshError.value = "";

      // Track load time for performance monitoring
      lastLoadTime.value = Math.round(performance.now() - startTime);
      lastUpdatedAt.value = new Date();
      return true;
    } catch (error) {
      clearTimeout(slowNoticeId);
      const errorMsg = String(error);

      if (items.value.length > 0) {
        refreshError.value = errorMsg;
      } else {
        loadError.value = errorMsg;
      }

      // Show user-friendly error message
      if (!silent) {
        if (errorMsg.includes("timeout") || errorMsg.includes("超时")) {
          message.error("端口扫描超时，请重试");
        } else if (errorMsg.includes("lsof")) {
          message.error("无法执行端口扫描命令，请检查系统权限");
        } else {
          message.error(`端口扫描失败：${errorMsg}`);
        }
      }
      return false;
    } finally {
      initialLoading.value = false;
      refreshing.value = false;
      currentLoadPromise = null;
      scheduleAutoRefresh();
    }
  })();

  return currentLoadPromise;
}

function updateSortByTable(nextSort: { key: SortKey; order: SortOrder }) {
  sortKey.value = nextSort.key;
  sortOrder.value = nextSort.order;
}

async function handleManualRefresh() {
  const succeeded = await loadPorts();
  if (succeeded) {
    message.success("刷新成功");
  }
}

function buildKillSuccessMessage(item: PortProcess, force: boolean) {
  const actionText = force ? "已强制结束" : "已结束";
  const processLabel = item.processName ? `“${item.processName}”` : "目标进程";
  const ports = relatedPortsByPid.value[item.pid] ?? [item.port];
  const portSummary =
    ports.length === 1
      ? `监听端口 ${ports[0]} 已停止`
      : `${ports.length} 个监听端口已停止（${ports.join("、")}）`;

  return `${actionText}${processLabel}（PID ${item.pid}），${portSummary}`;
}

function scheduleReboundCheck(item: PortProcess) {
  let completed = false;
  const groupTimers = new Set<number>();

  const clearRemainingTimers = () => {
    groupTimers.forEach((timerId) => {
      window.clearTimeout(timerId);
      reboundCheckTimers.delete(timerId);
    });
    groupTimers.clear();
  };

  REBOUND_CHECK_DELAYS_MS.forEach((delayMs) => {
    const timerId = window.setTimeout(async () => {
      groupTimers.delete(timerId);
      reboundCheckTimers.delete(timerId);

      if (completed) {
        return;
      }

      try {
        const result = await invoke<ReboundCheckResult>("check_port_rebound", {
          port: item.port,
          previousPid: item.pid,
          previousProcessName: item.processName,
        });

        if (!result.rebound) {
          return;
        }

        completed = true;
        clearRemainingTimers();
        message.warning(
          result.message ??
            `端口 ${item.port} 已被重新占用，请确认是否存在自动拉起的后台服务`,
        );
        await loadPorts({ silent: true });
      } catch {
        // Ignore rebound check failures to avoid interrupting the main kill flow.
      }
    }, delayMs);

    groupTimers.add(timerId);
    reboundCheckTimers.add(timerId);
  });
}

async function handleKill(item: PortProcess, force: boolean) {
  if (activeKillPids.value.includes(item.pid)) {
    return;
  }

  activeKillPids.value = [...activeKillPids.value, item.pid];

  try {
    await invoke<KillResult>("kill_process", {
      pid: item.pid,
      force,
      expectedPort: item.port,
      expectedStartedAtTs: item.startedAtTs,
      expectedProcessName: item.processName,
      expectedUser: item.user,
    });

    message.success(buildKillSuccessMessage(item, force));
    scheduleReboundCheck(item);
    await loadPorts({ silent: true });
  } catch (error) {
    const errorMsg = String(error);

    // Provide more context in error messages
    if (errorMsg.includes("不存在")) {
      message.warning(`进程 ${item.pid} 已退出`);
      await loadPorts({ silent: true }); // Refresh to update list
    } else if (errorMsg.includes("身份已变化")) {
      message.warning(errorMsg);
      await loadPorts({ silent: true });
    } else if (errorMsg.includes("权限")) {
      message.error(`无权限结束进程 ${item.pid}（${item.processName}）`);
    } else if (errorMsg.includes("未响应")) {
      message.warning(`进程 ${item.pid} 未响应，建议使用强制结束`);
    } else {
      message.error(`结束失败: ${errorMsg}`);
    }
  } finally {
    activeKillPids.value = activeKillPids.value.filter(
      (pid) => pid !== item.pid,
    );
  }
}

onMounted(() => {
  void loadPorts();
});

watch(
  settings,
  () => {
    persistSettings();
    scheduleAutoRefresh();
  },
  { deep: true },
);

onBeforeUnmount(() => {
  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer);
  }

  reboundCheckTimers.forEach((timerId) => {
    window.clearTimeout(timerId);
  });
  reboundCheckTimers.clear();
});
</script>

<template>
  <div
    :class="['workbench', { 'workbench--light': !isDarkTheme }]"
    data-tauri-drag-region
  >
    <header class="app-header" data-tauri-drag-region>
      <div class="header-brand">
        <h1 class="header-title">Port Kill</h1>
        <p class="header-desc">{{ visibleSummary }}</p>
      </div>

      <div class="header-actions">
        <button
          class="icon-button"
          type="button"
          :aria-label="
            isDarkTheme
              ? '当前为暗黑主题，点击切换到明亮主题'
              : '当前为明亮主题，点击切换到暗黑主题'
          "
          @click="$emit('toggleTheme')"
        >
          <SunIcon v-if="isDarkTheme" :size="18" aria-hidden="true" />
          <MoonIcon v-else :size="18" aria-hidden="true" />
        </button>

        <button
          class="icon-button"
          type="button"
          aria-label="打开设置"
          @click="settingsOpen = true"
        >
          <SettingsIcon :size="18" aria-hidden="true" />
        </button>
      </div>
    </header>

    <section class="panel">
      <div class="control-stack">
        <PortToolbar
          :query="query"
          :is-refreshing="refreshing"
          :scan-status="scanStatus"
          :has-refresh-error="Boolean(refreshError)"
          @update:query="query = $event"
          @refresh="handleManualRefresh"
        />

        <div v-if="refreshError" class="refresh-warning" role="status">
          <AlertTriangleIcon :size="14" aria-hidden="true" />
          <span>自动刷新失败，当前显示上次成功扫描的数据。</span>
          <button type="button" @click="handleManualRefresh">重试</button>
        </div>
      </div>

      <div class="table-shell">
        <PortTable
          v-show="!initialLoading && !loadError"
          :items="sortedItems"
          :current-user="currentUser"
          :sort-key="sortKey"
          :sort-order="sortOrder"
          :active-kill-pids="activeKillPids"
          :is-dark-theme="isDarkTheme"
          :confirm-before-terminate="settings.confirmBeforeTerminate"
          :related-ports-by-pid="relatedPortsByPid"
          :has-active-query="Boolean(query.trim())"
          @kill="handleKill"
          @update:sort="updateSortByTable"
        />

        <div v-if="initialLoading" class="state-shell state-shell--overlay">
          <n-spin size="large" />
        </div>

        <div v-else-if="loadError" class="state-shell state-shell--overlay">
          <n-empty description="端口列表加载失败">
            <template #extra>
              <div class="error-message">{{ loadError }}</div>
              <n-button type="primary" tertiary @click="handleManualRefresh"
                >重新加载</n-button
              >
            </template>
          </n-empty>
        </div>
      </div>
    </section>

    <n-modal
      v-model:show="settingsOpen"
      preset="card"
      title="设置"
      class="settings-modal"
      :bordered="false"
      :style="{ width: '520px' }"
    >
      <div class="settings-modal-body">
        <section class="settings-field">
          <div class="settings-copy">
            <strong>当前用户</strong>
            <small>用于判断进程权限和可操作范围。</small>
          </div>
          <span class="settings-readonly">{{ currentUser || "未知" }}</span>
        </section>

        <section class="settings-field">
          <div class="settings-copy">
            <strong>自动刷新</strong>
            <small>关闭后仍可手动刷新端口列表。</small>
          </div>
          <n-switch v-model:value="settings.autoRefreshEnabled" size="small" />
        </section>

        <section class="settings-field settings-field--stacked">
          <div class="settings-copy">
            <strong>刷新间隔</strong>
            <small>当前为 {{ refreshIntervalLabel }}。</small>
          </div>
          <n-radio-group
            v-model:value="settings.refreshIntervalMs"
            name="refresh-interval"
            size="small"
            class="settings-radio-group"
          >
            <n-radio-button
              v-for="option in refreshIntervalOptions"
              :key="option.value"
              :value="option.value"
            >
              {{ option.label }}
            </n-radio-button>
          </n-radio-group>
        </section>

        <section class="settings-field">
          <div class="settings-copy">
            <strong>结束前确认</strong>
            <small>普通结束进程前弹出确认；强制结束始终确认。</small>
          </div>
          <n-switch
            v-model:value="settings.confirmBeforeTerminate"
            size="small"
          />
        </section>
      </div>
    </n-modal>
  </div>
</template>

<style scoped>
.workbench {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 10px;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--app-border);
  border-radius: 14px;
  background: var(--app-panel);
  box-shadow: var(--shadow-window);
}

.app-header {
  display: grid;
  grid-template-columns: minmax(220px, 1fr) auto;
  gap: 16px;
  align-items: center;
  min-height: 56px;
  padding: 10px 14px 8px;
  overflow: hidden;
  border-bottom: 1px solid var(--app-border);
  background: var(--app-chrome);
  backdrop-filter: blur(18px);
}

.header-brand {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.header-title {
  margin: 0;
  color: var(--app-text);
  font-size: 14px;
  font-weight: 650;
  line-height: 1.2;
  letter-spacing: 0;
}

.header-desc {
  margin: 0;
  color: var(--app-text-muted);
  font-size: 12px;
  line-height: 1.25;
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  justify-content: flex-end;
}

.icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  flex: 0 0 auto;
  padding: 0;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--app-text-muted);
  cursor: pointer;
  transition:
    border-color 160ms ease,
    background-color 160ms ease,
    color 160ms ease;
}

.icon-button:hover {
  border-color: var(--app-border);
  background: var(--app-panel-subtle);
  color: var(--app-text);
}

.icon-button:focus-visible {
  outline: 2px solid var(--app-accent);
  outline-offset: 2px;
}

.panel {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  min-height: 0;
  padding: 0 12px 12px;
  overflow: hidden;
  background: var(--app-panel);
}

.control-stack {
  min-width: 0;
}

.refresh-warning {
  display: flex;
  gap: 7px;
  align-items: center;
  min-height: 30px;
  margin-bottom: 8px;
  padding: 5px 9px;
  border: 1px solid color-mix(in srgb, var(--app-warning) 28%, transparent);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--app-warning) 9%, transparent);
  color: var(--app-warning);
  font-size: 12px;
}

.refresh-warning span {
  flex: 1;
}

.refresh-warning button {
  padding: 2px 6px;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-weight: 650;
}

.refresh-warning button:focus-visible {
  outline: 2px solid var(--app-warning);
  outline-offset: 2px;
}

.table-shell {
  position: relative;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--app-border);
  border-radius: var(--radius-lg);
  background: var(--app-panel);
}

.state-shell {
  display: grid;
  place-items: center;
  height: 100%;
  min-height: 0;
}

.state-shell--overlay {
  position: absolute;
  inset: 0;
  z-index: 4;
  background: var(--app-panel);
}

.error-message {
  max-width: 560px;
  margin-bottom: 12px;
  color: var(--app-danger);
  word-break: break-word;
}

:deep(.settings-modal) {
  max-width: calc(100vw - 48px);
  overflow: hidden;
  border: 1px solid var(--app-border);
  border-radius: 14px;
  background: var(--app-panel);
  box-shadow: var(--shadow-window);
}

:deep(.settings-modal .n-card-header) {
  padding: 18px 18px 14px;
  border-bottom: 1px solid var(--app-border);
  background: var(--app-chrome);
}

:deep(.settings-modal .n-card-header__main) {
  color: var(--app-text);
  font-size: 16px;
  font-weight: 650;
}

:deep(.settings-modal .n-card__content) {
  padding: 8px 18px 18px;
}

.settings-modal-body {
  display: grid;
}

.settings-field {
  display: flex;
  gap: 16px;
  align-items: center;
  justify-content: space-between;
  min-width: 0;
  padding: 14px 0;
  border-bottom: 1px solid var(--app-border);
}

.settings-field:last-child {
  border-bottom: 0;
}

.settings-field--stacked {
  align-items: stretch;
  flex-direction: column;
  gap: 10px;
}

.settings-copy {
  min-width: 0;
}

.settings-copy strong {
  display: block;
  color: var(--app-text);
  font-size: 13px;
  line-height: 1.35;
}

.settings-copy small {
  display: block;
  margin-top: 2px;
  color: var(--app-text-muted);
  font-size: 12px;
  line-height: 1.35;
}

.settings-radio-group {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  max-width: 100%;
}

.settings-readonly {
  display: inline-flex;
  align-items: center;
  min-height: 28px;
  padding: 0 10px;
  border: 1px solid var(--app-border);
  border-radius: var(--radius-md);
  background: var(--app-panel-subtle);
  color: var(--app-text-muted);
  font-family: var(--font-mono);
  font-size: 12px;
  white-space: nowrap;
}

@media (max-width: 720px) {
  .app-header {
    grid-template-columns: minmax(140px, 1fr) auto;
    gap: 10px 12px;
    padding: 10px 12px;
  }

  .header-title {
    font-size: 14px;
  }

  .panel {
    padding: 0 10px 10px;
  }
}
</style>
