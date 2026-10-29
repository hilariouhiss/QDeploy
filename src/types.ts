export interface MachineConfig {
  qtRoot: string;
  isccExe: string;
  stagingRoot: string;
  issTemplatePath: string;
}

export interface ManifestEntry {
  source: string;
  target: string;
}

export interface ProductConfig {
  appName: string;
  appId: string;
  exePath: string;
  kit: string;
  outputDir: string;
  windeployqtArgs: string[];
  versionOverride: string;
  include: ManifestEntry[];
  exclude: string[];
}

export interface Config {
  schemaVersion: number;
  machine: MachineConfig;
  products: ProductConfig[];
}

export interface Kit {
  id: string;
  qtVersion: string;
  arch: string;
  compiler: string;
  windeployqt: string;
}

export interface ProbeInfo {
  ok: boolean;
  error: string;
  archBits: number;
  qtMajor: number;
  compiler: string;
}

export interface StageRun {
  name: string;
  ms: number;
  state: string;
}

export interface RunRecord {
  appName: string;
  at: number;
  durationMs: number;
  ok: boolean;
  message: string;
  installer: string;
  version: string;
  stagingKept: string;
  stages: StageRun[];
}

export interface ProductRunEvent {
  appName: string;
  status: string;
  stage: number;
  message: string;
  stagingKept: string;
  installer: string;
}

export interface LogEvent {
  appName: string;
  line: string;
}

export type Stages = "deploy" | "package" | "both";

export const STAGE_NAMES = [
  "预检",
  "清空",
  "拷 exe",
  "拷清单",
  "windeployqt",
  "排除",
  "生成 iss",
  "ISCC",
  "清理",
];

export function emptyProduct(): ProductConfig {
  return {
    appName: "",
    appId: "",
    exePath: "",
    kit: "auto",
    outputDir: "",
    windeployqtArgs: [],
    versionOverride: "",
    include: [],
    exclude: [],
  };
}
