# ClipboardX

![Build](https://github.com/IronManCantFix/clipboardX/actions/workflows/build.yml/badge.svg)

专为 macOS 打造的现代化剪贴板管理器 —— 原生体验、完全离线、低内存占用。

[English](README.en.md)

## 功能特性

- **全局快捷键** —— 任意位置按 `⌘⇧V` 唤起弹窗（可自定义）
- **剪贴板历史** —— 文本、URL、文件、文件夹、图片（PNG/JPEG/GIF/TIFF/HEIC）
- **智能去重** —— 重复复制相同内容自动移到最前，不产生重复记录
- **实时搜索** —— 模糊/精准匹配、忽略大小写，响应 <50ms
- **固定（Pin）** —— 重要内容置顶，固定项永不被自动清理
- **图片支持** —— 列表内缩略图预览，原图按需加载不占内存
- **屏幕自适应定位** —— 弹窗跟随鼠标，自动避开屏幕边缘
- **自动清理** —— 按条数（默认 15，最大 100），固定项受保护
- **菜单栏应用** —— 无 Dock 图标，支持开机自启
- **完全本地化** —— 不联网、不收集数据、无云服务

## 技术栈

- [Tauri 2](https://tauri.app) + Rust
- [Svelte 5](https://svelte.dev)（SvelteKit）
- [SQLite](https://sqlite.org)（rusqlite）
- [Vite](https://vite.dev)

## 开发

环境要求：Node.js ≥ 20、Rust stable、Xcode Command Line Tools。

```bash
# 安装前端依赖
npm install

# 启动开发（退出时自动释放 1420 端口）
npm run dev:start

# 端口被占时手动清理
npm run dev:stop
```

## 构建

```bash
# 构建 macOS 应用（Apple 芯片）
npm run tauri build

# 产物位置：src-tauri/target/release/bundle/
```

## CI 构建

GitHub Actions 会在推送 `v*` 标签时自动构建 macOS（Apple 芯片）应用，并把构建产物（`.dmg` / `.app.zip`）上传到 GitHub Releases 的**草稿**中，见 [`.github/workflows/build.yml`](.github/workflows/build.yml)。在 Release 页面发布草稿后即可供下载。

## macOS 安装（未签名应用）

本仓库构建的应用**没有 Apple Developer ID 签名**（签名需要付费的 Apple 开发者账号），也未经过公证（notarization），因此首次打开会被 Gatekeeper 拦截。按以下任一方式处理：

- **方式一（推荐）：右键 → 打开** — 在访达中按住 Control 键（或右键）点击 `ClipboardX.app`，选择**打开**，再点击对话框中的**打开**。
- **方式二：系统设置** — 先双击被拦截一次，再进入**系统设置 → 隐私与安全性**，找到 ClipboardX 的提示，点击**仍要打开**。
- **方式三：命令行移除隔离属性** — 执行一次以下命令，之后即可正常双击打开：

  ```bash
  xattr -dr com.apple.quarantine /Applications/ClipboardX.app
  ```

如果提示「应用已损坏，无法打开」（通常是下载不完整或签名被剥离），请重新下载，或执行：

```bash
xattr -cr /Applications/ClipboardX.app
```

> 请只安装可信来源的应用。无需关闭整个 Gatekeeper（`spctl --master-disable`）。

## 辅助功能授权（自动粘贴必需）

ClipboardX 通过模拟 `⌘V` 实现自动粘贴，需要 macOS 辅助功能权限。首次粘贴时系统会自动弹出授权对话框。

授权步骤：

1. 首次点击剪贴板条目时，系统弹出辅助功能授权对话框
2. 进入**系统设置 → 隐私与安全性 → 辅助功能**
3. 找到 ClipboardX 并打开开关
4. **完全退出 ClipboardX**（菜单栏图标 → 退出）后重新打开
5. 再次粘贴即可正常工作

> **dev 版本与 release 版本授权独立**：开发版显示为「ClipboardX Dev」，正式版显示为「ClipboardX」，两者互不干扰。如果辅助功能列表中有同名的旧条目，请先移除（点减号删除），再重新添加新版本。

## 快捷键

| 操作 | 快捷键 |
|---|---|
| 唤起弹窗 | `⌘⇧V`（可自定义） |
| 粘贴并关闭 | `⏎` |
| 上下选择 | `↑` / `↓` |
| 固定 / 取消固定 | `⌘P` |
| 删除 | `⌫` |
| 切换标签 | `⌘1` / `⌘2` |
| 关闭 | `Esc` |

## 设置

通过托盘图标或弹窗右上角齿轮打开设置：

- 历史条数上限、自动清理
- 外观：透明度、窗口尺寸（自动/自定义）、图片预览高度
- 搜索：开关、模糊/精准模式
- 位置：跟随鼠标 / 屏幕居中
- 快捷键录制
- 开机自启
- 存储统计 + 清除图片缓存
- 检查更新

## 数据存储

所有数据均保存在本地：

```
~/Library/Application Support/com.clipboardx.app/
├── entries.db      # 历史记录数据库
└── images/         # 捕获的图片（PNG）
```

## 许可证

[MIT](LICENSE)
