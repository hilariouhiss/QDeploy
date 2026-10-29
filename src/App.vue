<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { store, loadConfig, fetchHistory, wireEvents, scanKits } from "./store";
import ProductsView from "./views/ProductsView.vue";
import ProductView from "./views/ProductView.vue";
import HistoryView from "./views/HistoryView.vue";
import SettingsView from "./views/SettingsView.vue";

const win = getCurrentWindow();
const maximized = ref(false);

onMounted(async () => {
  await loadConfig();
  await fetchHistory();
  await wireEvents();
  if (store.cfg.machine.qtRoot) await scanKits();
  maximized.value = await win.isMaximized();
  await win.onResized(async () => {
    maximized.value = await win.isMaximized();
  });
});

async function toggleMax() {
  await win.toggleMaximize();
  maximized.value = await win.isMaximized();
}

const stagingTail = computed(() => store.cfg.machine.stagingRoot || "未设置暂存根");
const failed = computed(() => store.history.filter((r) => !r.ok).length);
</script>

<template>
  <div class="frame">
    <header class="topbar" data-tauri-drag-region>
      <svg class="mark" viewBox="0 0 20 20" aria-hidden="true" data-tauri-drag-region>
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
      <span class="brand" data-tauri-drag-region>QDeploy</span>
      <div class="scope" data-tauri-drag-region>
        <span class="mono">{{ store.cfg.machine.qtRoot || "Qt 根未设置" }}</span>
        <span>{{ store.kits.length }} 个套件</span>
      </div>
      <div class="wincontrols">
        <button class="winbtn" title="最小化" @click="win.minimize()">
          <svg viewBox="0 0 16 16">
            <path d="M4 8.5h8" stroke="currentColor" stroke-width="1.2" />
          </svg>
        </button>
        <button class="winbtn" :title="maximized ? '还原' : '最大化'" @click="toggleMax">
          <svg v-if="maximized" viewBox="0 0 16 16">
            <rect
              x="3.6"
              y="5.6"
              width="6.8"
              height="6.8"
              fill="none"
              stroke="currentColor"
              stroke-width="1.2"
            />
            <path d="M5.6 3.6h6.8v6.8" fill="none" stroke="currentColor" stroke-width="1.2" />
          </svg>
          <svg v-else viewBox="0 0 16 16">
            <rect
              x="4.6"
              y="4.6"
              width="6.8"
              height="6.8"
              fill="none"
              stroke="currentColor"
              stroke-width="1.2"
            />
          </svg>
        </button>
        <button class="winbtn close" title="关闭" @click="win.close()">
          <svg viewBox="0 0 16 16">
            <path d="M4.8 4.8l6.4 6.4M11.2 4.8l-6.4 6.4" stroke="currentColor" stroke-width="1.2" />
          </svg>
        </button>
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
        {{ stagingTail }}
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
