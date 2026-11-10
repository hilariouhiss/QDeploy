<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
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
  cancelRun,
  openDir,
  copyText,
  openScripts,
} from "../store";
import type { ProbeInfo } from "../types";
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
const copied = ref(false);

const failedStage = computed(() => {
  const r = rec.value;
  if (!r) return "";
  const s = r.stages.find((x) => x.state === "fail");
  return s ? s.name : "";
});

async function copyLog() {
  if (!(await copyText(logText.value))) return;
  copied.value = true;
  setTimeout(() => (copied.value = false), 1200);
}

const probe = ref("");
/** 共享的安装包目录；最后一次运行没出安装包时也用它兜底打开。 */
const installerRoot = ref("");
onMounted(async () => {
  try {
    installerRoot.value = await invoke<string>("installer_dir");
  } catch {
    installerRoot.value = "";
  }
});

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

const argsText = computed({
  get: () => (p.value ? p.value.windeployqtArgs.join("\n") : ""),
  set: (v: string) => {
    if (p.value)
      p.value.windeployqtArgs = v
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
  },
});

const COMMON_ARGS: [string, string][] = [
  ["--no-translations", "不打包翻译"],
  ["--no-opengl-sw", "不打包 OpenGL"],
  ["--no-system-d3d-compiler", "不打包 D3D"],
  ["--no-webkit2", "不打包 WebKit2"],
  ["--no-quick-import", "不打包 Quick"],
];

const hasArg = (a: string) => p.value?.windeployqtArgs.includes(a) ?? false;

function toggleArg(a: string) {
  const list = p.value?.windeployqtArgs;
  if (!list) return;
  const i = list.indexOf(a);
  if (i >= 0) list.splice(i, 1);
  else list.push(a);
}

const history = computed(() =>
  store.history
    .filter((r) => r.appName === p.value?.appName)
    .slice(-5)
    .reverse(),
);
</script>

