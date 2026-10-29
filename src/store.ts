import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  STAGE_NAMES,
  type Config,
  type Kit,
  type LogEvent,
  type ProductConfig,
  type ProductRunEvent,
  type RunRecord,
  type StageRun,
  type Stages,
} from "./types";

export interface Live {
  status: string;
  stage: number;
}

export const store = reactive({
  cfg: {
    schemaVersion: 1,
    machine: { qtRoot: "", isccExe: "", stagingRoot: "" },
    products: [],
  } as Config,
  history: [] as RunRecord[],
  kits: [] as Kit[],
  page: "products" as "products" | "history" | "settings",
  detail: false,
  current: -1,
  checked: {} as Record<string, boolean>,
  live: {} as Record<string, Live>,
  logs: {} as Record<string, string[]>,
  running: false,
  message: "",
});

export function currentProduct(): ProductConfig | null {
  return store.cfg.products[store.current] ?? null;
}

export async function loadConfig() {
  store.cfg = await invoke<Config>("load_config");
  const warning = await invoke<string | null>("take_load_warning");
  if (warning) store.message = warning;
}

export async function fetchHistory() {
  store.history = await invoke<RunRecord[]>("load_history");
}

export async function saveConfig(): Promise<boolean> {
  try {
    await invoke("save_config", { cfg: store.cfg });
    store.message = "已保存";
    return true;
  } catch (e) {
    store.message = "保存失败：" + String(e);
    return false;
  }
}

export async function scanKits() {
  store.kits = await invoke<Kit[]>("scan_kits", { qtRoot: store.cfg.machine.qtRoot });
}

export function lastRun(appName: string): RunRecord | null {
  for (const r of store.history) if (r.appName === appName) return r;
  return null;
}

/** Nine states for the ledger: none / done / now / fail. */
export function ledger(appName: string): StageRun[] {
  const live = store.live[appName];
  if (live && live.status === "running" && live.stage >= 0) {
    return STAGE_NAMES.map((name, i) => ({
      name,
      ms: 0,
      state: i < live.stage ? "done" : i === live.stage ? "now" : "none",
    }));
  }
  const rec = lastRun(appName);
  if (rec && rec.stages && rec.stages.length === STAGE_NAMES.length) return rec.stages;
  return STAGE_NAMES.map((name) => ({ name, ms: 0, state: "none" }));
}

export function ledgerLabel(appName: string): string {
  const live = store.live[appName];
  if (live && live.status === "running") return "运行中";
  const rec = lastRun(appName);
  if (!rec) return "从未发版";
  return rec.stagingKept ? "暂存保留" : "";
}

export function pad(n: number) {
  return n < 10 ? "0" + n : String(n);
}

export function formatWhen(unix: number): string {
  if (!unix) return "—";
  const d = new Date(unix * 1000);
  const now = new Date();
  const hm = pad(d.getHours()) + ":" + pad(d.getMinutes());
  const day = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const dd = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  if (dd === day) return "今天 " + hm;
  if (dd === day - 86400000) return "昨天 " + hm;
  return pad(d.getMonth() + 1) + "-" + pad(d.getDate()) + " " + hm;
}

export function formatMs(ms: number): string {
  if (!ms) return "";
  return ms >= 1000 ? (ms / 1000).toFixed(1) + " s" : ms + " ms";
}

export function checkedNames(): string[] {
  return store.cfg.products.filter((p) => store.checked[p.appName]).map((p) => p.appName);
}

export async function startRun(stage: Stages, names?: string[]) {
  const target = names && names.length ? names : checkedNames();
  if (target.length === 0) {
    store.message = "先勾选要跑的产品";
    return;
  }
  if (!(await saveConfig())) return;
  for (const n of target) store.logs[n] = [];
  store.running = true;
  store.message = "";
  try {
    await invoke("start_run", { appNames: target, stage });
  } catch (e) {
    store.running = await invoke<boolean>("is_running").catch(() => false);
    store.message = String(e);
  }
}

export async function cancelRun() {
  await invoke("cancel_run");
  store.message = "已请求停止：当前产品结束后不再继续";
}

export async function clearStaging() {
  const names = store.cfg.products.map((p) => p.appName);
  const msgs = await invoke<string[]>("clear_staging", { appNames: names });
  store.message = msgs.join("；");
}

export async function wireEvents() {
  await listen<ProductRunEvent>("run://product", async (e) => {
    const p = e.payload;
    const prev = store.live[p.appName];
    store.live[p.appName] = {
      status: p.status,
      stage: p.stage >= 0 ? p.stage : p.status === "running" ? -1 : prev ? prev.stage : -1,
    };
    if (p.status !== "running") await fetchHistory();
  });
  await listen<LogEvent>("run://log", (e) => {
    const p = e.payload;
    if (!store.logs[p.appName]) store.logs[p.appName] = [];
    store.logs[p.appName].push(p.line);
  });
  await listen<{ ok: number; failed: number; cancelled: number }>("run://done", (e) => {
    store.running = false;
    for (const [name, live] of Object.entries(store.live)) {
      if (live.status === "running") store.live[name] = { status: "failed", stage: live.stage };
    }
    const { ok, failed, cancelled } = e.payload;
    store.message = cancelled
      ? `已停止：成功 ${ok}，失败 ${failed}，未运行 ${cancelled}`
      : `完成：成功 ${ok}，失败 ${failed}`;
  });
}
