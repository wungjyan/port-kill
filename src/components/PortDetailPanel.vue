<script setup lang="ts">
import { computed } from "vue";

import { formatStartedAtFull } from "../portMeta";
import type { PortProcess } from "../types";

const props = defineProps<{
  item: PortProcess;
  isDarkTheme: boolean;
}>();

const fullAddress = computed(() => {
  if (props.item.hosts.length === 0) {
    return "-";
  }

  return props.item.hosts.join(" / ");
});
</script>

<template>
  <div :class="['detail-panel', { 'detail-panel--light': !isDarkTheme }]">
    <div class="detail-item detail-item--wide">
      <span class="detail-label">命令</span>
      <code class="detail-code">{{ item.command }}</code>
    </div>

    <div class="detail-item">
      <span class="detail-label">工作目录</span>
      <span class="detail-value">{{ item.cwd ?? "不可用" }}</span>
    </div>

    <div class="detail-item">
      <span class="detail-label">监听地址</span>
      <span class="detail-value">{{ fullAddress }}</span>
    </div>

    <div class="detail-item">
      <span class="detail-label">启动时间</span>
      <span class="detail-value">{{ formatStartedAtFull(item.startedAt) }}</span>
    </div>
  </div>
</template>

<style scoped>
.detail-panel {
  display: grid;
  grid-template-columns: 108px minmax(0, 1fr);
  gap: 8px 14px;
  padding: 2px 8px 4px 0;
}

.detail-item {
  display: contents;
}

.detail-item--wide {
  grid-column: auto;
}

.detail-label {
  align-self: start;
  color: #6b809a;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  line-height: 1.7;
  text-transform: uppercase;
}

.detail-value,
.detail-code {
  min-width: 0;
  color: #d9e8fa;
  font-family:
    "SF Mono",
    "JetBrains Mono",
    "IBM Plex Mono",
    monospace;
  font-size: 12px;
  line-height: 1.7;
  word-break: break-all;
}

.detail-code {
  padding: 0;
  border: 0;
  background: transparent;
}

.detail-panel--light .detail-label {
  color: #70839a;
}

.detail-panel--light .detail-value,
.detail-panel--light .detail-code {
  color: #24384f;
}

@media (max-width: 900px) {
  .detail-panel {
    grid-template-columns: 92px minmax(0, 1fr);
  }
}
</style>
