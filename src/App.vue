<script setup lang="ts">
import { computed, onMounted } from "vue";
import { store, loadConfig, fetchHistory, wireEvents, scanKits } from "./store";
import ProductsView from "./views/ProductsView.vue";
import ProductView from "./views/ProductView.vue";
import HistoryView from "./views/HistoryView.vue";
import SettingsView from "./views/SettingsView.vue";

onMounted(async () => {
  await loadConfig();
  await fetchHistory();
  await wireEvents();
  if (store.cfg.machine.qtRoot) await scanKits();
});

const stagingTail = computed(() => store.cfg.machine.stagingRoot || "未设置暂存根");
const templateName = computed(() => {
  const p = store.cfg.machine.issTemplatePath;
  return p ? p.split(/[\\/]/).pop() : "未设置 iss 模板";
});
const failed = computed(() => store.history.filter((r) => !r.ok).length);
</script>

<template>
  <div class="frame">
    <header class="topbar">
      <svg class="mark" viewBox="0 0 20 20" aria-hidden="true">
        <rect x="0" y="0" width="5.6" height="5.6" rx="1" fill="#131416" />
        <rect x="7.2" y="0" width="5.6" height="5.6" rx="1" fill="#131416" />
        <rect x="14.4" y="0" width="5.6" height="5.6" rx="1" fill="#131416" />
        <rect x="0" y="7.2" width="5.6" height="5.6" rx="1" fill="#131416" />
        <rect x="7.2" y="7.2" width="5.6" height="5.6" rx="1" fill="#131416" />
        <rect x="14.4" y="7.2" width="5.6" height="5.6" rx="1" fill="#dfe3e5" />
        <rect x="0" y="14.4" width="5.6" height="5.6" rx="1" fill="#131416" />
        <rect x="7.2" y="14.4" width="5.6" height="5.6" rx="1" fill="#131416" />
        <rect x="14.4" y="14.4" width="5.6" height="5.6" rx="1" fill="#131416" />
      </svg>
      <span class="brand">QDeploy</span>
      <div class="scope">
        <span class="mono">{{ store.cfg.machine.qtRoot || "Qt 根未设置" }}</span>
        <span>{{ store.kits.length }} 个套件</span>
      </div>
    </header>

    <nav class="side">
      <button
        class="navitem"
        :class="{ on: store.page === 'products' }"
        @click="store.page = 'products'"
      >
        产品<span class="count">{{ store.cfg.products.length }}</span>
      </button>
      <button
        class="navitem"
        :class="{ on: store.page === 'history' }"
        @click="store.page = 'history'"
      >
        发版记录<span class="count">{{ store.history.length }}</span>
      </button>
      <button
        class="navitem"
        :class="{ on: store.page === 'settings' }"
        @click="store.page = 'settings'"
      >
        设置
      </button>
      <div class="side-foot">
        <b>本机</b>
        {{ stagingTail }}<br />
        {{ templateName }}
        <div v-if="failed" style="color: var(--negative); margin-top: 6px">
          {{ failed }} 次失败记录
        </div>
      </div>
    </nav>

    <div class="sheetwrap">
      <main class="sheet">
        <ProductsView v-if="store.page === 'products' && !store.detail" />
        <ProductView v-else-if="store.page === 'products'" />
        <HistoryView v-else-if="store.page === 'history'" />
        <SettingsView v-else />
      </main>
    </div>
  </div>
</template>
