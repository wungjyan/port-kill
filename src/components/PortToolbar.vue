<script setup lang="ts">
import { computed } from "vue";
import { NButton, NInput } from "naive-ui";
import { RefreshCwIcon, SearchIcon } from "@lucide/vue";

const props = defineProps<{
  query: string;
  isRefreshing: boolean;
  scanStatus: string;
  hasRefreshError: boolean;
}>();

const emit = defineEmits<{
  "update:query": [value: string];
  refresh: [];
}>();

const statusLabel = computed(() => {
  if (props.isRefreshing) {
    return "扫描中";
  }

  if (props.hasRefreshError) {
    return "刷新失败";
  }

  return props.scanStatus === "等待中" ? "等待扫描" : "已更新";
});

const showStatusDetail = computed(
  () =>
    !props.isRefreshing &&
    !props.hasRefreshError &&
    props.scanStatus !== "等待中",
);
</script>

<template>
  <div class="toolbar">
    <n-input
      :value="query"
      class="toolbar-search"
      clearable
      placeholder="搜索端口 / 进程 / PID"
      @update:value="emit('update:query', $event)"
    >
      <template #prefix>
        <SearchIcon :size="15" aria-hidden="true" />
      </template>
    </n-input>

    <div class="toolbar-meta">
      <div
        :class="[
          'toolbar-status',
          {
            'toolbar-status--loading': isRefreshing,
            'toolbar-status--error': hasRefreshError,
          },
        ]"
        role="status"
        aria-live="polite"
        title="上次扫描时间与耗时"
      >
        <span class="status-dot" aria-hidden="true" />
        <span class="status-label">{{ statusLabel }}</span>
        <span v-if="showStatusDetail" class="status-detail">
          {{ scanStatus }}
        </span>
      </div>

      <div class="toolbar-actions">
        <n-button
          secondary
          size="small"
          type="primary"
          class="toolbar-refresh"
          :loading="isRefreshing"
          @click="emit('refresh')"
        >
          <template #icon>
            <RefreshCwIcon :size="14" aria-hidden="true" />
          </template>
          刷新
        </n-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 16px;
  align-items: center;
  padding: 10px 0;
}

.toolbar-meta {
  display: flex;
  gap: 14px;
  align-items: center;
  justify-content: flex-end;
  min-width: 0;
}

.toolbar-status {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  min-width: 0;
  color: var(--app-text-muted);
  font-size: 11px;
  line-height: 1;
  white-space: nowrap;
}

.status-dot {
  width: 6px;
  height: 6px;
  flex: 0 0 auto;
  border-radius: 999px;
  background: var(--app-success);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--app-success) 12%, transparent);
}

.status-label {
  color: var(--app-text-faint);
}

.status-detail {
  overflow: hidden;
  color: var(--app-text-muted);
  font-family: var(--font-mono);
  text-overflow: ellipsis;
}

.toolbar-status--loading .status-dot {
  background: var(--app-accent);
  box-shadow: 0 0 0 3px var(--app-accent-soft);
  animation: status-pulse 1.1s ease-in-out infinite;
}

.toolbar-status--error .status-dot {
  background: var(--app-danger);
  box-shadow: 0 0 0 3px var(--app-danger-soft);
}

.toolbar-status--error .status-label {
  color: var(--app-danger);
}

@keyframes status-pulse {
  50% {
    opacity: 0.4;
  }
}

.toolbar-search {
  min-width: 0;
  border-radius: var(--radius-md);
}

.toolbar-search :deep(.n-input__prefix) {
  color: var(--app-text-faint);
}

.toolbar-actions {
  display: flex;
  gap: 6px;
  align-items: center;
  justify-content: flex-end;
}

.toolbar-refresh {
  min-width: 74px;
  border-radius: var(--radius-md);
}

@media (max-width: 820px) {
  .toolbar {
    grid-template-columns: 1fr;
    gap: 8px;
  }

  .toolbar-meta {
    justify-content: space-between;
  }
}

@media (prefers-reduced-motion: reduce) {
  .toolbar-status--loading .status-dot {
    animation: none;
  }
}
</style>
