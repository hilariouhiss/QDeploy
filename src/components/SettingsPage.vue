<script setup lang="ts">
import { ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { store, scanKits, saveConfig, clearStaging } from "../store";

const kitInfo = ref("");

async function browseDir(field: "qtRoot" | "stagingRoot") {
  const d = await open({ directory: true, multiple: false });
  if (typeof d === "string") store.cfg.machine[field] = d;
}

async function browseFile(field: "isccExe" | "issTemplatePath", exts: string[]) {
  const f = await open({
    multiple: false,
    filters: [{ name: exts[0].toUpperCase(), extensions: exts }],
  });
  if (typeof f === "string") store.cfg.machine[field] = f;
}

async function doScan() {
  await scanKits();
  kitInfo.value =
    store.kits.length === 0
      ? "一个套件都没扫到，检查 Qt 安装根"
      : "扫到 " + store.kits.length + " 个套件";
}
</script>

<template>
  <div class="p-4 space-y-4 max-w-3xl">
    <h2 class="text-xl font-semibold">设置（这台机器）</h2>

    <label class="block">
      <span class="text-xs opacity-70">Qt 安装根</span>
      <div class="flex gap-2">
        <input
          v-model="store.cfg.machine.qtRoot"
          class="input input-bordered input-sm w-full font-mono text-xs"
          placeholder="C:\\Qt"
        />
        <button class="btn btn-sm" @click="browseDir('qtRoot')">浏览</button>
      </div>
    </label>

    <label class="block">
      <span class="text-xs opacity-70">ISCC.exe</span>
      <div class="flex gap-2">
        <input
          v-model="store.cfg.machine.isccExe"
          class="input input-bordered input-sm w-full font-mono text-xs"
        />
        <button class="btn btn-sm" @click="browseFile('isccExe', ['exe'])">浏览</button>
      </div>
    </label>

    <label class="block">
      <span class="text-xs opacity-70">部署暂存根</span>
      <div class="flex gap-2">
        <input
          v-model="store.cfg.machine.stagingRoot"
          class="input input-bordered input-sm w-full font-mono text-xs"
        />
        <button class="btn btn-sm" @click="browseDir('stagingRoot')">浏览</button>
      </div>
    </label>

    <label class="block">
      <span class="text-xs opacity-70">iss 模板（common_setup.iss）</span>
      <div class="flex gap-2">
        <input
          v-model="store.cfg.machine.issTemplatePath"
          class="input input-bordered input-sm w-full font-mono text-xs"
        />
        <button class="btn btn-sm" @click="browseFile('issTemplatePath', ['iss'])">浏览</button>
      </div>
    </label>

    <div class="flex items-center gap-2">
      <button class="btn btn-sm" @click="doScan">扫描套件</button>
      <button class="btn btn-sm" @click="saveConfig">保存配置</button>
      <button class="btn btn-sm btn-ghost" @click="clearStaging">清空全部暂存</button>
      <span class="text-xs opacity-60">{{ kitInfo }}</span>
    </div>

    <table class="table table-sm">
      <thead>
        <tr>
          <th>套件</th>
          <th>Qt</th>
          <th>架构</th>
          <th>编译器</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="k in store.kits" :key="k.id">
          <td class="font-mono text-xs">{{ k.id }}</td>
          <td>{{ k.qtVersion }}</td>
          <td>{{ k.arch }} 位</td>
          <td>{{ k.compiler }}</td>
        </tr>
        <tr v-if="store.kits.length === 0">
          <td colspan="4" class="text-center opacity-50">还没扫描</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
