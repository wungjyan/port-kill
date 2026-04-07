<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { NButton, NEmpty, NSpin, useMessage } from "naive-ui";

import IconMoon from "./IconMoon.vue";
import IconSun from "./IconSun.vue";
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

defineEmits<{
  toggleTheme: [];
}>();

const message = useMessage();
const { isDarkTheme } = defineProps<{
  isDarkTheme: boolean;
}>();

const AUTO_REFRESH_INTERVAL_MS = 8_000;
const LOAD_TIMEOUT_MS = 8_000; // Slightly longer than backend timeout (5s + buffer)
const REBOUND_CHECK_DELAY_MS = 900;

const initialLoading = ref(true);
const query = ref("");
const currentUser = ref("");
const items = ref<PortProcess[]>([]);
const sortKey = ref<SortKey>("recent");
const sortOrder = ref<SortOrder>("descend");
const activeKillPids = ref<number[]>([]);
const loadError = ref("");
const lastLoadTime = ref<number>(0);
let refreshTimer: number | null = null;
let currentLoadPromise: Promise<void> | null = null;
const reboundCheckTimers = new Set<number>();

const defaultOrders: Record<SortKey, SortOrder> = {
  recent: "descend",
  port: "ascend",
  process: "ascend",
};

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

function scheduleAutoRefresh() {
  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer);
  }

  refreshTimer = window.setTimeout(() => {
    void loadPorts({ silent: true });
  }, AUTO_REFRESH_INTERVAL_MS);
}

async function loadPorts(options: { silent?: boolean } = {}) {
  if (currentLoadPromise) {
    return currentLoadPromise;
  }

  loadError.value = "";
  const silent = options.silent ?? false;

  if (!silent || initialLoading.value) {
    initialLoading.value = true;
  }

  currentLoadPromise = (async () => {
    const startTime = performance.now();
    const timeoutId = setTimeout(() => {
      if (initialLoading.value && !silent) {
        message.warning("端口扫描耗时较长，请稍候...");
      }
    }, LOAD_TIMEOUT_MS);

    try {
      const response = await invoke<PortListResponse>("list_ports");
      currentUser.value = response.currentUser;
      items.value = response.items;
      clearTimeout(timeoutId);

      // Track load time for performance monitoring
      lastLoadTime.value = Math.round(performance.now() - startTime);
    } catch (error) {
      clearTimeout(timeoutId);
      loadError.value = String(error);

      // Show user-friendly error message
      if (!silent) {
        const errorMsg = String(error);
        if (errorMsg.includes("timeout")) {
          message.error("端口扫描超时，请重试");
        } else if (errorMsg.includes("lsof")) {
          message.error("无法执行端口扫描命令，请检查系统权限");
        }
      }
    } finally {
      initialLoading.value = false;
      currentLoadPromise = null;
      scheduleAutoRefresh();
    }
  })();

  return currentLoadPromise;
}

function applySort(nextKey: SortKey) {
  sortKey.value = nextKey;
  sortOrder.value = defaultOrders[nextKey];
}

function toggleSortOrder() {
  sortOrder.value = sortOrder.value === "ascend" ? "descend" : "ascend";
}

function updateSortByTable(nextSort: { key: SortKey; order: SortOrder }) {
  sortKey.value = nextSort.key;
  sortOrder.value = nextSort.order;
}

async function handleManualRefresh() {
  await loadPorts();
  message.success("刷新成功");
}

function buildKillSuccessMessage(item: PortProcess, force: boolean) {
  const actionText = force ? "已强制结束" : "已结束";
  const processLabel = item.processName ? `“${item.processName}”` : "目标进程";

  return `${actionText}${processLabel}（PID ${item.pid}），端口 ${item.port} 已释放`;
}

