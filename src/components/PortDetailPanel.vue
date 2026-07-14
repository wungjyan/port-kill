<script setup lang="ts">
import { computed } from "vue";

import { formatStartedAtFull } from "../portMeta";
import type { PortProcess } from "../types";

const props = defineProps<{
  item: PortProcess;
}>();

const fullAddress = computed(() => {
  if (props.item.hosts.length === 0) {
    return "-";
  }

  return props.item.hosts.join(" / ");
});
</script>

<template>
  <div class="detail-panel">
    <div class="detail-section detail-section--command">
      <span class="detail-label">命令</span>
      <code class="detail-code">{{ item.command }}</code>
    </div>

    <div class="detail-section">
      <span class="detail-label">工作目录</span>
      <span class="detail-value">{{ item.cwd ?? "不可用" }}</span>
    </div>

    <div class="detail-section">
      <span class="detail-label">监听地址</span>
      <span class="detail-value">{{ fullAddress }}</span>
    </div>

    <div class="detail-section">
      <span class="detail-label">用户</span>
      <span class="detail-value">{{ item.user }}</span>
    </div>

    <div class="detail-section">
      <span class="detail-label">协议 / 状态</span>
      <span class="detail-value">{{ item.protocol }} · {{ item.state }}</span>
    </div>

    <div class="detail-section">
      <span class="detail-label">启动时间</span>
      <span class="detail-value">{{ formatStartedAtFull(item.startedAt) }}</span>
    </div>
  </div>
</template>

<style scoped>
.detail-panel {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 18px;
  padding: 12px 12px 12px 42px;
  border-top: 1px solid var(--app-border);
  background: var(--app-panel-subtle);
}

.detail-section {
  display: grid;
  gap: 4px;
  min-width: 0;
}

.detail-section--command {
  grid-column: 1 / -1;
}

.detail-label {
  color: var(--app-text-faint);
  font-size: 11px;
  font-weight: 600;
  line-height: 1.2;
}

.detail-value,
.detail-code {
  min-width: 0;
  color: var(--app-text);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.55;
  word-break: break-all;
}

.detail-code {
  padding: 8px 10px;
  border: 1px solid var(--app-border);
  border-radius: var(--radius-md);
  background: var(--app-panel);
}

@media (max-width: 900px) {
  .detail-panel {
    grid-template-columns: 1fr;
    padding-left: 12px;
  }
}
</style>
