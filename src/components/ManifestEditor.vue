<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import type { ProductConfig } from "../types";

const props = defineProps<{ product: ProductConfig }>();

async function addFile(dir: boolean) {
  const picked = await open({ directory: dir, multiple: false });
  if (typeof picked !== "string") return;
  const parts = picked.split(/[\\/]/);
  props.product.include.push({ source: picked, target: parts[parts.length - 1] || "" });
}
</script>

<template>
  <div>
    <div class="h2">
      文件清单
      <span class="side"
        >添加 {{ product.include.length }} · 移除 {{ product.exclude.length }}</span
      >
    </div>

    <table class="mtable">
      <thead>
        <tr>
          <th>添加（相对 exe 目录或绝对路径）</th>
          <th>暂存目录内</th>
          <th style="width: 56px"></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(e, i) in product.include" :key="i">
          <td><input v-model="e.source" class="input mono" style="height: 26px" /></td>
          <td><input v-model="e.target" class="input mono" style="height: 26px" /></td>
          <td style="text-align: right">
            <button
              class="btn ghost"
              style="height: 24px; padding: 0 6px"
              @click="product.include.splice(i, 1)"
            >
              ×
            </button>
          </td>
        </tr>
        <tr v-if="product.include.length === 0">
          <td colspan="3" class="muted">没有额外文件</td>
        </tr>
      </tbody>
    </table>
    <div style="display: flex; gap: 8px; margin-top: 8px">
      <button class="btn ghost" @click="addFile(false)">+ 文件</button>
      <button class="btn ghost" @click="addFile(true)">+ 目录</button>
      <button class="btn ghost" @click="product.include.push({ source: '', target: '' })">
        + 手填
      </button>
    </div>

    <table class="mtable" style="margin-top: 18px">
      <thead>
        <tr>
          <th>移除（相对暂存目录的通配）</th>
          <th style="width: 40px"></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(pat, i) in product.exclude" :key="i">
          <td><input v-model="product.exclude[i]" class="input mono" style="height: 26px" /></td>
          <td style="text-align: right">
            <button
              class="btn ghost"
              style="height: 24px; padding: 0 6px"
              @click="product.exclude.splice(i, 1)"
            >
              ×
            </button>
          </td>
        </tr>
        <tr v-if="product.exclude.length === 0">
          <td colspan="2" class="muted">没有排除规则</td>
        </tr>
      </tbody>
    </table>
    <div style="margin-top: 8px">
      <button class="btn ghost" @click="product.exclude.push('')">+ 规则</button>
    </div>
  </div>
</template>
