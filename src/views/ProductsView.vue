<script setup lang="ts">
import { computed, ref } from "vue";
import {
  store,
  ledger,
  ledgerLabel,
  lastRun,
  formatWhen,
  checkedNames,
  startRun,
  cancelRun,
  saveConfig,
} from "../store";
import { emptyProduct } from "../types";
import Ledger from "../components/Ledger.vue";

const filter = ref<"all" | "todo" | "bad">("all");

const bad = computed(
  () => store.cfg.products.filter((p) => lastRun(p.appName)?.ok === false).length,
);

const rows = computed(() =>
  store.cfg.products.filter((p) => {
    const r = lastRun(p.appName);
    if (filter.value === "todo") return !!store.checked[p.appName];
    if (filter.value === "bad") return r ? !r.ok : false;
    return true;
  }),
);

const finding = computed(() => {
  for (const p of store.cfg.products) {
    const r = lastRun(p.appName);
    if (r && !r.ok) {
      const stage = r.stages.find((s) => s.state === "fail");
      return { name: p.appName, when: formatWhen(r.at), stage: stage ? stage.name : "" };
    }
  }
  return null;
});

function exeName(p: { exePath: string }) {
  return p.exePath.split(/[\\/]/).pop() || p.exePath;
}

function versionOf(name: string) {
  const r = lastRun(name);
  return r && r.version ? r.version : "—";
}

function failedStage(name: string) {
  const r = lastRun(name);
  if (!r) return "";
  const s = r.stages.find((x) => x.state === "fail");
  return s ? s.name : "";
}

function toggle(name: string) {
  store.checked[name] = !store.checked[name];
}

function open(i: number) {
  store.current = store.cfg.products.indexOf(rows.value[i]);
  store.detail = true;
}

function addProduct() {
  const p = emptyProduct();
  p.appName = "Product" + (store.cfg.products.length + 1);
  p.appId = newGuid();
  store.cfg.products.push(p);
  store.current = store.cfg.products.length - 1;
  store.detail = true;
}

function newGuid(): string {
  if (crypto.randomUUID) return crypto.randomUUID().toUpperCase();
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx"
    .replace(/[xy]/g, (c) => {
      const r = (Math.random() * 16) | 0;
      return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
    })
    .toUpperCase();
}

const run = (s: "deploy" | "package" | "both") => startRun(s);
</script>

<template>
  <div class="titlerow">
    <div>
      <h1>产品</h1>
      <div class="sub">
        {{ store.cfg.products.length }} 个产品 · {{ checkedNames().length }} 个勾选待发 ·
        <template v-if="finding">
          <span class="neg">{{ finding.name }} {{ finding.when }} 失败（{{ finding.stage }}）</span
          >，其余最近都成功
        </template>
        <template v-else>最近都成功</template>
      </div>
    </div>
    <div class="actions">
      <template v-if="store.running">
        <button class="btn" @click="cancelRun">取消</button>
      </template>
      <template v-else>
        <button class="btn ghost" @click="run('deploy')">仅部署</button>
        <button class="btn" @click="run('package')">仅打包</button>
        <button class="btn primary" @click="run('both')">
          部署并打包（{{ checkedNames().length }}）
        </button>
      </template>
    </div>
  </div>

  <div class="tabs">
    <button class="tab" :class="{ on: filter === 'all' }" @click="filter = 'all'">
      全部<span class="n">{{ store.cfg.products.length }}</span>
    </button>
    <button class="tab" :class="{ on: filter === 'todo' }" @click="filter = 'todo'">
      待发<span class="n">{{ checkedNames().length }}</span>
    </button>
    <button class="tab" :class="{ on: filter === 'bad' }" @click="filter = 'bad'">
      有问题<span class="n">{{ bad }}</span>
    </button>
    <button class="btn ghost" style="margin-left: auto; margin-bottom: 6px" @click="addProduct">
      + 新建产品
    </button>
  </div>

  <div v-if="store.message" class="banner" style="margin-top: 12px">{{ store.message }}</div>

  <div class="table">
    <div class="th">
      <div></div>
      <div>产品</div>
      <div class="c-kit">工具链</div>
      <div style="text-align: right">版本</div>
      <div>上次发版</div>
      <div>阶段</div>
      <div></div>
    </div>

    <div
      v-for="(p, i) in rows"
      :key="p.appName"
      class="tr"
      :class="{ sel: store.checked[p.appName] }"
      @click="open(i)"
    >
      <button
        class="check"
        :class="{ on: store.checked[p.appName] }"
        @click.stop="toggle(p.appName)"
      ></button>
      <div class="cell">
        <div class="pname">{{ p.appName }}</div>
        <div class="pex mono">{{ exeName(p) }}</div>
      </div>
      <div class="cell c-kit kit mono">{{ p.kit === "auto" ? "自动识别" : p.kit }}</div>
      <div class="cell ver mono">{{ versionOf(p.appName) }}</div>
      <div class="cell when num">
        <template v-if="lastRun(p.appName)">
          <span v-if="lastRun(p.appName)!.ok">成功</span>
          <span v-else class="bad">失败</span>
          <small
            >{{ formatWhen(lastRun(p.appName)!.at)
            }}<template v-if="failedStage(p.appName)">
              · {{ failedStage(p.appName) }}</template
            ></small
          >
        </template>
        <template v-else><span class="muted">—</span><small>从未发版</small></template>
      </div>
      <div class="cell c-ledge">
        <Ledger :stages="ledger(p.appName)" :label="ledgerLabel(p.appName)" />
      </div>
      <div class="c-kebab kebab">···</div>
    </div>

    <div v-if="rows.length === 0" class="empty">这里还没有产品。点右上角「+ 新建产品」。</div>
  </div>
</template>
