# AGENTS.md

给在本仓库动手的 agent 的说明书。人类向的介绍在 README.md，这里只写动手前必须知道的事。界面文案、错误信息、文档都用中文。

## 项目概览

QDeploy 是 Windows 桌面工具：把「Qt exe → windeployqt 部署 → 生成 iss → ISCC 出安装包」这条手工链路变成可重复、可检查、可批量执行的流水线。Tauri 2 应用，前端 Vue 3 + TypeScript + Vite（`src/`），后端 Rust（`src-tauri/src/`），两者只通过 Tauri command 与事件通信。`design/` 是界面 mockup 与图标源，不参与构建。

## 环境与首次启动

- Windows 10/11 x64；Node + pnpm 12.3.4（`packageManager` 固定）；Rust stable（仓库没有 toolchain pin）；Tauri 2 在 Windows 上的构建依赖（WebView2、VS Build Tools）。
- 安装：`pnpm install`（`pnpm-lock.yaml` 已提交）。
- 开发运行：`pnpm tauri dev`（先跑 `pnpm dev` 起 Vite，固定 1420 端口；窗口 1440×900、无系统标题栏）。
- 打 QDeploy 自己的安装包：`pnpm tauri build`（`beforeBuildCommand` 会先跑 `pnpm build`）。
- 应用配置不在仓库里：`%APPDATA%\QDeploy\config.json`（`machine` 是这台机器的路径，`products` 是产品列表），首次运行为空。

## 改完要跑的检查

| 场景                           | 命令                                                                                                              |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| 全仓检查（格式 + lint + 类型） | `pnpm check`（即 `vp check`；这就是门禁）                                                                         |
| 前端单个文件的格式与 lint      | `pnpm exec vp check src/views/ProductView.vue`（换成你改的文件）                                                  |
| 文档格式（只查 .md）           | `pnpm exec vp check --no-lint AGENTS.md`（只给 .md 路径时 lint 阶段会报“没有可 lint 的文件”，所以要 `--no-lint`） |
| 后端单测（全量）               | `cargo test --manifest-path src-tauri/Cargo.toml`                                                                 |
| 单个后端测试                   | `cargo test --manifest-path src-tauri/Cargo.toml probe::`（按名字过滤）                                           |
| 前端能否构建                   | `pnpm build`（即 `vp build`，产物进 `dist/`）                                                                     |

- `pnpm check`（即 `vp check`）跑格式 + lint + 类型检查，但本仓库没有配置 `lint.options.typeCheck`，也没装 vue-tsc——`.vue` 不做类型检查，类型错误只能靠编辑器或运行时发现。
- 前端没有测试文件（`pnpm test` 是 `vp test`，暂无用例）；前端改动靠 `pnpm tauri dev` 手测。
- 仓库没有 CI，上述本地检查就是全部门禁。
- `cargo fmt --check` 全仓本来就不干净（既有），不要顺手整体格式化 Rust；新代码跟所在文件的现有风格即可。

## 代码地图

| 位置                        | 职责                                                                                                                      |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/lib.rs`      | Tauri command 注册、`AppState`（配置 Mutex + running/cancel 标志）                                                        |
| `src-tauri/src/runner.rs`   | 九阶段流水线、`STAGES`、`Ticker` 计时、`ToolRunner` trait（生产用 `RealTools`，测试用 fake）                              |
| `src-tauri/src/batch.rs`    | 后台线程顺序跑多个产品，写日志，发 `run://product` / `run://log` / `run://done` 事件                                      |
| `src-tauri/src/probe.rs`    | 用 goblin 读 PE 导入表，判定 Qt 主版本、位数、编译器                                                                      |
| `src-tauri/src/kits.rs`     | 扫 `<qt_root>/<版本>/<kit>/bin/windeployqt.exe`，kit id = `<版本>-<compiler>-<arch>`                                      |
| `src-tauri/src/manifest.rs` | 暂存目录、添加/移除清单、复制与通配排除、路径越界守卫                                                                     |
| `src-tauri/src/iss.rs`      | 生成 `<应用名>.iss`（5 个 define + include 模板）；模板 `src-tauri/templates/common_setup.iss` 用 `include_str!` 编进 exe |
| `src-tauri/src/config.rs`   | `config.json` 读写与校验（应用名字符、36 位 GUID 等）                                                                     |
| `src-tauri/src/history.rs`  | `history.json`，保留最近 300 条                                                                                           |
| `src/store.ts`              | 前端唯一 reactive store；所有 invoke 与事件订阅集中在这                                                                   |
| `src/views/`                | `ProductsView`（列表）/ `ProductView`（详情）/ `HistoryView`（发版记录）/ `SettingsView`（设置）                          |
| `src/components/`           | `Ledger.vue` 九格阶段条、`ManifestEditor.vue` 文件清单                                                                    |

