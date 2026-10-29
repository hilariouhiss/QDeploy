# QDeploy

把「Qt 程序 → windeployqt 部署 → 生成 iss → ISCC 出安装包」这条手工链路，变成一次可重复、可检查、可批量执行的操作。

## 开发

    pnpm install
    pnpm tauri dev      # 开发窗口
    pnpm tauri build    # 出 QDeploy 自己的安装包
    pnpm check          # 格式 + lint
    cargo test --manifest-path src-tauri/Cargo.toml

## 界面

左上角的九格是这套工具的核心：**九个阶段**（预检 / 清空 / 拷 exe / 拷清单 / windeployqt / 排除 / 生成 iss / ISCC / 清理）。列表里它是一行极简段，详情里是九格时间轴加每格耗时；红色那格就是停下的地方。整个界面只有失败项带颜色。

字体随包（IBM Plex Sans / Plex Mono 的 woff2 在 `src/assets/fonts/`），不依赖 CDN。

窗口没有系统标题栏：顶部 44px 是自绘的标题栏（标记、名字、机器范围、最小化/最大化/关闭），拖动区和按钮分开，边缘仍然可缩放（窗口样式保留 WS_THICKFRAME）。
首帧由 `index.html` 直接画出底色与标记，不走 JS；任何渲染异常会被 `src/main.ts` 拦住并显示成错误面板，不会白屏。
应用图标是同一个 3×3 标记（`design/app-icon.png` 源图，`pnpm tauri icon` 生成全套）。

## 配置

单一 JSON：`%APPDATA%\QDeploy\config.json`，分两块：

- `machine`：这台机器的路径（Qt 安装根、ISCC.exe、暂存根），换电脑只改这里；
- `products`：每个产品一条（应用名、GUID、exe、套件、输出目录、增删清单）。

运行历史：`%APPDATA%\QDeploy\history.json`，每次运行一条，含九个阶段的耗时与状态，界面的「发版记录」和列表里的阶段条都读它。
日志按次落在 `%APPDATA%\QDeploy\logs\<时间戳>\<应用名>.log`。

## 一次执行

1. 预检：exe 能探、套件能解析、版本可得、路径通过安全守卫
2. 清空 `<暂存根>\<应用名>\` 与 `<暂存根>\.iss\<应用名>\`
3. 复制 exe、复制「添加文件」清单
4. 在暂存目录里跑 `windeployqt <额外参数> <应用名>.exe`（按 exe 自动识别 debug/release）
5. 按「移除文件」通配删掉多余文件
6. 生成 `<应用名>.iss`、写出内置的 `common_setup.iss`，调 ISCC
7. 成功（且本次包含打包）后删掉暂存与工作目录；失败或仅部署则保留，界面会标出路径

## 内置模板

安装脚本模板 `common_setup.iss` 编在 QDeploy 里（`src-tauri/templates/common_setup.iss`，`include_str!` 进 exe），界面里没有、也不接受外部路径。每次打包时 QDeploy 在工作目录里写两份文件再调 ISCC：`<应用名>.iss`（生成的 5 行 `#define` + 一行 include）和 `common_setup.iss`（模板原文，只为了给 ISPP 一个可 `#include` 的文件）。要改安装行为就改仓库里这个模板再重新构建 QDeploy。

模板只认 5 个 `#define`，值全部由 QDeploy 算好，模板不再自己推导默认值：

| define       | 值                                                                  |
| ------------ | ------------------------------------------------------------------- |
| `AppName`    | 应用名（= exe 名去掉 `.exe` = 暂存目录名）                          |
| `AppGuid`    | 36 位 GUID，不带花括号                                              |
| `AppVersion` | 短版本号（完整版本号去掉最后一段）；覆盖版本也按同一规则            |
| `SourceDir`  | 暂存目录绝对路径 `<暂存根>\<应用名>`，里面有 exe 与全部要打包的文件 |
| `OutputDir`  | 安装包输出目录绝对路径                                              |

ISCC 的产物必须叫 `<应用名> <短版本号> Installer.exe` 并落在 `OutputDir` 下，工具靠这个文件名判断本次是否成功。

生成的脚本长这样：

    #define AppName "<应用名>"
    #define AppGuid "<GUID，不带花括号>"
    #define AppVersion "<短版本号>"
    #define SourceDir "<暂存根>\<应用名>"
    #define OutputDir "<产品输出目录>"
    #include "common_setup.iss"

模板本身只处理 Inno 的安装行为（安装/卸载、桌面图标、中文界面、x64），不处理 VC 运行库：MinGW 运行库由部署阶段拷进暂存目录，MSVC 工具链不支持。无 BOM 的 UTF-8 脚本与 `x64compatible` 都要求 Inno Setup 6.3+。

## 边界

- 只做 64 位；MSVC 工具链未支持（界面也不提供）。
- exe 文件名必须等于 `<应用名>.exe`，因为模板按 AppName 拼 exe 路径。
- 版本号由 QDeploy 从 exe 版本资源读；读不到需在产品的「版本覆盖」里手填。
- 暂存根路径太浅（如 `C:\`）会被拒绝删除，这是故意的。
