<script setup lang="ts">
import { ref, watch } from "vue";
import { NButton, NDropdown, NModal, NPopconfirm } from "naive-ui";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  ChevronRightIcon,
  ChevronsUpDownIcon,
  MoreHorizontalIcon,
} from "@lucide/vue";

import {
  formatAddressSummary,
  formatIpSummary,
  formatStartedAtShort,
  getRowKey,
  resolveExposure,
  resolvePortTags,
} from "../portMeta.ts";
import type { PortProcess, SortKey, SortOrder } from "../types.ts";
import PortDetailPanel from "./PortDetailPanel.vue";

const props = defineProps<{
  items: PortProcess[];
  currentUser: string;
  sortKey: SortKey;
  sortOrder: SortOrder;
  activeKillPids: number[];
  isDarkTheme: boolean;
  confirmBeforeTerminate: boolean;
  relatedPortsByPid: Record<number, number[]>;
  hasActiveQuery: boolean;
}>();

const emit = defineEmits<{
  kill: [item: PortProcess, force: boolean];
  "update:sort": [value: { key: SortKey; order: SortOrder }];
}>();

const expandedRowKeys = ref<string[]>([]);
const forceTarget = ref<PortProcess | null>(null);
const forceMenuOptions = [{ label: "强制结束", key: "force" }];

const defaultOrders: Record<SortKey, SortOrder> = {
  recent: "descend",
  port: "ascend",
  process: "ascend",
};

watch(
  () => props.items,
  (items) => {
    const validKeys = new Set(items.map((item) => getRowKey(item)));
    expandedRowKeys.value = expandedRowKeys.value.filter((key) =>
      validKeys.has(key),
    );
  },
  { immediate: true },
);

function isExpanded(item: PortProcess) {
  return expandedRowKeys.value.includes(getRowKey(item));
}

function toggleExpand(item: PortProcess) {
  const key = getRowKey(item);
  expandedRowKeys.value = isExpanded(item)
    ? expandedRowKeys.value.filter((itemKey) => itemKey !== key)
    : [...expandedRowKeys.value, key];
}

function updateSort(nextKey: SortKey) {
  if (props.sortKey !== nextKey) {
    emit("update:sort", {
      key: nextKey,
      order: defaultOrders[nextKey],
    });
    return;
  }

  emit("update:sort", {
    key: nextKey,
    order: props.sortOrder === "ascend" ? "descend" : "ascend",
  });
}

function sortIconFor(key: SortKey) {
  if (props.sortKey !== key) {
    return ChevronsUpDownIcon;
  }

  return props.sortOrder === "ascend" ? ArrowUpIcon : ArrowDownIcon;
}

function ariaSort(key: SortKey): "ascending" | "descending" | "none" {
  if (props.sortKey !== key) {
    return "none";
  }

  return props.sortOrder === "ascend" ? "ascending" : "descending";
}

function formatProcessMeta(row: PortProcess) {
  if (row.cwd) {
    const normalized = row.cwd.replace(/\\/g, "/");
    const homeDirectory = props.currentUser
      ? `/Users/${props.currentUser}`
      : "";

    if (homeDirectory && normalized === homeDirectory) {
      return "~";
    }

    if (homeDirectory && normalized.startsWith(`${homeDirectory}/`)) {
      return `~/${normalized.slice(homeDirectory.length + 1)}`;
    }

    const parts = normalized.split("/").filter(Boolean);
    return parts.length > 0 ? `…/${parts.slice(-2).join("/")}` : row.cwd;
  }

  return row.command;
}

function canTerminate(item: PortProcess) {
  return Boolean(props.currentUser) && item.user === props.currentUser;
}

function relatedPorts(item: PortProcess) {
  return props.relatedPortsByPid[item.pid] ?? [item.port];
}

function terminateConfirmation(item: PortProcess) {
  const ports = relatedPorts(item);
  if (ports.length === 1) {
    return `确认结束 PID ${item.pid}？`;
  }

  return `确认结束 PID ${item.pid}？该进程同时监听端口 ${ports.join("、")}。`;
}

function openForceConfirm(row: PortProcess) {
  if (!canTerminate(row)) {
    return;
  }
  forceTarget.value = row;
}