<template>
  <div v-if="!p" class="empty">
    <div class="t">没有选中产品</div>
    <div class="d">从产品列表里点一行，或先新建一个产品。</div>
    <div class="row"><button class="btn" @click="store.detail = false">回到产品列表</button></div>
  </div>

  <div v-else>
    <div class="crumb">
      <button @click="store.detail = false">产品</button>
      <span class="sep">/</span>
      <span>{{ p.appName || "(未命名)" }}</span>
    </div>

    <div class="titlerow">
      <div>
        <h1>{{ p.appName || "(未命名)" }}</h1>
        <div class="sub">
          {{ probe || "未识别 exe" }}
          <template v-if="rec">
            · 上次 {{ formatWhen(rec.at) }} {{ rec.ok ? "成功" : "失败" }} · 用时
            {{ formatMs(rec.durationMs) }}
          </template>
          <template v-else> · 还没跑过</template>
        </div>
      </div>
      <div class="actions">
        <template v-if="running || store.running">
          <button class="btn" @click="cancelRun">停止后续</button>
        </template>
        <template v-else>
          <button class="btn ghost" @click="startRun('deploy', [p.appName])">仅部署</button>
          <button class="btn" @click="startRun('package', [p.appName])">仅打包</button>
          <button class="btn primary" @click="startRun('both', [p.appName])">部署并打包</button>
        </template>
      </div>
    </div>

    <div v-if="store.message" class="notice">
      <span class="dot"></span>
      <span>{{ store.message }}</span>
      <button class="close" title="清除提示" @click="store.message = ''">×</button>
    </div>

    <div class="detail">
      <div class="main">
        <section class="panel">
          <div class="panel-head">
            <h2>{{ running ? "本次运行" : "最近一次运行" }}</h2>
            <span class="panel-note">
              <template v-if="rec && !running">
                {{ formatWhen(rec.at) }} · 用时 {{ formatMs(rec.durationMs)
                }}<template v-if="rec.stagingKept"> · 暂存保留</template>
              </template>
              <template v-else-if="running">进行中</template>
              <template v-else>还没跑过</template>
            </span>
            <button
              class="btn sm"
              style="margin-left: 10px"
              @click="openScripts(p.appName)"
              title="查看这个产品打包时写出的 iss"
            >
              查看 iss
            </button>
          </div>

          <div class="panel-body">
            <div class="stages">
              <div
                v-for="(s, i) in ledger(p.appName)"
                :key="i"
                class="stage"
                :class="s.state === 'now' ? 'run' : s.state"
              >
                <div class="bar"></div>
                <div class="nm">{{ s.name }}</div>
                <div class="ms">
                  {{
                    s.state === "fail"
                      ? "失败"
                      : s.state === "skipped"
                        ? "跳过"
                        : s.ms
                          ? formatMs(s.ms)
                          : ""
                  }}
                </div>
              </div>
            </div>

            <div v-if="rec && !rec.ok && !running" class="fail-line">
              <span class="msg">{{
                rec.message || failedStage + " 阶段失败，下面日志里有原因"
              }}</span>
              <button
                v-if="rec.stagingKept"
                class="btn"
                title="暂存目录还在，跳过部署直接重跑 ISCC"
                @click="startRun('package', [p.appName])"
              >
                仅打包重试
              </button>
            </div>

            <div v-if="rec && !running" class="outputs">
              <div v-if="rec.installer" class="output">
                <span class="k">安装包</span>
                <span class="v" :title="rec.installer">{{ rec.installer }}</span>
                <button class="btn sm" @click="openDir(rec.installer)">打开目录</button>
              </div>
              <div v-else-if="installerRoot" class="output">
                <span class="k">安装包目录</span>
                <span class="v" :title="installerRoot">{{ installerRoot }}</span>
                <button class="btn sm" @click="openDir(installerRoot)">打开目录</button>
              </div>
              <div v-if="rec.stagingKept" class="output">
                <span class="k">暂存</span>
                <span class="v" :title="rec.stagingKept">{{ rec.stagingKept }}</span>
                <button class="btn sm" @click="openDir(rec.stagingKept)">打开</button>
              </div>
            </div>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h2>日志</h2>
            <span class="panel-note">最近一次运行</span>
            <button class="btn sm" style="margin-left: 10px" @click="copyLog">
              {{ copied ? "已复制" : "复制" }}
            </button>
          </div>
          <div class="panel-body">
            <pre class="log">{{ logText }}</pre>
          </div>
        </section>

        <section class="panel">
          <div class="panel-head">
            <h2>发版记录</h2>
            <span class="panel-note">最近 {{ history.length }} 次</span>
          </div>
          <div class="panel-body">
            <table class="mtable hist">
              <thead>
                <tr>
                  <th class="n">版本</th>
                  <th>时间</th>
                  <th class="n">用时</th>
                  <th>结果</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in history" :key="r.at">
                  <td class="n p">{{ r.version || "—" }}</td>
                  <td>{{ formatWhen(r.at) }}</td>
                  <td class="n">{{ formatMs(r.durationMs) }}</td>
                  <td class="res" :class="{ fail: !r.ok }">
                    {{
                      r.ok
                        ? "成功"
                        : (r.stages.find((s) => s.state === "fail")?.name || "") + " 失败"
                    }}
                  </td>
                </tr>
                <tr v-if="history.length === 0">
                  <td colspan="4" class="muted">还没发过版</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
      </div>

      <aside class="aside">
        <section class="railsec">
          <div class="panel-head">
            <h2>配置</h2>
            <span class="panel-note">保存后生效</span>
          </div>
          <div class="panel-body">
            <label class="field">
              <span class="lbl">应用名</span>
              <input v-model="p.appName" class="input" />
            </label>
            <label class="field">
              <span class="lbl">GUID<span class="hint">不带花括号</span></span>
              <input v-model="p.appId" class="input mono" />
            </label>
            <label class="field">
              <span class="lbl">exe 路径</span>
              <div class="with-btn">
                <input v-model="p.exePath" class="input mono" />
                <button class="btn" @click="browseExe">浏览</button>
              </div>
            </label>
            <label class="field">
              <span class="lbl">工具链<span class="hint">按 exe 识别或手动指定</span></span>
              <select v-model="p.kit" class="select">
                <option value="auto">自动识别</option>
                <option v-for="k in store.kits" :key="k.id" :value="k.id">{{ k.id }}</option>
              </select>
            </label>
            <label class="field">
              <span class="lbl">版本覆盖<span class="hint">留空读 exe</span></span>
              <input v-model="p.versionOverride" class="input mono" placeholder="不覆盖" />
            </label>
            <label class="field">
              <span class="lbl">windeployqt 额外参数<span class="hint">一行一个</span></span>
              <div class="chips">
                <button
                  v-for="[a, name] in COMMON_ARGS"
                  :key="a"
                  class="chip"
                  :class="{ on: hasArg(a) }"
                  :title="a"
                  @click="toggleArg(a)"
                >
                  {{ name }}
                </button>
              </div>
              <textarea
                v-model="argsText"
                class="input mono"
                rows="2"
                placeholder="--no-translations"
              ></textarea>
            </label>
          </div>
        </section>

        <section class="railsec">
          <div class="panel-head">
            <h2>文件清单</h2>
            <span class="panel-note">
              添加 {{ p.include.length }} · 移除 {{ p.exclude.length }}
            </span>
          </div>
          <div class="panel-body">
            <ManifestEditor :product="p" />
          </div>
        </section>

        <div class="form-foot">
          <button class="btn" @click="saveConfig">保存配置</button>
          <span class="saved">{{ store.message === "已保存" ? "已保存" : "" }}</span>
        </div>
      </aside>
    </div>
  </div>
</template>
