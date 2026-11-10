<script setup lang="ts">
import { computed, ref } from "vue";
import { store, formatWhen, formatMs } from "../store";

const onlyFailed = ref(false);
const failedCount = computed(() => store.history.filter((r) => !r.ok).length);
const rows = computed(() =>
  (onlyFailed.value ? store.history.filter((r) => !r.ok) : store.history).slice(-80).reverse(),
);
const lastAt = computed(() => {
  const h = store.history;
  return h.length ? h[h.length - 1].at : 0;
});

function openProduct(name: string) {
  const i = store.cfg.products.findIndex((p) => p.appName === name);
  if (i < 0) return;
  store.current = i;
  store.detail = true;
  store.page = "products";
}
</script>

<template>
  <div class="titlerow">
    <div>
      <h1>发版记录</h1>
      <div class="sub">
        {{ store.history.length }} 次运行 · {{ failedCount }} 次失败<template v-if="lastAt">
          · 最近一次 {{ formatWhen(lastAt) }}</template
        >
      </div>
    </div>
  </div>

  <div class="tabs">
    <button class="tab" :class="{ on: !onlyFailed }" @click="onlyFailed = false">
      全部<span class="n">{{ store.history.length }}</span>
    </button>
    <button class="tab" :class="{ on: onlyFailed }" @click="onlyFailed = true">
      只看失败<span class="n">{{ failedCount }}</span>
    </button>
  </div>

  <div class="toolbar">
    <div class="note">
      显示最近 {{ rows.length }} 条，点一行打开对应产品<template v-if="rows.length">
        · 最早 {{ formatWhen(rows[rows.length - 1].at) }}</template
      >
    </div>
  </div>

  <div class="table hist">
    <div class="thead">
      <div>时间</div>
      <div>产品</div>
      <div class="r">版本</div>
      <div class="r">用时</div>
      <div>结果 / 安装包</div>
    </div>
    <div
      v-for="r in rows"
      :key="r.at + r.appName"
      class="tr"
      tabindex="0"
      @click="openProduct(r.appName)"
      @keydown.enter="openProduct(r.appName)"
    >
      <div class="cell c-time when">{{ formatWhen(r.at) }}</div>
      <div class="cell c-name pname">{{ r.appName }}</div>
      <div class="cell c-ver ver"><span class="k">版本</span>{{ r.version || "—" }}</div>
      <div class="cell c-dur ver"><span class="k">用时</span>{{ formatMs(r.durationMs) }}</div>
      <div class="cell c-detail">
        <span class="res" :class="{ fail: !r.ok }">{{ r.ok ? "成功" : "失败" }}</span>
        <div class="pex" :title="r.ok ? r.installer : r.message">
          {{ r.ok ? r.installer : r.message }}
        </div>
      </div>
    </div>

    <div v-if="rows.length === 0" class="empty">
      <template v-if="store.history.length === 0">
        <div class="t">还没有运行记录</div>
        <div class="d">在产品页勾选产品并运行后，每次运行都会在这里留一条记录。</div>
      </template>
      <template v-else>
        <div class="t">没有失败的记录</div>
        <div class="d">最近 {{ store.history.length }} 次运行都成功了。</div>
        <div class="row"><button class="btn" @click="onlyFailed = false">看全部记录</button></div>
      </template>
    </div>
  </div>
</template>
