<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { copyText, store } from "../store";
import type { IssScript } from "../types";

const selected = ref(store.scriptApp);
const script = ref<IssScript | null>(null);
const error = ref("");
const busy = ref(false);
const copied = ref(false);
const codeEl = ref<HTMLPreElement | null>(null);

const selectedProduct = computed(
  () => store.cfg.products.find((p) => p.appName === selected.value) ?? null,
);
const workRoot = computed(() => store.cfg.machine.workRoot || store.defaultWorkRoot || "未设置");

async function load() {
  busy.value = true;
  error.value = "";
  script.value = null;
  try {
    script.value = await invoke<IssScript>("iss_script", {
      cfg: store.cfg,
      appName: selected.value || null,
    });
    await nextTick();
    if (codeEl.value) codeEl.value.scrollTop = 0;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}

function pick(name: string) {
  selected.value = name;
  store.scriptApp = name;
}

/** The fix lives on the product detail: select it there and let the form show the gap. */
function fixProduct() {
  const i = store.cfg.products.findIndex((p) => p.appName === selected.value);
  if (i < 0) return;
  store.current = i;
  store.detail = true;
  store.page = "products";
}

async function copy() {
  if (!script.value) return;
  if (await copyText(script.value.text)) {
    copied.value = true;
    setTimeout(() => (copied.value = false), 1200);
  }
}

watch(selected, () => void load());
onMounted(() => void load());
</script>

<template>
  <div class="titlerow">
    <div>
      <h1>安装脚本</h1>
      <div class="sub">
        内置模板和每个产品的 iss。产品脚本按当前配置生成，打包时写到工作目录的 .iss 下。
      </div>
    </div>
  </div>

  <div class="scripts">
    <div class="scriptlist">
      <div class="sgroup">
        <div class="group-label">内置模板</div>
        <button
          class="scriptitem"
          :class="{ on: selected === '' }"
          :aria-current="selected === '' ? 'true' : undefined"
          @click="pick('')"
        >
          <span class="sn">common_setup.iss</span>
          <span class="sd">安装行为模板</span>
        </button>
      </div>
      <div class="sgroup">
        <div class="group-label">产品脚本</div>
        <button
          v-for="p in store.cfg.products"
          :key="p.appName"
          class="scriptitem"
          :class="{ on: selected === p.appName }"
          :aria-current="selected === p.appName ? 'true' : undefined"
          @click="pick(p.appName)"
        >
          <span class="sn">{{ p.appName }}</span>
          <span class="sd mono">{{ p.appName }}.iss</span>
        </button>
        <div v-if="store.cfg.products.length === 0" class="empty-row">还没有产品</div>
      </div>
    </div>

    <section class="scriptpane" :aria-busy="busy">
      <div class="script-head">
        <div class="sh-main" aria-live="polite">
          <h2>{{ script?.title || (selected ? selected + ".iss" : "common_setup.iss") }}</h2>
          <div v-if="!selected" class="script-path">内置在 exe 里，打包时原样写到工作目录</div>
          <div v-else class="script-path">
            <template v-if="script">
              打包时写入 <span class="mono">{{ script.path }}</span>
            </template>
            <template v-else-if="busy">正在生成…</template>
            <template v-else>配置修好后才能生成路径</template>
          </div>
        </div>
        <button class="btn sm" :disabled="!script || busy" @click="copy">
          {{ copied ? "已复制" : "复制" }}
        </button>
      </div>

      <div v-if="error" class="script-error">
        <p class="err">{{ error }}</p>
        <p class="hint">
          生成 iss 需要能读到 exe，并从 exe 的版本资源或产品的版本覆盖里拿到版本号。
        </p>
        <dl v-if="selectedProduct" class="kv diag">
          <dt>应用名</dt>
          <dd>{{ selectedProduct.appName }}</dd>
          <dt>GUID</dt>
          <dd class="mono">{{ selectedProduct.appId }}</dd>
          <dt>exe</dt>
          <dd class="mono" :title="selectedProduct.exePath">
            {{ selectedProduct.exePath || "未设置" }}
          </dd>
          <dt>工具链</dt>
          <dd>{{ selectedProduct.kit === "auto" ? "自动识别" : selectedProduct.kit }}</dd>
          <dt>版本覆盖</dt>
          <dd>{{ selectedProduct.versionOverride || "未填" }}</dd>
          <dt>工作目录</dt>
          <dd class="mono" :title="workRoot">{{ workRoot }}</dd>
        </dl>
        <div class="err-actions">
          <button class="btn sm" @click="fixProduct">去产品页改</button>
          <button class="btn sm ghost" @click="pick('')">看内置模板</button>
          <button class="btn sm ghost" @click="load">重新生成</button>
        </div>
      </div>
      <pre v-else ref="codeEl" class="code">{{ script?.text ?? (busy ? "正在生成…" : "") }}</pre>

      <div v-if="script && selected && !error" class="script-help">
        <h3>脚本里的 5 个 define</h3>
        <dl class="kv">
          <dt>AppName</dt>
          <dd>应用名，决定 exe 名和暂存目录名</dd>
          <dt>AppGuid</dt>
          <dd>安装、卸载与同版本检测用的 GUID</dd>
          <dt>AppVersion</dt>
          <dd>短版本号，决定安装包文件名</dd>
          <dt>SourceDir</dt>
          <dd>要打包的暂存目录</dd>
          <dt>OutputDir</dt>
          <dd>安装包输出目录</dd>
        </dl>
      </div>
    </section>
  </div>
</template>
