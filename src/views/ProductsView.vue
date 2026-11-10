<script setup lang="ts">
import { computed, ref } from "vue";
import {
  store,
  ledger,
  lastRun,
  formatWhen,
  formatMs,
  checkedNames,
  startRun,
  cancelRun,
} from "../store";
import { emptyProduct } from "../types";
import Ledger from "../components/Ledger.vue";

const filter = ref<"all" | "todo" | "bad">("all");

const bad = computed(
  () => store.cfg.products.filter((p) => lastRun(p.appName)?.ok === false).length,
);
const checked = computed(() => checkedNames().length);
const never = computed(() => store.cfg.products.filter((p) => !lastRun(p.appName)).length);
const okCount = computed(
  () => store.cfg.products.filter((p) => lastRun(p.appName)?.ok === true).length,
);

const rows = computed(() =>
  store.cfg.products.filter((p) => {
    const r = lastRun(p.appName);
    if (filter.value === "todo") return !!store.checked[p.appName];
    if (filter.value === "bad") return r ? !r.ok : false;
    return true;
  }),
);

/** The one thing to notice: which product failed, at which stage, when. */
const finding = computed(() => {
  for (const p of store.cfg.products) {
    const r = lastRun(p.appName);
    if (r && !r.ok) {
      const stage = r.stages.find((s) => s.state === "fail");
      return { name: p.appName, stage: stage ? stage.name : "", when: formatWhen(r.at) };
    }
  }
  return null;
});

const lastAt = computed(() => {
  let at = 0;
  for (const p of store.cfg.products) {
    const r = lastRun(p.appName);
    if (r && r.at > at) at = r.at;
  }
  return at;
});

