<script setup lang="ts">
/**
 * CombinedTrendChart —— 组合趋势图（纯 SVG，零依赖）
 * 把三类运行记录合并到同一时间轴：
 *  - 覆盖率折线（amber）
 *  - 通过率折线（emerald，0-100% 共用刻度）
 *  - 生成事件标记（brand 蓝点，位于底部基线）
 * events 需按时间升序传入。
 */
import { computed } from "vue";

interface TrendEvent {
  type: "coverage" | "passRate" | "generation";
  /** coverage / passRate 的 0-100 数值 */
  value?: number;
  /** 显示标签（HH:MM） */
  time: string;
  /** 悬停详情 */
  detail?: string;
}

const props = withDefaults(
  defineProps<{ events: TrendEvent[]; height?: number }>(),
  { height: 220 },
);

const W = 640;
const PAD = { top: 14, right: 14, bottom: 10, left: 30 };

const H = computed(() => props.height);
const xMax = computed(() => Math.max(1, props.events.length - 1));

function xAt(index: number) {
  return PAD.left + (index / xMax.value) * (W - PAD.left - PAD.right);
}

function yAt(value: number) {
  const v = Math.min(100, Math.max(0, value));
  return PAD.top + (1 - v / 100) * (H.value - PAD.top - PAD.bottom);
}

/** 各序列按全局事件索引收集（保证同一时间轴） */
const coverage = computed(() =>
  props.events
    .map((e, i) => ({ i, e }))
    .filter(({ e }) => e.type === "coverage"),
);
const passRate = computed(() =>
  props.events
    .map((e, i) => ({ i, e }))
    .filter(({ e }) => e.type === "passRate"),
);
const generations = computed(() =>
  props.events
    .map((e, i) => ({ i, e }))
    .filter(({ e }) => e.type === "generation"),
);

const coveragePath = computed(() =>
  coverage.value
    .map(
      ({ i, e }, k) =>
        `${k === 0 ? "M" : "L"}${xAt(i).toFixed(1)},${yAt(e.value ?? 0).toFixed(1)}`,
    )
    .join(" "),
);

const passRatePath = computed(() =>
  passRate.value
    .map(
      ({ i, e }, k) =>
        `${k === 0 ? "M" : "L"}${xAt(i).toFixed(1)},${yAt(e.value ?? 0).toFixed(1)}`,
    )
    .join(" "),
);

/** 覆盖率折线下方淡色区域 */
const areaPoints = computed(() => {
  if (coverage.value.length < 2) return "";
  const pts = coverage.value.map(({ i, e }) =>
    `${xAt(i).toFixed(1)},${yAt(e.value ?? 0).toFixed(1)}`,
  );
  const x0 = xAt(coverage.value[0].i);
  const x1 = xAt(coverage.value[coverage.value.length - 1].i);
  const bottom = H.value - PAD.bottom;
  return `${x0.toFixed(1)},${bottom.toFixed(1)} ${pts.join(" ")} ${x1.toFixed(1)},${bottom.toFixed(1)}`;
});
</script>

<template>
  <svg
    :viewBox="`0 0 ${W} ${H}`"
    class="block w-full"
    :style="{ height: `${H}px` }"
    role="img"
    aria-label="Progress over time"
  >
    <!-- 纵轴网格与刻度（0 / 50 / 100%） -->
    <g v-for="gv in [0, 50, 100]" :key="gv">
      <line
        :x1="PAD.left"
        :x2="W - PAD.right"
        :y1="yAt(gv)"
        :y2="yAt(gv)"
        stroke="#f4f4f5"
        stroke-width="1"
      />
      <text
        :x="PAD.left - 6"
        :y="yAt(gv) + 3"
        text-anchor="end"
        font-size="9"
        fill="#a1a1aa"
      >
        {{ gv }}
      </text>
    </g>

    <!-- 80% 达标参考线（弱化，区别于数据） -->
    <line
      :x1="PAD.left"
      :x2="W - PAD.right"
      :y1="yAt(80)"
      :y2="yAt(80)"
      stroke="#e4e4e7"
      stroke-width="1"
      stroke-dasharray="3 3"
    />
    <text
      :x="W - PAD.right - 2"
      :y="yAt(80) - 4"
      text-anchor="end"
      font-size="9"
      fill="#d4d4d8"
    >
      80%
    </text>

    <!-- 覆盖率区域 + 折线 -->
    <polygon v-if="areaPoints" :points="areaPoints" fill="#f59e0b" opacity="0.08" />
    <path
      v-if="coveragePath && coverage.length > 1"
      :d="coveragePath"
      fill="none"
      stroke="#f59e0b"
      stroke-width="2"
      stroke-linejoin="round"
      stroke-linecap="round"
    />
    <g v-for="({ i, e }, k) in coverage" :key="`c-${k}`">
      <circle :cx="xAt(i)" :cy="yAt(e.value ?? 0)" r="3" fill="#f59e0b">
        <title>{{ e.detail ?? e.time }} · {{ (e.value ?? 0).toFixed(1) }}%</title>
      </circle>
      <!-- 数据点少时直接标注数值 -->
      <text
        v-if="coverage.length <= 6"
        :x="xAt(i) + 4"
        :y="yAt(e.value ?? 0) - 4"
        font-size="9"
        fill="#b45309"
      >
        {{ (e.value ?? 0).toFixed(1) }}
      </text>
    </g>

    <!-- 通过率折线 -->
    <path
      v-if="passRatePath && passRate.length > 1"
      :d="passRatePath"
      fill="none"
      stroke="#10b981"
      stroke-width="2"
      stroke-linejoin="round"
      stroke-linecap="round"
    />
    <g v-for="({ i, e }, k) in passRate" :key="`p-${k}`">
      <circle :cx="xAt(i)" :cy="yAt(e.value ?? 0)" r="3" fill="#10b981">
        <title>{{ e.detail ?? e.time }} · {{ (e.value ?? 0).toFixed(1) }}%</title>
      </circle>
      <text
        v-if="passRate.length <= 6"
        :x="xAt(i) + 4"
        :y="yAt(e.value ?? 0) - 4"
        font-size="9"
        fill="#047857"
      >
        {{ (e.value ?? 0).toFixed(1) }}
      </text>
    </g>

    <!-- 生成事件标记（底部基线） -->
    <g v-for="({ i, e }, k) in generations" :key="`g-${k}`">
      <circle :cx="xAt(i)" :cy="yAt(0) + 8" r="2.5" fill="#3b6ef0">
        <title>{{ e.detail ?? e.time }}</title>
      </circle>
    </g>
  </svg>
</template>
