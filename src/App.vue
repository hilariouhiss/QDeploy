<script setup lang="ts">
import { onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  store,
  loadConfig,
  wireEvents,
  saveConfig,
  startRun,
  cancelRun,
  checkedNames,
} from "./store";
import { emptyProduct } from "./types";
import ProductPage from "./components/ProductPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
import RunPanel from "./components/RunPanel.vue";

onMounted(async () => {
  await loadConfig();
  await wireEvents();
  await invoke("is_running").then((r) => (store.running = Boolean(r)));
});

function newGuid(): string {
  if (crypto.randomUUID) return crypto.randomUUID().toUpperCase();
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    .replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0;
      return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
    })
    .toUpperCase();
}

function addProduct() {
  const p = emptyProduct();
  p.appName = "Product" + (store.cfg.products.length + 1);
  p.appId = newGuid();
  store.cfg.products.push(p);
  store.current = store.cfg.products.length - 1;
  store.page = "product";
}

function selectProduct(i: number) {
  store.current = i;
  store.page = "product";
}

function removeProduct(i: number) {
  const name = store.cfg.products[i].appName;
  if (!confirm("删除产品 " + name + "？")) return;
  store.cfg.products.splice(i, 1);
  delete store.checked[name];
  if (store.current >= store.cfg.products.length) store.current = store.cfg.products.length - 1;
}

const run = (stage: "deploy" | "package" | "both") => startRun(stage);
</script>

<template>
  <div class="flex h-full bg-base-100 text-base-content">
    <aside class="w-64 shrink-0 border-r border-base-300 flex flex-col">
      <div class="p-3 border-b border-base-300">
        <h1 class="text-lg font-semibold">QDeploy</h1>
        <p class="text-xs opacity-60">Qt 部署 · 安装包</p>
      </div>

      <ul class="menu menu-sm flex-1 overflow-auto p-2 gap-0.5">
        <li v-for="(p, i) in store.cfg.products" :key="p.appName + i">
          <div
            class="flex items-center gap-2 w-full"
            :class="{ active: store.page === 'product' && store.current === i }"
          >
            <input
              type="checkbox"
              class="checkbox checkbox-xs"
              :checked="!!store.checked[p.appName]"
              @change="store.checked[p.appName] = ($event.target as HTMLInputElement).checked"
              @click.stop
            />
            <a class="flex-1 truncate" @click="selectProduct(i)">{{ p.appName || "(未命名)" }}</a>
            <button class="btn btn-ghost btn-xs px-1" title="删除" @click.stop="removeProduct(i)">
              ×
            </button>
          </div>
        </li>
      </ul>

      <div class="p-2 border-t border-base-300 space-y-1">
        <button class="btn btn-sm btn-block" @click="addProduct">+ 新建产品</button>
        <button class="btn btn-sm btn-block btn-ghost" @click="store.page = 'settings'">
          设置
        </button>
      </div>
    </aside>

    <main class="flex-1 flex flex-col min-w-0">
      <div class="flex items-center gap-2 p-3 border-b border-base-300">
        <button class="btn btn-sm btn-primary" :disabled="store.running" @click="run('both')">
          部署并打包
        </button>
        <button class="btn btn-sm" :disabled="store.running" @click="run('deploy')">仅部署</button>
        <button class="btn btn-sm" :disabled="store.running" @click="run('package')">仅打包</button>
        <button class="btn btn-sm btn-ghost" :disabled="!store.running" @click="cancelRun">
          取消
        </button>
        <span v-if="store.running" class="loading loading-spinner loading-sm"></span>
        <div class="flex-1"></div>
        <span class="text-xs opacity-60">{{ checkedNames().length }} 个已勾选</span>
        <button class="btn btn-sm" @click="saveConfig">保存配置</button>
      </div>

      <div v-if="store.message" class="px-3 py-1 text-xs bg-base-200">{{ store.message }}</div>

      <div class="flex-1 overflow-auto min-h-0">
        <SettingsPage v-if="store.page === 'settings'" />
        <ProductPage v-else />
      </div>

      <RunPanel />
    </main>
  </div>
</template>
