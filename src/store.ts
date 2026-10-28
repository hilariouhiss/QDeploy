import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Config, Kit, ProductConfig, Stages, ProductRunEvent, LogEvent } from "./types";

export interface Row {
  status: string;
  step: string;
  message: string;
  stagingKept: string;
  installer: string;
}

export const store = reactive({
  cfg: {
    schemaVersion: 1,
    machine: { qtRoot: "", isccExe: "", stagingRoot: "", issTemplatePath: "" },
    products: [],
  } as Config,
  kits: [] as Kit[],
  current: -1,
  checked: {} as Record<string, boolean>,
  rows: {} as Record<string, Row>,
  logs: {} as Record<string, string[]>,
  running: false,
  page: "product" as "product" | "settings",
  message: "",
});

export function currentProduct(): ProductConfig | null {
  return store.cfg.products[store.current] ?? null;
}

export async function loadConfig() {
  store.cfg = await invoke<Config>("load_config");
  if (store.cfg.products.length > 0 && store.current < 0) store.current = 0;
}

export async function saveConfig() {
  store.message = "";
  try {
    await invoke("save_config", { cfg: store.cfg });
    store.message = "已保存";
  } catch (e) {
    store.message = "保存失败：" + String(e);
  }
}

export async function scanKits() {
  store.kits = await invoke<Kit[]>("scan_kits", { qtRoot: store.cfg.machine.qtRoot });
}

export async function probeLine(p: ProductConfig): Promise<string> {
  if (!p.exePath) return "";
  try {
    const info = await invoke<any>("probe_exe", { path: p.exePath });
    if (!info.ok) return "无法识别：" + info.error;
    const kit = await invoke<any>("resolve_kit", {
      exePath: p.exePath,
      kit: p.kit,
      qtRoot: store.cfg.machine.qtRoot,
    });
    return (
      "识别为 Qt" +
      info.qtMajor +
      " / " +
      info.archBits +
      " 位 / " +
      info.compiler +
      " → " +
      kit.id +
      "（版本 " +
      info.versionFull +
      "）"
    );
  } catch (e) {
    return String(e);
  }
}

export function checkedNames(): string[] {
  return store.cfg.products.filter((p) => store.checked[p.appName]).map((p) => p.appName);
}

export async function startRun(stage: Stages) {
  const names = checkedNames();
  if (names.length === 0) {
    store.message = "先在左侧勾选要跑的产品";
    return;
  }
  for (const n of names) {
    store.rows[n] = { status: "pending", step: "", message: "", stagingKept: "", installer: "" };
    store.logs[n] = [];
  }
  store.running = true;
  store.message = "";
  try {
    await invoke("start_run", { appNames: names, stage });
  } catch (e) {
    store.running = false;
    store.message = String(e);
  }
}

export async function cancelRun() {
  await invoke("cancel_run");
}

export async function clearStaging() {
  const names = checkedNames();
  const target = names.length > 0 ? names : store.cfg.products.map((p) => p.appName);
  const msgs = await invoke<string[]>("clear_staging", { appNames: target });
  store.message = msgs.join("；");
}

export async function wireEvents() {
  await listen<ProductRunEvent>("run://product", (e) => {
    const p = e.payload;
    store.rows[p.appName] = {
      status: p.status,
      step: p.step,
      message: p.message,
      stagingKept: p.stagingKept,
      installer: p.installer ?? "",
    };
  });
  await listen<LogEvent>("run://log", (e) => {
    const p = e.payload;
    if (!store.logs[p.appName]) store.logs[p.appName] = [];
    store.logs[p.appName].push(p.line);
  });
  await listen<{ ok: number; failed: number }>("run://done", (e) => {
    store.running = false;
    store.message = "完成：成功 " + e.payload.ok + "，失败 " + e.payload.failed;
  });
}
