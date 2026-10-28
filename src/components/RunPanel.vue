<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { store } from "../store";

const rows = computed(() =>
  store.cfg.products
    .filter((p) => store.rows[p.appName])
    .map((p) => ({ name: p.appName, ...store.rows[p.appName] })),
);

const active = ref("");
watch(rows, (v) => {
  if (v.length > 0 && !v.some((r) => r.name === active.value)) active.value = v[0].name;
});

const logText = computed(() => (store.logs[active.value] ?? []).join("\n"));

const badge = (status: string) =>
  status === "ok"
    ? "badge-success"
    : status === "failed"
      ? "badge-error"
      : status === "running"
        ? "badge-info"
        : "badge-ghost";
</script>

<template>
  <div v-if="rows.length" class="border-t border-base-300 p-3 space-y-2">
    <div class="flex flex-wrap gap-2">
      <button
        v-for="r in rows"
        :key="r.name"
        class="btn btn-xs"
        :class="active === r.name ? 'btn-neutral' : 'btn-ghost'"
        @click="active = r.name"
      >
        <span class="badge badge-xs" :class="badge(r.status)"></span>
        {{ r.name }}
        <span v-if="r.step" class="opacity-60">{{ r.step }}</span>
      </button>
    </div>

    <div v-for="r in rows.filter((x) => x.name === active)" :key="r.name" class="text-xs space-y-1">
      <div v-if="r.message" :class="r.status === 'failed' ? 'text-error' : ''">{{ r.message }}</div>
      <div v-if="r.stagingKept" class="opacity-60">暂存已保留：{{ r.stagingKept }}</div>
      <div v-if="r.installer" class="opacity-60">安装包：{{ r.installer }}</div>
    </div>

    <pre class="h-48 overflow-auto bg-base-200 rounded p-2 text-xs whitespace-pre-wrap">{{
      logText
    }}</pre>
  </div>
</template>