function confirmForceKill() {
  if (!forceTarget.value) {
    return;
  }

  emit("kill", forceTarget.value, true);
  forceTarget.value = null;
}

function requestTerminate(item: PortProcess) {
  if (!canTerminate(item)) {
    return;
  }
  emit("kill", item, false);
}
</script>

<template>
  <div :class="['table-root', { 'table-root--light': !isDarkTheme }]">
    <div class="table-scroll">
      <table class="port-table">
        <colgroup>
          <col class="col-expand" />
          <col class="col-port" />
          <col class="col-process" />
          <col class="col-pid" />
          <col class="col-address" />
          <col class="col-started" />
          <col class="col-actions" />
        </colgroup>
        <thead>
          <tr>
            <th class="col-expand" aria-label="展开详情"></th>
            <th class="col-port" :aria-sort="ariaSort('port')">
              <button
                class="head-button"
                type="button"
                aria-label="按端口排序"
                @click="updateSort('port')"
              >
                <span>端口</span>
                <span
                  :class="[
                    'sort-indicator',
                    { 'sort-indicator--active': sortKey === 'port' },
                  ]"
                >
                  <component
                    :is="sortIconFor('port')"
                    :size="12"
                    aria-hidden="true"
                  />
                </span>
              </button>
            </th>
            <th class="col-process" :aria-sort="ariaSort('process')">
              <button
                class="head-button"
                type="button"
                aria-label="按进程名排序"
                @click="updateSort('process')"
              >
                <span>进程</span>
                <span
                  :class="[
                    'sort-indicator',
                    { 'sort-indicator--active': sortKey === 'process' },
                  ]"
                >
                  <component
                    :is="sortIconFor('process')"
                    :size="12"
                    aria-hidden="true"
                  />
                </span>
              </button>
            </th>
            <th class="col-pid">PID</th>
            <th class="col-address">地址</th>
            <th class="col-started" :aria-sort="ariaSort('recent')">
              <button
                class="head-button"
                type="button"
                aria-label="按启动时间排序"
                @click="updateSort('recent')"
              >
                <span>启动时间</span>
                <span
                  :class="[
                    'sort-indicator',
                    { 'sort-indicator--active': sortKey === 'recent' },
                  ]"
                >
                  <component
                    :is="sortIconFor('recent')"
                    :size="12"
                    aria-hidden="true"
                  />
                </span>
              </button>
            </th>
            <th class="col-actions">操作</th>
          </tr>
        </thead>

        <tbody>
          <template v-if="items.length > 0">
            <template v-for="item in items" :key="getRowKey(item)">
              <tr class="data-row">
                <td class="col-expand">
                  <button
                    class="expand-button"
                    type="button"
                    :aria-label="isExpanded(item) ? '收起详情' : '展开详情'"
                    :aria-expanded="isExpanded(item)"
                    @click="toggleExpand(item)"
                  >
                    <ChevronRightIcon :size="14" aria-hidden="true" />
                  </button>
                </td>
                <td class="col-port">
                  <span class="port-value">{{ item.port }}</span>
                </td>
                <td class="col-process">
                  <div class="process-cell">
                    <div class="process-line">
                      <span class="process-name" :title="item.command">
                        {{ item.processName }}
                      </span>
                      <span
                        v-if="currentUser && item.user !== currentUser"
                        class="process-tag process-tag--warning"
                      >
                        {{ item.user }}
                      </span>
                      <span
                        v-for="tag in resolvePortTags(item).slice(0, 2)"
                        :key="`${getRowKey(item)}:${tag.label}`"
                        :class="['process-tag', `process-tag--${tag.tone}`]"
                      >
                        {{ tag.label }}
                      </span>
                    </div>
                    <span
                      class="process-meta"
                      :title="item.cwd ?? item.command"
                    >
                      {{ formatProcessMeta(item) }}
                    </span>
                  </div>
                </td>
                <td class="col-pid">
                  <span class="mono-muted">{{ item.pid }}</span>
                </td>
                <td class="col-address">
                  <div class="address-cell" :title="item.hostSummary">
                    <span class="address-value">{{ formatAddressSummary(item) }}</span>
                    <span
                      :class="[
                        'address-meta',
                        `address-meta--${resolveExposure(item).tone}`,
                      ]"
                    >
                      {{ formatIpSummary(item) }} · {{ resolveExposure(item).label }}
                    </span>
                  </div>
                </td>
                <td class="col-started">
                  <span class="mono-muted" :title="item.startedAt ?? '未知'">
                    {{ formatStartedAtShort(item.startedAt) }}
                  </span>
                </td>
                <td class="col-actions">
                  <div class="row-actions">
                    <n-popconfirm
                      v-if="confirmBeforeTerminate"
                      positive-text="结束"
                      negative-text="取消"
                      @positive-click="requestTerminate(item)"
                    >
                      <template #trigger>
                        <n-button
                          size="small"
                          secondary
                          type="warning"
                          class="action-button"
                          :loading="activeKillPids.includes(item.pid)"
                          :disabled="
                            activeKillPids.includes(item.pid) || !canTerminate(item)
                          "
                          :title="
                            canTerminate(item)
                              ? '向进程发送 TERM 信号'
                              : `进程属于用户 ${item.user}，当前不可操作`
                          "
                          @click.stop
                        >
                          结束
                        </n-button>
                      </template>
                      {{ terminateConfirmation(item) }}
                    </n-popconfirm>

                    <n-button
                      v-else
                      size="small"
                      secondary
                      type="warning"
                      class="action-button"
                      :loading="activeKillPids.includes(item.pid)"
                      :disabled="
                        activeKillPids.includes(item.pid) || !canTerminate(item)
                      "
                      :title="
                        canTerminate(item)
                          ? '向进程发送 TERM 信号'
                          : `进程属于用户 ${item.user}，当前不可操作`
                      "
                      @click.stop="requestTerminate(item)"
                    >
                      结束
                    </n-button>

                    <n-dropdown
                      :options="forceMenuOptions"
                      trigger="click"
                      @select="openForceConfirm(item)"
                    >
                      <n-button
                        size="small"
                        quaternary
                        circle
                        class="more-button"
                        :aria-label="`打开 PID ${item.pid} 的更多操作`"
                        :disabled="
                          activeKillPids.includes(item.pid) || !canTerminate(item)
                        "
                        @click.stop
                      >
                        <template #icon>
                          <MoreHorizontalIcon :size="15" aria-hidden="true" />
                        </template>
                      </n-button>
                    </n-dropdown>
                  </div>
                </td>
              </tr>

              <tr v-if="isExpanded(item)" class="detail-row">
                <td colspan="7">
                  <PortDetailPanel :item="item" />
                </td>
              </tr>
            </template>
          </template>

          <tr v-else class="empty-row">
            <td colspan="7">
              {{ hasActiveQuery ? "没有匹配到端口监听进程" : "当前没有 TCP 监听端口" }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <n-modal
      :show="Boolean(forceTarget)"
      preset="dialog"
      type="error"
      title="强制结束进程"
      positive-text="强制结束"
      negative-text="取消"
      @positive-click="confirmForceKill"
      @negative-click="forceTarget = null"
      @mask-click="forceTarget = null"
      @esc="forceTarget = null"
    >
      <p class="force-copy">
        PID {{ forceTarget?.pid }} · {{ forceTarget?.processName }} 将收到 KILL
        信号。该进程监听的端口
        {{ forceTarget ? relatedPorts(forceTarget).join("、") : "" }} 都会受到影响。
      </p>
    </n-modal>
  </div>
</template>

<style scoped>
.table-root {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.table-scroll {
  height: 100%;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  scrollbar-color: rgba(128, 128, 128, 0.34) transparent;
  scrollbar-width: thin;
}

.table-scroll::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.table-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.table-scroll::-webkit-scrollbar-thumb {
  border: 2px solid transparent;
  border-radius: 999px;
  background-clip: content-box;
  background-color: rgba(128, 128, 128, 0.28);
}

.table-scroll:hover::-webkit-scrollbar-thumb {
  background-color: rgba(128, 128, 128, 0.46);
}

.table-scroll::-webkit-scrollbar-corner {
  background: transparent;
}

.port-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  color: var(--app-text);
  table-layout: fixed;
}

.port-table th,
.port-table td {
  border-bottom: 1px solid var(--app-border);
  vertical-align: middle;
}

.port-table th {
  position: sticky;
  top: 0;
  z-index: 2;
  height: 32px;
  padding: 0 8px;
  background: var(--app-panel-subtle);
  color: var(--app-text-faint);
  font-size: 11px;
  font-weight: 600;
  text-align: left;
}

.port-table td {
  height: 48px;
  padding: 7px 8px;
  background: var(--app-panel);
}

.data-row:hover td {
  background: var(--app-row-hover);
}

.head-button {
  display: inline-flex;
  gap: 4px;
  align-items: center;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-size: inherit;
  font-weight: inherit;
}

.head-button:hover {
  color: var(--app-text);
}

.head-button:focus-visible {
  border-radius: var(--radius-xs);
  outline: 2px solid var(--app-accent);
  outline-offset: 2px;
}

.sort-indicator {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
  color: var(--app-text-faint);
}

.head-button:hover .sort-indicator {
  color: var(--app-text-muted);
}

.sort-indicator--active {
  color: var(--app-accent);
}

.head-button:hover .sort-indicator--active {
  color: var(--app-accent);
}

.col-expand {
  width: 34px;
  text-align: center;
}

.col-port {
  width: 100px;
}

.col-process {
  width: 300px;
}

.col-pid {
  width: 76px;
}

.col-address {
  width: 130px;
}

.col-started {
  width: 112px;
}

.col-actions {
  width: 100px;
  text-align: left;
}

.expand-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--app-text-faint);
  cursor: pointer;
  transition:
    background-color 140ms ease,
    color 140ms ease,
    transform 140ms ease;
}

