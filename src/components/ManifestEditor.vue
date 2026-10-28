<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import type { ManifestEntry } from "../types";

const props = defineProps<{ entries: ManifestEntry[] }>();

async function add(entry: ManifestEntry) {
  props.entries.push(entry);
}

async function pick(index: number, dir: boolean) {
  const picked = await open({ directory: dir, multiple: false });
  if (typeof picked !== "string") return;
  const entry = props.entries[index];
  entry.source = picked;
  if (!entry.target) entry.target = picked.split(/[\\/]/).pop() ?? "";
}
</script>

<template>
  <div class="card bg-base-200/60">
    <div class="card-body p-3 gap-2">
      <div class="flex items-center gap-2">
        <h3 class="font-medium text-sm">添加文件（部署时复制进暂存目录）</h3>
        <div class="flex-1"></div>
        <button class="btn btn-xs" @click="add({ source: '', target: '' })">+ 手填</button>
        <button
          class="btn btn-xs"
          @click="
            add({ source: '', target: '' });
            pick(entries.length - 1, false);
          "
        >
          + 文件
        </button>
        <button
          class="btn btn-xs"
          @click="
            add({ source: '', target: '' });
            pick(entries.length - 1, true);
          "
        >
          + 目录
        </button>
      </div>

      <table class="table table-xs">
        <thead>
          <tr>
            <th>源（相对 exe 目录，或绝对路径）</th>
            <th>暂存目录内目标</th>
            <th class="w-16"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(e, i) in entries" :key="i">
            <td><input v-model="e.source" class="input input-xs w-full font-mono" /></td>
            <td><input v-model="e.target" class="input input-xs w-full font-mono" /></td>
            <td class="text-right">
              <button class="btn btn-ghost btn-xs" @click="pick(i, false)">选</button>
              <button class="btn btn-ghost btn-xs" @click="entries.splice(i, 1)">×</button>
            </td>
          </tr>
          <tr v-if="entries.length === 0">
            <td colspan="3" class="text-center opacity-50">还没有添加项</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
