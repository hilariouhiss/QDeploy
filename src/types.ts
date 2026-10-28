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
  versionFull: string;
  versionShort: string;
}

export type Stages = "deploy" | "package" | "both";

export type RunStatus = "pending" | "running" | "ok" | "failed";

export interface ProductRunEvent {
  appName: string;
  status: RunStatus;
  step: string;
  message: string;
  stagingKept: string;
  installer: string;
}

export interface LogEvent {
  appName: string;
  line: string;
}

export interface DoneEvent {
  ok: number;
  failed: number;
}

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