分层规则：

- 文件与进程操作只在 Rust 侧；前端不拼路径，一律走 command。加命令要同时改 `lib.rs` 的 `generate_handler!` 和前端 `invoke`（参数是 camelCase）。
- 九阶段是产品模型：改流程要同时改 `runner::STAGES`、`src/types.ts::STAGE_NAMES`，并核对 README 的「内置模板」。
- 配置结构在 `config.rs` 与 `src/types.ts` 一一对应，都靠 `serde(rename_all = "camelCase")` 和 `#[serde(default)]`；改字段两边一起改，并保证旧的 `config.json` 仍能读。
- 模板契约：`iss.rs::render` 给出的 5 个 define 与模板里的 `{#...}` 一一对应，改一边就同时改模板、`iss.rs` 和 README；`iss` 的单测会挡。

## 约定

- Rust：edition 2021；单测写在同一文件的 `#[cfg(test)] mod tests`；command 边界用 `Result<T, String>`，内部用 anyhow；错误文案是给用户看的中文。
- 前端：Vue 3 `<script setup lang="ts">`，2 空格、双引号、分号；样式以手写 CSS 类为主（`src/style.css` 里的 `.h2` / `.btn` / `.input` / `.mtable` 等），Tailwind 只做 `@import "tailwindcss"`；字体是随包 woff2，不要引 CDN。
- 界面克制：只有失败项带颜色。
- 首帧由 `index.html` 直接画出、`src/main.ts` 兜底错误面板；不要让启动界面依赖 JS 才能显示。
- 自绘标题栏：`decorations: false`，拖动区用 `data-tauri-drag-region`（见 `App.vue`）。

## 测试

- 后端单测与模块同文件；用 `tempfile` 建临时目录，`runner` 测试用 `FakeTools` 实现 `ToolRunner`，不真跑 windeployqt/ISCC；需要查看暂存结果时传 `RunCtx.cleanup = false`。
- 探测/解析类改动（`probe.rs`、`manifest.rs`、`iss.rs`）要加一个会在改坏时变红的单测，参考 `probe::tests::import_dlls_are_library_names`。
- 测试不要读写真实的 `%APPDATA%\QDeploy`（config/history/logs 都写在那里）。

## 提交

- Conventional Commits，英文小写，例：`fix(probe): classify by import DLL names, not symbols`、`feat(manifest): add files with one multi-select pick`。
- 一个逻辑改动一个 commit。
- 不要提交生成目录：`dist/`、`src-tauri/target/`、`src-tauri/gen/`（已在 `.gitignore`）。

## 危险区与陷阱

- 「清空」阶段会 `remove_dir_all` 掉 `<暂存根>\<应用名>` 和 `<暂存根>\.iss\<应用名>`。`manifest::staging_dir` 会拒绝层级过浅的暂存根（如 `C:\`），这是故意的，不要放宽；手动跑应用前确认暂存根指向你愿意删的位置。
- 真实运行会调 `windeployqt.exe`、`ISCC.exe`，并写 `%APPDATA%\QDeploy\{config,history}.json` 与 `logs\<时间戳>\<应用名>.log`。调试优先用 `cargo test` 里的 fake，不要为试验反复真跑。
- `probe.rs` 的 PE 导入项必须用 `Import.dll`（库名），不能回到 `Import.name`（函数符号）：用 `name` 会把所有 Qt exe 误判成「不是 Qt 程序」。
- 只支持 64 位；README 写明 MSVC 套件未支持，别往那个方向扩。
- `src-tauri/icons/` 是生成物但纳入版本库：改图标只动 `design/app-icon.png`（`design/gen-icon.cjs` 重画）再 `pnpm tauri icon` 生成整套，不要手改单个尺寸文件。
- `vite.config.ts` 忽略监听 `src-tauri/**`（`target` 会造成 EBUSY / 白屏），别去掉。
- 1420 端口是 strictPort：被占用时 `pnpm tauri dev` 直接失败。
- 不要把本机路径、用户名、密钥写进仓库文件或测试。