function scheduleReboundCheck(item: PortProcess) {
  const timerId = window.setTimeout(async () => {
    reboundCheckTimers.delete(timerId);

    try {
      const result = await invoke<ReboundCheckResult>("check_port_rebound", {
        port: item.port,
        previousPid: item.pid,
        previousProcessName: item.processName,
      });

      if (!result.rebound) {
        return;
      }

      message.warning(
        result.message ??
          `端口 ${item.port} 已被重新占用，请确认是否存在自动拉起的后台服务`,
      );
      await loadPorts({ silent: true });
    } catch {
      // Ignore rebound check failures to avoid interrupting the main kill flow.
    }
  }, REBOUND_CHECK_DELAY_MS);

  reboundCheckTimers.add(timerId);
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
  <div :class="['workbench', { 'workbench--light': !isDarkTheme }]">
    <header class="hero">
      <div class="hero-main">
        <h1 class="hero-title">端口占用管理工具</h1>
        <p class="hero-desc">
          扫描当前机器正在监听的端口与进程信息，支持按端口、进程和启动时间快速筛选，并可直接结束或强制结束目标进程。
        </p>
      </div>

      <button
        class="theme-toggle"
        type="button"
        :aria-label="
          isDarkTheme
            ? '当前为暗黑主题，点击切换到明亮主题'
            : '当前为明亮主题，点击切换到暗黑主题'
        "
        @click="$emit('toggleTheme')"
      >
        <IconMoon
          v-if="isDarkTheme"
          class="theme-toggle-icon"
          aria-hidden="true"
        />
        <IconSun v-else class="theme-toggle-icon" aria-hidden="true" />
      </button>
    </header>

    <section class="panel">
      <div class="panel-head">
        <div class="panel-meta">
          <span class="panel-title">Live Table</span>
          <span class="panel-subtitle">
            当前用户 {{ currentUser || "未知" }} · 共
            {{ filteredItems.length }} 条结果
            <template v-if="lastLoadTime > 0">
              · 加载耗时 {{ lastLoadTime }}ms
            </template>
          </span>
        </div>

        <PortToolbar
          :query="query"
          :sort-key="sortKey"
          :sort-order="sortOrder"
          @update:query="query = $event"
          @update:sort-key="applySort"
          @toggle:sort-order="toggleSortOrder"
          @refresh="handleManualRefresh"
        />
      </div>

      <div class="table-shell">
        <div v-if="initialLoading" class="state-shell">
          <n-spin size="large" />
        </div>

        <div v-else-if="loadError" class="state-shell">
          <n-empty description="端口列表加载失败">
            <template #extra>
              <div class="error-message">{{ loadError }}</div>
              <n-button type="primary" tertiary @click="handleManualRefresh"
                >重新加载</n-button
              >
            </template>
          </n-empty>
        </div>

        <div v-else-if="sortedItems.length === 0" class="state-shell">
          <n-empty description="没有匹配到端口监听进程" />
        </div>

        <PortTable
          v-else
          :items="sortedItems"
          :current-user="currentUser"
          :sort-key="sortKey"
          :sort-order="sortOrder"
          :active-kill-pids="activeKillPids"
          :is-dark-theme="isDarkTheme"
          @kill="handleKill"
          @update:sort="updateSortByTable"
        />
      </div>
    </section>
  </div>
</template>

<style scoped>
.workbench {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 14px;
  height: 100%;
  min-height: 0;
}

.hero {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  padding: 14px 18px;
  border: 1px solid rgba(82, 112, 146, 0.18);
  border-radius: 20px;
  background:
    radial-gradient(
      circle at top left,
      rgba(63, 115, 255, 0.22),
      transparent 34%
    ),
    radial-gradient(
      circle at bottom right,
      rgba(42, 186, 132, 0.16),
      transparent 28%
    ),
    linear-gradient(180deg, rgba(10, 20, 34, 0.96), rgba(7, 15, 27, 0.98));
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.2);
}

.hero-main {
  display: grid;
  gap: 6px;
}

.hero-title {
  margin: 0;
  color: #f6fbff;
  font-size: clamp(22px, 3vw, 30px);
  line-height: 1.05;
  letter-spacing: -0.05em;
}

.hero-desc {
  max-width: 760px;
  margin: 0;
  color: #88a0bd;
  font-size: 13px;
  line-height: 1.55;
}

.theme-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 42px;
  height: 42px;
  flex: 0 0 auto;
  padding: 0;
  border: 1px solid rgba(108, 139, 176, 0.2);
  border-radius: 14px;
  background: rgba(10, 22, 38, 0.62);
  color: #f3f7ff;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
  cursor: pointer;
  transition:
    transform 160ms ease,
    border-color 160ms ease,
    background-color 160ms ease,
    color 160ms ease;
}

.theme-toggle:hover {
  border-color: rgba(123, 156, 198, 0.32);
  background: rgba(14, 29, 48, 0.76);
  transform: translateY(-1px);
}

.theme-toggle:focus-visible {
  outline: 2px solid rgba(75, 127, 255, 0.5);
  outline-offset: 2px;
}

.theme-toggle-icon {
  width: 18px;
  height: 18px;
}

.panel {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  gap: 10px;
  min-height: 0;
  padding: 12px;
  overflow: hidden;
  border: 1px solid rgba(82, 112, 146, 0.14);
  border-radius: 20px;
  background: rgba(6, 13, 24, 0.84);
  backdrop-filter: blur(18px);
}

.panel-head {
  display: grid;
  gap: 10px;
}

.panel-meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: flex-end;
}

.panel-title {
  color: #f1f6ff;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.panel-subtitle {
  color: #6d86a4;
  font-size: 11px;
}

.table-shell {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.state-shell {
  display: grid;
  place-items: center;
  height: 100%;
  min-height: 0;
}

.error-message {
  max-width: 560px;
  margin-bottom: 12px;
  color: #f0bdc0;
  word-break: break-word;
}

.workbench--light .hero {
  border-color: rgba(117, 145, 179, 0.22);
  background:
    radial-gradient(
      circle at top left,
      rgba(72, 120, 235, 0.16),
      transparent 34%
    ),
    radial-gradient(
      circle at bottom right,
      rgba(44, 179, 130, 0.12),
      transparent 28%
    ),
    linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.96),
      rgba(243, 248, 254, 0.98)
    );
  box-shadow: 0 18px 34px rgba(69, 94, 123, 0.08);
}

.workbench--light .hero-title {
  color: #12263c;
}

.workbench--light .hero-desc {
  color: #5a7088;
}

.workbench--light .theme-toggle {
  border-color: rgba(115, 143, 176, 0.2);
  background: rgba(255, 255, 255, 0.78);
  color: #f2a125;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.8);
}

.workbench--light .theme-toggle:hover {
  border-color: rgba(86, 122, 176, 0.34);
  background: rgba(248, 251, 255, 0.98);
}

.workbench--light .panel {
  border-color: rgba(117, 145, 179, 0.16);
  background: rgba(255, 255, 255, 0.72);
}

.workbench--light .panel-title {
  color: #14273d;
}

.workbench--light .panel-subtitle {
  color: #698098;
}

.workbench--light .error-message {
  color: #c55562;
}

@media (max-width: 720px) {
  .hero,
  .panel {
    padding: 10px 12px;
    border-radius: 16px;
  }

  .panel-meta {
    flex-direction: column;
    align-items: flex-start;
  }

  .hero {
    gap: 12px;
  }
}
</style>