.expand-button:hover {
  background: var(--app-accent-soft);
  color: var(--app-text);
}

.expand-button[aria-expanded="true"] {
  transform: rotate(90deg);
}

.port-value {
  color: var(--app-text);
  font-family: var(--font-mono);
  font-size: 15px;
  font-weight: 700;
}

.process-cell {
  display: grid;
  gap: 3px;
  min-width: 0;
}

.process-line {
  display: flex;
  gap: 6px;
  align-items: center;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
}

.process-name {
  display: block;
  min-width: 0;
  overflow: hidden;
  color: var(--app-text);
  text-overflow: ellipsis;
  font-size: 13px;
  font-weight: 650;
}

.process-meta {
  display: block;
  min-width: 0;
  overflow: hidden;
  color: var(--app-text-faint);
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.process-tag {
  display: inline-flex;
  align-items: center;
  height: 18px;
  flex: 0 0 auto;
  padding: 0 6px;
  border-radius: 999px;
  background: var(--app-accent-soft);
  color: var(--app-accent);
  font-size: 10px;
  font-weight: 650;
}

.process-tag--service {
  background: rgba(48, 209, 88, 0.12);
  color: var(--app-success);
}

.process-tag--infra,
.process-tag--warning {
  background: rgba(255, 159, 10, 0.14);
  color: var(--app-warning);
}

.address-cell {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.address-value {
  overflow: hidden;
  color: var(--app-text-muted);
  font-family: var(--font-mono);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.address-meta,
.mono-muted {
  color: var(--app-text-muted);
  font-family: var(--font-mono);
  font-size: 12px;
}

.address-meta {
  color: var(--app-text-faint);
  font-size: 11px;
}

.address-meta--local {
  color: var(--app-success);
}

.address-meta--public {
  color: var(--app-warning);
}

.row-actions {
  display: inline-flex;
  gap: 4px;
  align-items: center;
  justify-content: flex-end;
}

.action-button {
  min-width: 42px;
  border-radius: var(--radius-sm);
}

.more-button {
  color: var(--app-text-faint);
}

.detail-row td {
  height: auto;
  padding: 0;
  background: var(--app-panel-subtle);
}

.empty-row td {
  height: 180px;
  color: var(--app-text-faint);
  text-align: center;
}

.force-copy {
  margin: 0;
  color: var(--app-text-muted);
  font-size: 13px;
  line-height: 1.5;
}
</style>
