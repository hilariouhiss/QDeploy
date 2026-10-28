<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import {
  store,
  currentProduct,
  ledger,
  lastRun,
  formatWhen,
  formatMs,
  saveConfig,
  startRun,
} from "../store";
import type { ProbeInfo } from "../types";
import Ledger from "../components/Ledger.vue";
import ManifestEditor from "../components/ManifestEditor.vue";

const p = computed(() => currentProduct());
const rec = computed(() => (p.value ? lastRun(p.value.appName) : null));
const live = computed(() => (p.value ? store.live[p.value.appName] : null));
const running = computed(() => !!live.value && live.value.status === "running");
const logText = computed(() => {
  const name = p.value?.appName ?? "";
  const lines = store.logs[name] ?? [];
  if (lines.length) return lines.join("\n");
  return rec.value
    ? rec.value.ok
      ? "成功：" + rec.value.installer
      : rec.value.message
    : "还没跑过。";
});
const probe = ref("");

async function refreshProbe() {
  if (!p.value || !p.value.exePath) {
    probe.value = "";
    return;
  }
  try {
    const info = await invoke<ProbeInfo>("probe_exe", { path: p.value.exePath });
    if (!info.ok) {
      probe.value = "无法识别：" + info.error;
      return;
    }
    const kit = await invoke<any>("resolve_kit", {
      exePath: p.value.exePath,
      kit: p.value.kit,
      qtRoot: store.cfg.machine.qtRoot,
    });
    probe.value =
      "Qt" + info.qtMajor + " / " + info.archBits + " 位 / " + info.compiler + " → " + kit.id;
  } catch (e) {
    probe.value = String(e);
  }
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

const kitLabel = computed(() =>
  p.value ? (p.value.kit === "auto" ? "自动识别" : p.value.kit) : "",
);
const history = computed(() =>
  store.history.filter((r) => r.appName === p.value?.appName).slice(0, 5),
);
</script>

<template>
  <div v-if="!p" class="empty">左侧选一个产品。</div>

  <div v-else>
    <div class="crumb">
      <button @click="store.detail = false">产品</button>
      <span>/</span>
      <span>{{ p.appName }}</span>
    </div>

    <div class="titlerow">
      <div>
        <h1>{{ p.appName || "(未命名)" }}</h1>
        <div class="sub">
          {{ probe || "未识别" }}
          <template v-if="rec">
            · 版本 {{ rec.version || "—" }} ·
            <span :class="rec.ok ? '' : 'neg'"
              >上次 {{ formatWhen(rec.at) }} {{ rec.ok ? "成功" : "失败" }}</span
            >
          </template>
          <template v-else> · 还没跑过</template>
        </div>
      </div>
      <div class="actions">
        <template v-if="running">
          <button class="btn" @click="store.running">运行中…</button>
        </template>
        <template v-else>
          <button class="btn ghost" @click="startRun('deploy', [p.appName])">仅部署</button>
          <button class="btn" @click="startRun('package', [p.appName])">仅打包</button>
          <button class="btn primary" @click="startRun('both', [p.appName])">部署并打包</button>
        </template>
      </div>
    </div>

    <div class="detail">
      <div>
        <section class="section">
          <div class="h2">
            {{ running ? "本次运行" : "上次运行" }}
            <span class="side">
              <template v-if="rec && !running"
                >{{ formatWhen(rec.at) }} · 用时 {{ formatMs(rec.durationMs)
                }}<template v-if="rec.stagingKept"> · 暂存已保留</template></template
              >
              <template v-else-if="running">进行中</template>
            </span>
          </div>
          <div class="steps">
            <div
              v-for="(s, i) in ledger(p.appName)"
              :key="i"
              class="st"
              :class="s.state === 'now' ? 'run' : s.state"
            >
              <div class="bar"></div>
              <div class="nm">{{ s.name }}</div>
              <div class="ms">
                {{
                  s.state === "fail"
                    ? "失败"
                    : s.ms
                      ? formatMs(s.ms)
                      : s.state === "skipped"
                        ? "跳过"
                        : ""
                }}
              </div>
            </div>
          </div>
        </section>

        <section class="section">
          <div class="h2">日志<span class="side">最近一次</span></div>
          <pre class="log">{{ logText }}</pre>
          <div v-if="rec && !rec.ok" class="sub" style="margin-top: 12px">{{ rec.message }}</div>
        </section>

        <section class="section">
          <div class="h2">
            发版记录<span class="side">最近 {{ history.length }} 次</span>
          </div>
          <table class="mtable hist">
            <thead>
              <tr>
                <th>版本</th>
                <th>时间</th>
                <th class="n">用时</th>
                <th>结果</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in history" :key="r.at">
                <td class="p">{{ r.version || "—" }}</td>
                <td>{{ formatWhen(r.at) }}</td>
                <td class="n">{{ formatMs(r.durationMs) }}</td>
                <td :style="r.ok ? '' : 'color:var(--negative)'">{{ r.ok ? "成功" : "失败" }}</td>
              </tr>
              <tr v-if="history.length === 0">
                <td colspan="4" class="muted">还没发过版</td>
              </tr>
            </tbody>
          </table>
        </section>
      </div>

      <aside>
        <section class="section">
          <div class="h2">配置</div>
          <div class="row2">
            <label class="field"
              ><span>应用名</span><input v-model="p.appName" class="input"
            /></label>
            <label class="field"
              ><span>GUID（不带花括号）</span><input v-model="p.appId" class="input mono"
            /></label>
          </div>
          <label class="field" style="margin-top: 14px">
            <span>exe 路径</span>
            <div class="with-btn">
              <input v-model="p.exePath" class="input mono" />
              <button class="btn" @click="browseExe">浏览</button>
            </div>
          </label>
          <div class="row2" style="margin-top: 14px">
            <label class="field">
              <span>工具链</span>
              <select v-model="p.kit" class="select">
                <option value="auto">自动识别</option>
                <option v-for="k in store.kits" :key="k.id" :value="k.id">{{ k.id }}</option>
              </select>
            </label>
            <label class="field"
              ><span>版本覆盖（留空读 exe）</span
              ><input v-model="p.versionOverride" class="input mono"
            /></label>
          </div>
          <label class="field" style="margin-top: 14px">
            <span>安装包输出目录</span>
            <div class="with-btn">
              <input v-model="p.outputDir" class="input mono" />
              <button class="btn" @click="browseOut">浏览</button>
            </div>
          </label>
          <label class="field" style="margin-top: 14px">
            <span>windeployqt 额外参数</span>
            <input v-model="argsText" class="input mono" placeholder="--no-translations" />
          </label>
          <div style="margin-top: 16px; display: flex; gap: 8px; justify-content: flex-end">
            <button class="btn" @click="saveConfig">保存</button>
          </div>
        </section>

        <section class="section">
          <ManifestEditor :product="p" />
        </section>
      </aside>
    </div>
  </div>
</template>
