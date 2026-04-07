<script setup lang="ts">
import {
  computed,
  h,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import {
  NButton,
  NDataTable,
  NPopconfirm,
  NSpace,
  NTag,
  type DataTableColumns,
  type DataTableRowKey,
  type DataTableSortState,
} from "naive-ui";

import {
  formatAddressSummary,
  formatIpSummary,
  formatStartedAtShort,
  getRowKey,
  resolvePortTags,
  tagTypeForTone,
} from "../portMeta";
import type { PortProcess, SortKey, SortOrder } from "../types";
import PortDetailPanel from "./PortDetailPanel.vue";

const props = defineProps<{
  items: PortProcess[];
  currentUser: string;
  sortKey: SortKey;
  sortOrder: SortOrder;
  activeKillPids: number[];
  isDarkTheme: boolean;
}>();

const emit = defineEmits<{
  kill: [item: PortProcess, force: boolean];
  "update:sort": [value: { key: SortKey; order: SortOrder }];
}>();

const expandedRowKeys = ref<DataTableRowKey[]>([]);
const tableRootRef = ref<HTMLElement | null>(null);
const tableMaxHeight = ref(320);

let resizeObserver: ResizeObserver | null = null;

watch(
  () => props.items,
  (items) => {
    const validKeys = new Set(items.map((item) => getRowKey(item)));
    expandedRowKeys.value = expandedRowKeys.value.filter((key) =>
      validKeys.has(String(key)),
    );
  },
  { immediate: true },
);

function onSorterUpdate(
  sorter: DataTableSortState | DataTableSortState[] | null,
) {
  const sortState = Array.isArray(sorter) ? sorter[0] : sorter;
  if (!sortState?.columnKey) {
    emit("update:sort", { key: "recent", order: "descend" });
    return;
  }

  const keyMap: Record<string, SortKey> = {
    port: "port",
    process: "process",
    uptime: "recent",
  };

  const nextKey = keyMap[String(sortState.columnKey)] ?? "recent";
  const nextOrder = sortState.order === "ascend" ? "ascend" : "descend";

  emit("update:sort", { key: nextKey, order: nextOrder });
}

function updateTableMaxHeight() {
  const nextHeight = tableRootRef.value?.clientHeight ?? 0;
  tableMaxHeight.value = nextHeight > 0 ? nextHeight : 320;
}

onMounted(async () => {
  await nextTick();
  updateTableMaxHeight();

  if (!tableRootRef.value) {
    return;
  }

  resizeObserver = new ResizeObserver(() => {
    updateTableMaxHeight();
  });

  resizeObserver.observe(tableRootRef.value);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});

// Column renderers stay in one computed block so sort indicators and handlers remain aligned.
const columns = computed<DataTableColumns<PortProcess>>(() => [
  {
    type: "expand",
    width: 52,
    expandable: () => true,
    renderExpand: (row) =>
      h(PortDetailPanel, { item: row, isDarkTheme: props.isDarkTheme }),
  },
  {
    title: "端口",
    key: "port",
    width: 90,
    sorter: true,
    sortOrder: props.sortKey === "port" ? props.sortOrder : false,
    render: (row) =>
      h("div", { class: "port-cell" }, [
        h("span", { class: "port-value" }, String(row.port)),
      ]),
  },
  {
    title: "进程",
    key: "process",
    width: 220,
    sorter: true,
    sortOrder: props.sortKey === "process" ? props.sortOrder : false,
    render: (row) => {
      const tags = resolvePortTags(row);
      const children = [
        h(
          "span",
          { class: "process-name", title: row.command },
          row.processName,
        ),
      ];

      if (props.currentUser && row.user !== props.currentUser) {
        children.push(
          h(
            NTag,
            {
              size: "tiny",
              bordered: false,
              type: "warning",
              class: "process-tag",
            },
            { default: () => row.user },
          ),
        );
      }

      tags.slice(0, 2).forEach((tag) => {
        children.push(
          h(
            NTag,
            {
              size: "tiny",
              bordered: false,
              type: tagTypeForTone(tag.tone),
              class: "process-tag",
            },
            { default: () => tag.label },
          ),
        );
      });

      return h("div", { class: "process-cell" }, children);
    },
  },
  {
    title: "PID",
    key: "pid",
    width: 100,
    render: (row) =>
      h("div", { class: "pid-cell" }, [
        h("span", { class: "pid-value" }, String(row.pid)),
      ]),
  },
  {
    title: "地址",
    key: "address",
    width: 200,
    ellipsis: true,
    render: (row) =>
      h("div", { class: "address-cell", title: row.hostSummary }, [
        h("span", { class: "address-value" }, formatAddressSummary(row)),
        h("span", { class: "address-meta" }, `(${formatIpSummary(row)})`),
      ]),
  },
  {
    title: "启动时间",
    key: "uptime",
    width: 140,
    sorter: true,
    sortOrder: props.sortKey === "recent" ? props.sortOrder : false,
    render: (row) =>
      h("div", { class: "uptime-wrap" }, [
        h(
          "span",
          {
            class: "uptime-cell",
            title: row.startedAt ?? "未知",
          },
          formatStartedAtShort(row.startedAt),
        ),
      ]),
  },
  {
    title: "操作",
    key: "actions",
    width: 140,
    fixed: "right",
    render: (row) => {
      const loading = props.activeKillPids.includes(row.pid);
      const stopEvent = (event: MouseEvent) => event.stopPropagation();

      return h(
        NSpace,
        { size: 8, justify: "end", wrapItem: false },
        {
          default: () => [
            h(
              NPopconfirm,
              {
                positiveText: "结束",
                negativeText: "取消",
                onPositiveClick: () => emit("kill", row, false),
              },
              {
                trigger: () =>
                  h(
                    NButton,
                    {
                      size: "small",
                      secondary: true,
                      type: "warning",
                      loading,
                      disabled: loading,
                      onClick: stopEvent,
                    },
                    { default: () => "结束" },
                  ),
                default: () => `确认结束 PID ${row.pid}？`,
              },
            ),
            h(
              NPopconfirm,
              {
                positiveText: "强制结束",
                negativeText: "取消",
                onPositiveClick: () => emit("kill", row, true),
              },
              {
                trigger: () =>
                  h(
                    NButton,
                    {
                      size: "small",
                      tertiary: true,
                      type: "error",
                      loading,
                      disabled: loading,
                      onClick: stopEvent,
                    },
                    { default: () => "强制" },
                  ),
                default: () => `确认强制结束 PID ${row.pid}？`,
              },
            ),
          ],
        },
      );
    },
  },
]);
</script>

<template>
  <div
    ref="tableRootRef"
    :class="['table-root', { 'table-root--light': !isDarkTheme }]"
  >
    <n-data-table
      :columns="columns"
      :data="items"
      :bordered="false"
      :bottom-bordered="false"
      :single-line="false"
      :single-column="true"
      size="small"
      :max-height="tableMaxHeight"
      :scroll-x="942"
      :expanded-row-keys="expandedRowKeys"
      :row-key="getRowKey"
      @update:expanded-row-keys="expandedRowKeys = $event"
      @update:sorter="onSorterUpdate"
    />
  </div>
</template>

<style scoped>
.table-root {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

:deep(.n-data-table) {
  --n-td-color: transparent;
  --n-td-color-hover: rgba(30, 53, 83, 0.72);
  --n-td-color-striped: rgba(16, 30, 46, 0.4);
  --n-th-color: rgba(9, 21, 36, 0.9);
  --n-border-color: rgba(80, 107, 136, 0.16);
  --n-th-text-color: #7d96b5;
  --n-td-text-color: #e6effb;
  flex: 1;
  min-height: 0;
}

:deep(.n-data-table-th) {
  font-size: 11px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

:deep(.n-data-table .n-data-table-td) {
  padding-top: 6px;
  padding-bottom: 6px;
}

:deep(.n-data-table-wrapper) {
  min-height: 100%;
}

:deep(.n-data-table .n-data-table-expand .n-data-table-td) {
  padding-top: 2px;
  padding-bottom: 8px;
}

:deep(.port-cell),
:deep(.pid-cell),
:deep(.address-cell) {
  display: grid;
  gap: 2px;
  min-width: 0;
}

:deep(.port-cell) {
  align-content: center;
  white-space: nowrap;
}

:deep(.port-value) {
  color: #f5fbff;
  font-family: "SF Mono", "JetBrains Mono", "IBM Plex Mono", monospace;
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.03em;
}

:deep(.address-meta) {
  color: #6f89a7;
  font-size: 11px;
}

:deep(.process-cell) {
  display: flex;
  gap: 8px;
  align-items: center;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
}

:deep(.process-name) {
  display: block;
  flex: 1;
  max-width: 100%;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  color: #edf5ff;
  text-overflow: ellipsis;
  font-size: 13px;
  font-weight: 600;
}

:deep(.process-tag) {
  flex: 0 0 auto;
  margin-left: 2px;
}

:deep(.pid-cell) {
  white-space: nowrap;
}

:deep(.pid-label) {
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

:deep(.pid-value),
:deep(.address-value),
:deep(.uptime-cell) {
  font-family: "SF Mono", "JetBrains Mono", "IBM Plex Mono", monospace;
  font-size: 12px;
}

:deep(.address-value) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:deep(.address-cell),
:deep(.uptime-wrap) {
  min-width: 0;
  white-space: nowrap;
}

:deep(.uptime-cell) {
  color: #dce7f8;
}

.table-root--light :deep(.n-data-table) {
  --n-td-color: transparent;
  --n-td-color-hover: rgba(225, 235, 246, 0.86);
  --n-td-color-striped: rgba(241, 246, 252, 0.72);
  --n-th-color: rgba(244, 248, 253, 0.98);
  --n-border-color: rgba(92, 118, 149, 0.14);
  --n-th-text-color: #61778f;
  --n-td-text-color: #182c43;
}

.table-root--light :deep(.port-value),
.table-root--light :deep(.process-name) {
  color: #11263d;
}

.table-root--light :deep(.address-meta) {
  color: #73879d;
}

.table-root--light :deep(.uptime-cell) {
  color: #2f455d;
}
</style>
