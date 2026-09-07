<script setup lang="ts">
import { computed } from "vue";
import {
  ChevronDown,
  ChevronRight,
  Check,
  Folder,
  FolderOpen,
  FileCode2,
  AlertTriangle,
} from "@lucide/vue";

// 递归组件需要显式声明 name (Vue 3.3+)
defineOptions({ name: "TreeItem" });

interface TreeNode {
  id: string;
  name: string;
  path: string;
  relativePath: string;
  type: "file" | "directory";
  children: TreeNode[];
  depth: number;
}

const props = defineProps<{
  node: TreeNode;
  expandedDirs: Set<string>;
  selectedFiles: string[];
  disabled?: boolean;
  blockedFiles?: string[];
  blockedTip?: string;
}>();

const emit = defineEmits<{
  (e: "toggle-dir", path: string): void;
  (e: "toggle-file", path: string): void;
  (e: "context-menu", payload: { node: TreeNode; ev: MouseEvent }): void;
}>();

const isExpanded = computed(() => props.expandedDirs.has(props.node.path));
const isSelected = computed(() =>
  props.selectedFiles.includes(props.node.relativePath)
);
const isBlocked = computed(() =>
  props.blockedFiles?.includes(props.node.relativePath) ?? false
);

function onContextMenu(ev: MouseEvent) {
  emit("context-menu", { node: props.node, ev });
}
</script>

<template>
  <div>
    <div
      class="flex items-center gap-2 border-b border-slate-100 px-3 py-2 text-left transition last:border-b-0 hover:bg-slate-50"
      :style="{ paddingLeft: node.depth * 16 + 12 + 'px' }"
      @contextmenu.prevent="onContextMenu"
    >
      <!-- 目录节点 -->
      <template v-if="node.type === 'directory'">
        <button
          type="button"
          class="flex h-4 w-4 shrink-0 items-center justify-center"
          @click="emit('toggle-dir', node.path)"
        >
          <ChevronDown v-if="isExpanded" class="h-3.5 w-3.5 text-slate-500" />
          <ChevronRight v-else class="h-3.5 w-3.5 text-slate-500" />
        </button>

        <FolderOpen v-if="isExpanded" class="h-4 w-4 shrink-0 text-brand-500" />
        <Folder v-else class="h-4 w-4 shrink-0 text-slate-400" />

        <button
          type="button"
          class="min-w-0 flex-1 truncate text-sm font-medium text-slate-800"
          @click="emit('toggle-dir', node.path)"
        >
          {{ node.name }}
        </button>
      </template>

      <!-- 文件节点 -->
      <template v-else>
        <button
          type="button"
          :disabled="disabled || isBlocked"
          class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition disabled:cursor-not-allowed"
          :class="
            isSelected
              ? 'border-brand-500 bg-brand-500 text-white'
              : isBlocked
                ? 'border-amber-200 bg-amber-50'
                : 'border-slate-300 bg-white'
          "
          @click="emit('toggle-file', node.relativePath)"
        >
          <Check v-if="isSelected" class="h-3 w-3" />
        </button>

        <FileCode2 class="h-4 w-4 shrink-0 text-slate-400" />

        <button
          type="button"
          :disabled="disabled || isBlocked"
          :title="isBlocked ? blockedTip : undefined"
          class="min-w-0 flex-1 truncate text-left text-sm disabled:cursor-not-allowed"
          :class="isBlocked ? 'text-amber-600' : 'text-slate-800'"
          @click="emit('toggle-file', node.relativePath)"
        >
          {{ node.name }}
        </button>

        <AlertTriangle
          v-if="isBlocked"
          class="h-4 w-4 shrink-0 text-amber-500"
        />
      </template>
    </div>

    <!-- 递归渲染子节点 -->
    <template v-if="node.type === 'directory' && isExpanded">
      <TreeItem
        v-for="child in node.children"
        :key="child.id"
        :node="child"
        :expanded-dirs="expandedDirs"
        :selected-files="selectedFiles"
        :disabled="disabled"
        :blocked-files="blockedFiles"
        :blocked-tip="blockedTip"
        @toggle-dir="emit('toggle-dir', $event)"
        @toggle-file="emit('toggle-file', $event)"
        @context-menu="emit('context-menu', $event)"
      />
    </template>
  </div>
</template>
