<script setup lang="ts">
import { computed, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { store, scanKits, saveConfig, clearStaging, openDir } from "../store";

const kitInfo = ref("");
const confirming = ref(false);

const kitSummary = computed(() => {
  if (kitInfo.value) return kitInfo.value;
  return store.kits.length
    ? "已识别 " + store.kits.length + " 个套件"
    : "还没扫描，或 Qt 安装根下没有匹配的套件";
});

async function browseDir(field: "qtRoot" | "workRoot") {
  const d = await open({ directory: true, multiple: false });
  if (typeof d === "string") store.cfg.machine[field] = d;
}

async function browseIscc() {
  const f = await open({ multiple: false, filters: [{ name: "EXE", extensions: ["exe"] }] });
  if (typeof f === "string") store.cfg.machine.isccExe = f;
}

async function doScan() {
  await scanKits();
  kitInfo.value =
    store.kits.length === 0
      ? "一个套件都没扫到，检查 Qt 安装根"
      : "扫到 " + store.kits.length + " 个套件";
}

async function clearAll() {
  confirming.value = false;
  await clearStaging();
}
</script>

<template>
  <div class="titlerow">
    <div>
      <h1>设置</h1>
      <div class="sub">这台机器的路径。换电脑只改这一页。</div>
    </div>
    <div class="actions">
      <button class="btn primary" @click="saveConfig">保存</button>
    </div>
  </div>

  <div v-if="store.message" class="notice">
    <span class="dot"></span>
    <span>{{ store.message }}</span>
    <button class="close" title="清除提示" @click="store.message = ''">×</button>
  </div>

  <div class="settings">
    <section class="setting">
      <div class="setting-label">
        <h2>Qt 安装根</h2>
        <p>
          按 &lt;版本&gt;/&lt;套件&gt;/bin/windeployqt.exe
          扫描工具链。改动后重新扫描，产品页的工具链下拉才会更新。
        </p>
      </div>
      <div class="setting-panel">
        <div class="with-btn">
          <input v-model="store.cfg.machine.qtRoot" class="input mono" placeholder="C:\Qt" />
          <button class="btn" @click="browseDir('qtRoot')">浏览</button>
        </div>
        <div class="inline-note">
          <span>{{ kitSummary }}</span>
          <button class="btn" @click="doScan">扫描套件</button>
        </div>
        <table v-if="store.kits.length" class="mtable kits">
          <thead>
            <tr>
              <th>套件</th>
              <th>Qt</th>
              <th class="n">架构</th>
              <th>编译器</th>
              <th>windeployqt</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="k in store.kits" :key="k.id">
              <td class="p" data-label="套件">{{ k.id }}</td>
              <td data-label="Qt">{{ k.qtVersion }}</td>
              <td class="n" data-label="架构">{{ k.arch }} 位</td>
              <td data-label="编译器">{{ k.compiler }}</td>
              <td class="p trunc" data-label="windeployqt" :title="k.windeployqt">
                {{ k.windeployqt }}
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty-row" style="margin-top: 12px">
          扫描后会在这里列出识别到的套件。
        </div>
      </div>
    </section>

    <section class="setting">
      <div class="setting-label">
        <h2>ISCC.exe</h2>
        <p>Inno Setup 的命令行编译器。生成 iss 后由它出安装包。</p>
      </div>
      <div class="setting-panel">
        <div class="with-btn">
          <input
            v-model="store.cfg.machine.isccExe"
            class="input mono"
            placeholder="C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
          />
          <button class="btn" @click="browseIscc">浏览</button>
        </div>
      </div>
    </section>

    <section class="setting">
      <div class="setting-label">
        <h2>工作目录</h2>
        <p>
          部署与安装包都在这里，自动分成 deploy（部署）和
          installers（安装包）两个子目录；留空用安装目录下的默认位置。
        </p>
      </div>
      <div class="setting-panel">
        <div class="with-btn">
          <input
            v-model="store.cfg.machine.workRoot"
            class="input mono"
            :placeholder="store.defaultWorkRoot"
          />
          <button class="btn" @click="browseDir('workRoot')">浏览</button>
          <button class="btn" @click="openDir(store.cfg.machine.workRoot || store.defaultWorkRoot)">
            打开
          </button>
        </div>
      </div>
    </section>

    <section class="setting">
      <div class="setting-label">
        <h2 class="danger-title">清空暂存</h2>
        <p>删除工作目录下 deploy 与 .iss 里所有产品的目录，不可恢复；已经生成的安装包不受影响。</p>
      </div>
      <div class="setting-panel">
        <button v-if="!confirming" class="btn danger" @click="confirming = true">
          清空全部暂存
        </button>
        <div v-else class="confirm">
          <span
            >删除 {{ store.cfg.products.length }} 个产品的 deploy 与 .iss
            目录，不可恢复；安装包不受影响。</span
          >
          <button class="btn danger" @click="clearAll">确认清空</button>
          <button class="btn ghost" @click="confirming = false">取消</button>
        </div>
      </div>
    </section>
  </div>
</template>
