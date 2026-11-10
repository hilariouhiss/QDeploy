<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { store, loadConfig, fetchHistory, wireEvents, scanKits } from "./store";
import ProductsView from "./views/ProductsView.vue";
import ProductView from "./views/ProductView.vue";
import HistoryView from "./views/HistoryView.vue";
import ScriptsView from "./views/ScriptsView.vue";
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

/** Sidebar switch must also leave the product detail, or "产品" looks dead there. */
function go(page: "products" | "history" | "scripts" | "settings") {
  store.detail = false;
  store.page = page;
}

const workRoot = computed(
  () => store.cfg.machine.workRoot || store.defaultWorkRoot || "工作目录未设置",
);
const iscc = computed(() => (store.cfg.machine.isccExe ? "已设置" : "未设置"));
</script>

<template>
  <div class="frame">
    <header class="strip" data-tauri-drag-region>
      <svg class="mark" viewBox="0 0 18 18" aria-hidden="true" data-tauri-drag-region>
        <rect x="0" y="0" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
        <rect x="6.6" y="0" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
        <rect x="13.2" y="0" width="4.8" height="4.8" rx="0.9" fill="#b9bec5" />
        <rect x="0" y="6.6" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
        <rect x="6.6" y="6.6" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
        <rect x="13.2" y="6.6" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
        <rect x="0" y="13.2" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
        <rect x="6.6" y="13.2" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
        <rect x="13.2" y="13.2" width="4.8" height="4.8" rx="0.9" fill="#0a0a0a" />
      </svg>
      <span class="brand" data-tauri-drag-region>QDeploy</span>
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

    <nav class="side" aria-label="主导航">
      <div class="scope">
        <div class="scope-label">本机</div>
        <div class="scope-path" :title="workRoot">{{ workRoot }}</div>
        <div class="scope-meta">{{ store.kits.length }} 个套件 · ISCC {{ iscc }}</div>
      </div>
      <div class="nav">
        <button
          class="navitem"
          :class="{ on: store.page === 'products' }"
          :aria-current="store.page === 'products' ? 'page' : undefined"
          @click="go('products')"
        >
          产品
        </button>
        <button
          class="navitem"
          :class="{ on: store.page === 'history' }"
          :aria-current="store.page === 'history' ? 'page' : undefined"
          @click="go('history')"
        >
          发版记录
        </button>
        <button
          class="navitem"
          :class="{ on: store.page === 'scripts' }"
          :aria-current="store.page === 'scripts' ? 'page' : undefined"
          @click="go('scripts')"
        >
          安装脚本
        </button>
        <button
          class="navitem"
          :class="{ on: store.page === 'settings' }"
          :aria-current="store.page === 'settings' ? 'page' : undefined"
          @click="go('settings')"
        >
          设置
        </button>
      </div>
      <div class="side-foot">模板 <span class="mono">common_setup.iss</span></div>
    </nav>

    <main class="canvas">
      <ProductsView v-if="store.page === 'products' && !store.detail" />
      <ProductView v-else-if="store.page === 'products'" />
      <HistoryView v-else-if="store.page === 'history'" />
      <ScriptsView v-else-if="store.page === 'scripts'" />
      <SettingsView v-else />
    </main>
  </div>
</template>
