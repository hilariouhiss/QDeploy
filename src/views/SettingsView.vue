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
  <div class="titlerow">
    <div>
      <h1>设置</h1>
      <div class="sub">这台机器的路径。换电脑只改这一页。</div>
    </div>
    <div class="actions">
      <button class="btn" @click="doScan">扫描套件</button>
      <button class="btn" @click="saveConfig">保存</button>
    </div>
  </div>

  <div v-if="store.message" class="banner" style="margin-top: 16px">{{ store.message }}</div>

  <div class="detail" style="margin-top: 20px">
    <div>
      <label class="field">
        <span>Qt 安装根</span>
        <div class="with-btn">
          <input v-model="store.cfg.machine.qtRoot" class="input mono" placeholder="C:\Qt" />
          <button class="btn" @click="browseDir('qtRoot')">浏览</button>
        </div>
      </label>

      <label class="field" style="margin-top: 16px">
        <span>ISCC.exe</span>
        <div class="with-btn">
          <input v-model="store.cfg.machine.isccExe" class="input mono" />
          <button class="btn" @click="browseFile('isccExe', ['exe'])">浏览</button>
        </div>
      </label>

      <label class="field" style="margin-top: 16px">
        <span>部署暂存根</span>
        <div class="with-btn">
          <input v-model="store.cfg.machine.stagingRoot" class="input mono" />
          <button class="btn" @click="browseDir('stagingRoot')">浏览</button>
        </div>
      </label>

      <label class="field" style="margin-top: 16px">
        <span>iss 模板（common_setup.iss）</span>
        <div class="with-btn">
          <input v-model="store.cfg.machine.issTemplatePath" class="input mono" />
          <button class="btn" @click="browseFile('issTemplatePath', ['iss'])">浏览</button>
        </div>
      </label>

      <div style="margin-top: 20px">
        <button class="btn ghost" @click="clearStaging">清空全部暂存</button>
        <span class="muted" style="font-size: 12.5px; margin-left: 10px">{{ kitInfo }}</span>
      </div>
    </div>

    <aside>
      <div class="h2">已识别的套件</div>
      <table class="mtable">
        <thead>
          <tr>
            <th>套件</th>
            <th>Qt</th>
            <th>架构</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="k in store.kits" :key="k.id">
            <td class="p">{{ k.id }}</td>
            <td>{{ k.qtVersion }}</td>
            <td>{{ k.arch }} 位</td>
          </tr>
          <tr v-if="store.kits.length === 0">
            <td colspan="3" class="muted">点右上角「扫描套件」</td>
          </tr>
        </tbody>
      </table>
    </aside>
  </div>
</template>
