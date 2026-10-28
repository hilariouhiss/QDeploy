<script setup lang="ts">
import { computed, ref } from "vue";
import { store, formatWhen, formatMs } from "../store";

const onlyFailed = ref(false);
const rows = computed(() =>
  (onlyFailed.value ? store.history.filter((r) => !r.ok) : store.history).slice(-80).reverse(),
);

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
      <div class="sub">每次运行一条，落盘在 %APPDATA%/QDeploy/history.json</div>
    </div>
    <div class="actions">
      <button class="btn" :class="{ ghost: !onlyFailed }" @click="onlyFailed = !onlyFailed">
        {{ onlyFailed ? "只看失败" : "全部记录" }}
      </button>
    </div>
  </div>

  <div class="table">
    <div class="th" style="grid-template-columns: 168px 220px 96px 96px 1fr">
      <div>时间</div>
      <div>产品</div>
      <div style="text-align: right">版本</div>
      <div style="text-align: right">用时</div>
      <div>结果 / 安装包</div>
    </div>
    <div
      v-for="r in rows"
      :key="r.at + r.appName"
      class="tr"
      style="grid-template-columns: 168px 220px 96px 96px 1fr"
      @click="openProduct(r.appName)"
    >
      <div class="cell">{{ formatWhen(r.at) }}</div>
      <div class="cell pname" style="font-size: 13px">{{ r.appName }}</div>
      <div class="cell mono ver">{{ r.version || "—" }}</div>
      <div class="cell mono ver">{{ formatMs(r.durationMs) }}</div>
      <div class="cell" style="font-size: 13px">
        <span :style="r.ok ? '' : 'color:var(--negative)'">{{ r.ok ? "成功" : "失败" }}</span>
        <div class="pex mono">{{ r.ok ? r.installer : r.message }}</div>
      </div>
    </div>
    <div v-if="rows.length === 0" class="empty">还没有运行记录。</div>
  </div>
</template>
