<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { store, saveConfig, probeLine } from "../store";
import ManifestEditor from "./ManifestEditor.vue";

const p = computed(() => store.cfg.products[store.current] ?? null);
const probe = ref("");

async function refreshProbe() {
  probe.value = p.value ? await probeLine(p.value) : "";
}

watch(
  () => [p.value?.exePath, p.value?.kit, store.cfg.machine.qtRoot],
  () => void refreshProbe(),
  { immediate: true },
);

async function browseExe() {
  if (!p.value) return;
  const f = await open({ multiple: false, filters: [{ name: "可执行文件", extensions: ["exe"] }] });
  if (typeof f === "string") p.value.exePath = f;
}

async function browseOut() {
  if (!p.value) return;
  const d = await open({ directory: true, multiple: false });
  if (typeof d === "string") p.value.outputDir = d;
}

const argsText = computed({
  get: () => (p.value ? p.value.windeployqtArgs.join(" ") : ""),
  set: (v: string) => {
    if (p.value) p.value.windeployqtArgs = v.split(/\s+/).filter(Boolean);
  },
});
</script>

<template>
  <div v-if="!p" class="p-8 text-center opacity-60">左侧新建一个产品开始。</div>

  <div v-else class="p-4 space-y-4">
    <div class="flex items-center gap-2">
      <h2 class="text-xl font-semibold">{{ p.appName || "(未命名)" }}</h2>
      <span class="text-xs opacity-60 font-mono">{{ p.appId }}</span>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
      <label class="block">
        <span class="text-xs opacity-70">应用名（也是暂存目录名和 exe 名）</span>
        <input v-model="p.appName" class="input input-bordered input-sm w-full" />
      </label>

      <label class="block">
        <span class="text-xs opacity-70">GUID（不带花括号）</span>
        <input v-model="p.appId" class="input input-bordered input-sm w-full font-mono text-xs" />
      </label>

      <label class="block md:col-span-2">
        <span class="text-xs opacity-70">exe 路径</span>
        <div class="flex gap-2">
          <input
            v-model="p.exePath"
            class="input input-bordered input-sm w-full font-mono text-xs"
          />
          <button class="btn btn-sm" @click="browseExe">浏览</button>
        </div>
      </label>

      <label class="block">
        <span class="text-xs opacity-70">工具链</span>
        <select v-model="p.kit" class="select select-bordered select-sm w-full">
          <option value="auto">自动识别</option>
          <option v-for="k in store.kits" :key="k.id" :value="k.id">{{ k.id }}</option>
        </select>
      </label>

      <label class="block">
        <span class="text-xs opacity-70">安装包输出目录</span>
        <div class="flex gap-2">
          <input
            v-model="p.outputDir"
            class="input input-bordered input-sm w-full font-mono text-xs"
          />
          <button class="btn btn-sm" @click="browseOut">浏览</button>
        </div>
      </label>

      <label class="block">
        <span class="text-xs opacity-70">windeployqt 额外参数（空格分隔）</span>
        <input
          v-model="argsText"
          class="input input-bordered input-sm w-full font-mono text-xs"
          placeholder="--no-translations"
        />
      </label>

      <label class="block">
        <span class="text-xs opacity-70">版本覆盖（留空 = 读 exe 版本资源）</span>
        <input
          v-model="p.versionOverride"
          class="input input-bordered input-sm w-full font-mono text-xs"
        />
      </label>
    </div>

    <div v-if="probe" class="text-xs px-3 py-2 rounded bg-base-200">{{ probe }}</div>

    <ManifestEditor :entries="p.include" />

    <div class="card bg-base-200/60">
      <div class="card-body p-3 gap-2">
        <div class="flex items-center gap-2">
          <h3 class="font-medium text-sm">
            移除文件（部署完按通配删除，主要砍 windeployqt 多拉的）
          </h3>
          <div class="flex-1"></div>
          <button class="btn btn-xs" @click="p.exclude.push('')">+ 规则</button>
        </div>
        <table class="table table-xs">
          <tbody>
            <tr v-for="(pat, i) in p.exclude" :key="i">
              <td>
                <input
                  v-model="p.exclude[i]"
                  class="input input-xs w-full font-mono"
                  placeholder="plugins\sqldrivers\*"
                />
              </td>
              <td class="w-10 text-right">
                <button class="btn btn-ghost btn-xs" @click="p.exclude.splice(i, 1)">×</button>
              </td>
            </tr>
            <tr v-if="p.exclude.length === 0">
              <td class="text-center opacity-50">还没有排除规则</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="flex justify-end">
      <button class="btn btn-sm btn-primary" @click="saveConfig">保存配置</button>
    </div>
  </div>
</template>
