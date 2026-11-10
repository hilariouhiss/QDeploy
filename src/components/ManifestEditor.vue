<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import type { ProductConfig } from "../types";

const props = defineProps<{ product: ProductConfig }>();

async function addFile(dir: boolean) {
  const picked = await open({ directory: dir, multiple: !dir });
  if (!picked) return;
  for (const p of Array.isArray(picked) ? picked : [picked]) {
    const parts = p.split(/[\\/]/);
    props.product.include.push({ source: p, target: parts[parts.length - 1] || "" });
  }
}
</script>

<template>
  <div class="manifest">
    <div class="manifest-head">
      <span>添加（相对 exe 目录或绝对路径）</span>
      <span class="add">
        <button class="btn sm ghost" @click="addFile(false)">+ 文件</button>
        <button class="btn sm ghost" @click="addFile(true)">+ 目录</button>
        <button class="btn sm ghost" @click="product.include.push({ source: '', target: '' })">
          + 手填
        </button>
      </span>
    </div>
    <div v-for="(e, i) in product.include" :key="i" class="row-edit">
      <input v-model="e.source" class="input mono" :title="e.source" placeholder="源路径" />
      <input v-model="e.target" class="input mono" :title="e.target" placeholder="暂存目录内" />
      <button class="xbtn" title="删除这一条" @click="product.include.splice(i, 1)">×</button>
    </div>
    <div v-if="product.include.length === 0" class="empty-row">没有额外文件</div>

    <div class="manifest">
      <div class="manifest-head">
        <span>移除（相对暂存目录的通配）</span>
        <span class="add">
          <button class="btn sm ghost" @click="product.exclude.push('')">+ 规则</button>
        </span>
      </div>
      <div v-for="(pat, i) in product.exclude" :key="i" class="row-edit one">
        <input
          v-model="product.exclude[i]"
          class="input mono"
          :title="pat"
          placeholder="plugins\*"
        />
        <button class="xbtn" title="删除这一条" @click="product.exclude.splice(i, 1)">×</button>
      </div>
      <div v-if="product.exclude.length === 0" class="empty-row">没有排除规则</div>
    </div>
  </div>
</template>
