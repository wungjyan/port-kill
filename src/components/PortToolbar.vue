<script setup lang="ts">
import { computed } from "vue";
import { NButton, NButtonGroup, NInput, NSpace } from "naive-ui";

import type { SortKey, SortOrder } from "../types";

const props = defineProps<{
  query: string;
  sortKey: SortKey;
  sortOrder: SortOrder;
}>();

const emit = defineEmits<{
  "update:query": [value: string];
  "update:sortKey": [value: SortKey];
  "toggle:sortOrder": [];
  refresh: [];
}>();

const sortOptions = [
  { label: "最近启动", value: "recent" },
  { label: "端口", value: "port" },
  { label: "进程", value: "process" },
] satisfies Array<{ label: string; value: SortKey }>;

const orderLabel = computed(() => {
  if (props.sortKey === "recent") {
    return props.sortOrder === "descend" ? "新到旧" : "旧到新";
  }

  return props.sortOrder === "ascend" ? "升序" : "降序";
});
</script>

<template>
  <div class="toolbar">
    <n-input
      :value="query"
      class="toolbar-search"
      clearable
      placeholder="搜索端口 / 进程 / PID"
      @update:value="emit('update:query', $event)"
    />

    <n-space class="toolbar-actions" :size="10" align="center">
      <n-button-group class="toolbar-segment">
        <n-button
          v-for="option in sortOptions"
          :key="option.value"
          size="small"
          :type="sortKey === option.value ? 'primary' : 'default'"
          :secondary="sortKey === option.value"
          @click="emit('update:sortKey', option.value)"
        >
          {{ option.label }}
        </n-button>
      </n-button-group>

      <n-button
        quaternary
        size="small"
        class="toolbar-order"
        @click="emit('toggle:sortOrder')"
      >
        {{ orderLabel }}
      </n-button>

      <n-button
        tertiary
        size="small"
        type="primary"
        class="toolbar-refresh"
        @click="emit('refresh')"
      >
        刷新
      </n-button>
    </n-space>
  </div>
</template>

<style scoped>
.toolbar {
  display: grid;
  grid-template-columns: minmax(240px, 1fr) auto;
  gap: 12px;
  align-items: center;
}

.toolbar-search :deep(.n-input) {
  border-radius: 14px;
}

.toolbar-actions {
  justify-content: flex-end;
}

.toolbar-segment {
  border-radius: 12px;
}

.toolbar-order {
  min-width: 96px;
}

.toolbar-refresh {
  min-width: 80px;
}

@media (max-width: 900px) {
  .toolbar {
    grid-template-columns: 1fr;
  }

  .toolbar-actions {
    justify-content: flex-start;
    flex-wrap: wrap;
  }
}
</style>