const note = computed(() => {
  if (filter.value === "todo") {
    return checked.value
      ? checked.value + " 个已勾选，按列表顺序依次执行"
      : "还没有勾选产品，勾选后可以批量运行";
  }
  if (filter.value === "bad") {
    return bad.value
      ? bad.value + " 个产品上次运行停在失败阶段，可单独重试"
      : "上次运行没有失败的产品";
  }
  return [
    store.cfg.products.length + " 个产品",
    checked.value ? checked.value + " 个已勾选待发" : "",
    lastAt.value ? "最近一次 " + formatWhen(lastAt.value) : "",
  ]
    .filter(Boolean)
    .join(" · ");
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

function railLabel(name: string) {
  const live = store.live[name];
  if (live && live.status === "running") return "运行中";
  const r = lastRun(name);
  if (!r) return "从未发版";
  if (!r.ok) {
    const stage = failedStage(name);
    return stage ? stage + " 失败" : "失败";
  }
  const ms = formatMs(r.durationMs);
  return ms || "完成";
}

function toggle(name: string) {
  store.checked[name] = !store.checked[name];
}

function open(i: number) {
  store.current = store.cfg.products.indexOf(rows.value[i]);
  store.detail = true;
}

/** Enter on a control inside the row must not also open the row. */
function onRowKey(e: KeyboardEvent, i: number) {
  if (e.target !== e.currentTarget) return;
  open(i);
}

function retry(name: string) {
  void startRun("package", [name]);
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
        {{ store.cfg.products.length }} 个产品<template v-if="checked">
          · {{ checked }} 个勾选待发</template
        >
        <template v-if="finding">
          ·
          <span class="neg">{{ finding.name }} {{ finding.when }} 在 {{ finding.stage }} 失败</span
          >，其余 {{ okCount }} 个上次成功
        </template>
        <template v-else-if="okCount"> · 上次运行全部成功</template>
        <template v-if="never"> · {{ never }} 个从未发版</template>
      </div>
    </div>
    <div class="actions">
      <template v-if="store.running">
        <button class="btn" @click="cancelRun">停止后续</button>
      </template>
      <template v-else>
        <button
          class="btn ghost"
          :disabled="checked === 0"
          :title="checked === 0 ? '先勾选要运行的产品' : '只跑 windeployqt 部署阶段'"
          @click="run('deploy')"
        >
          仅部署
        </button>
        <button
          class="btn"
          :disabled="checked === 0"
          :title="checked === 0 ? '先勾选要运行的产品' : '只用现有暂存目录打包'"
          @click="run('package')"
        >
          仅打包
        </button>
        <button
          class="btn primary"
          :disabled="checked === 0"
          :title="checked === 0 ? '先勾选要运行的产品' : '依次部署并用 Inno Setup 打包'"
          @click="run('both')"
        >
          部署并打包（{{ checked }}）
        </button>
      </template>
    </div>
  </div>

  <div class="tabs">
    <button class="tab" :class="{ on: filter === 'all' }" @click="filter = 'all'">
      全部<span class="n">{{ store.cfg.products.length }}</span>
    </button>
    <button class="tab" :class="{ on: filter === 'todo' }" @click="filter = 'todo'">
      待发<span class="n">{{ checked }}</span>
    </button>
    <button class="tab" :class="{ on: filter === 'bad' }" @click="filter = 'bad'">
      有问题<span class="n">{{ bad }}</span>
    </button>
  </div>

  <div class="toolbar">
    <div class="note">{{ note }}</div>
    <div class="spacer"></div>
    <button class="btn" @click="addProduct">+ 新建产品</button>
  </div>

  <div v-if="store.message" class="notice">
    <span class="dot"></span>
    <span>{{ store.message }}</span>
    <button class="close" title="清除提示" @click="store.message = ''">×</button>
  </div>

  <div class="table">
    <div class="thead">
      <div></div>
      <div>产品</div>
      <div class="r">版本</div>
      <div>上次运行</div>
      <div>阶段</div>
      <div class="end">操作</div>
    </div>

    <div
      v-for="(p, i) in rows"
      :key="p.appName"
      class="tr"
      tabindex="0"
      @click="open(i)"
      @keydown.enter="onRowKey($event, i)"
    >
      <button
        class="check"
        :class="{ on: store.checked[p.appName] }"
        role="checkbox"
        :aria-checked="!!store.checked[p.appName]"
        :aria-label="'勾选 ' + p.appName"
        @click.stop="toggle(p.appName)"
      ></button>
      <div class="cell c-name">
        <div class="pname" :title="p.appName">{{ p.appName }}</div>
        <div class="pex" :title="p.exePath">{{ exeName(p) }}</div>
      </div>
      <div class="cell c-ver ver">{{ versionOf(p.appName) }}</div>
      <div class="cell c-when when">
        <template v-if="lastRun(p.appName)">{{ formatWhen(lastRun(p.appName)!.at) }}</template>
        <template v-else><span class="muted">—</span></template>
      </div>
      <div class="cell c-rail">
        <Ledger
          :stages="ledger(p.appName)"
          :label="railLabel(p.appName)"
          :negative="!!lastRun(p.appName) && !lastRun(p.appName)!.ok"
        />
      </div>
      <div class="cell act">
        <button
          v-if="lastRun(p.appName) && !lastRun(p.appName)!.ok && !store.running"
          class="btn sm"
          title="用保留的暂存目录只重跑打包"
          @click.stop="retry(p.appName)"
        >
          重试
        </button>
      </div>
    </div>

    <div v-if="rows.length === 0" class="empty">
      <template v-if="store.cfg.products.length === 0">
        <div class="t">还没有产品</div>
        <div class="d">
          先在设置里填好 Qt 安装根、ISCC 和工作目录，再建第一个产品；之后每次发版都从这一页批量跑。
        </div>
        <div class="row">
          <button class="btn" @click="addProduct">+ 新建产品</button>
          <button class="btn ghost" @click="store.page = 'settings'">打开设置</button>
        </div>
      </template>
      <template v-else>
        <div class="t">这个筛选下没有产品</div>
        <div class="d">
          <template v-if="filter === 'todo'">还没有勾选产品。在「全部」里勾选要发的产品。</template>
          <template v-else>上次运行都成功了，没有需要处理的失败。</template>
        </div>
        <div class="row">
          <button class="btn" @click="filter = 'all'">
            看全部 {{ store.cfg.products.length }} 个
          </button>
        </div>
      </template>
    </div>
  </div>
</template>
