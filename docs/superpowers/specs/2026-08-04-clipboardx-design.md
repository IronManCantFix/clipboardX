# ClipboardX 设计文档

> macOS 现代化剪贴板管理器 · 极简 / 高性能 / 原生体验 / 完全本地化
> 日期：2026-08-04

## 1. 项目概述

ClipboardX 是一款专为 macOS 打造的轻量剪贴板管理器，定位类似 Maccy，但更强调**内存控制**与**原生体验**。

产品原则：
- 不联网、不收集数据、不做 AI、不依赖云服务
- 专注剪贴板管理，界面极简
- 内存占用是核心 KPI（对比 Maccy 长期使用后内存偏高的问题）

## 2. 技术选型

| 层 | 选型 | 理由 |
|---|---|---|
| 桌面框架 | **Tauri 2** | 不安装完整 Xcode（仅需 Command Line Tools），二进制小 |
| 后端 | **Rust** | 剪贴板监听、存储、粘贴模拟；内存可控 |
| 前端 | **Svelte 5** | 编译期框架、无虚拟 DOM、包体最小、内存最低 |
| 存储 | **SQLite**（rusqlite） | 单文件、事务、性能足够 |
| 全局快捷键 | tauri-plugin-global-shortcut | 默认 `Cmd+Shift+V`，可自定义 |

> 已知取舍：Tauri 的 WebView（WKWebView）常驻内存高于原生 Swift，通过"前端只持有必要数据 + 图片按需解码 + 轻量列表"策略将影响降到最低。

## 3. 系统架构（事件驱动分层）

```
┌────────────────────────────────────────────┐
│ Svelte 5 前端（WebView）                     │
│  弹窗视图（剪贴板/固定 Tab） · 设置弹窗 · 预览 │
└──────────────────┬─────────────────────────┘
                   │ Tauri IPC（invoke / events）
┌──────────────────▼─────────────────────────┐
│ Rust 核心                                   │
│  listener → store → commands                │
│  （search 由前端内存过滤承担）                │
└──────────────────┬─────────────────────────┘
                   │ NSPasteboard / NSWorkspace / CGEvent / 磁盘
```

### 模块职责

- **listener**：轮询 `NSPasteboard.changeCount`（间隔 500ms）；检测变化后读取内容并分类（图片 > RTF > 文件/URL > 文本）；排除应用过滤；与最近一条内容去重
- **store**：SQLite 读写（rusqlite，WAL 模式）；图片缩略图磁盘缓存；数量/时间自动清理
- **commands**：Tauri 命令桥，暴露 `list/search/pin/delete/paste/export/settings` 等
- **前端**：持有全量列表快照（≤100 条），本地实时过滤（<1ms），`clipboard:new-entry` 事件驱动刷新

### 核心数据流

1. 检测到剪贴板变化（changeCount 变化）
2. 读取内容 → 类型分类
3. 排除应用检查：前台应用在黑名单则跳过
4. 与最近一条内容去重，相同则跳过
5. 写入 SQLite；图片同时生成 512px 缩略图
6. 推送 `clipboard:new-entry` 事件 → 前端刷新列表（最近使用置顶）

### 粘贴实现

- 文本 / URL：写入 NSPasteboard 纯文本
- RTF：写入原始 RTF 数据（保留格式）
- 图片：写入 NSImage
- 文件：写入 file URLs
- 通过 CGEvent 模拟 `Cmd+V` 粘贴到前台应用，随后关闭弹窗

## 4. 数据模型

存储目录：`~/Library/Application Support/ClipboardX/`（`entries.db` + `images/` + `thumbs/`）

```sql
CREATE TABLE entries (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  type         TEXT NOT NULL,   -- text | rtf | image | file | url
  content      TEXT,            -- 纯文本预览；url 存地址；file 存路径
  rtf_data     BLOB,            -- RTF 原始数据（粘贴时还原格式）
  image_path   TEXT,            -- 原图路径（按需解码，不常驻内存）
  thumb_path   TEXT,            -- 512px 缩略图路径（列表展示）
  file_paths   TEXT,            -- 文件引用 JSON 数组
  source_app   TEXT,            -- 来源应用名（展示用）
  pinned       INTEGER DEFAULT 0,
  created_at   INTEGER,         -- 复制时间（清理策略依据）
  last_used_at INTEGER          -- 最近使用时间（排序依据）
);
CREATE INDEX idx_entries_sort ON entries(pinned DESC, last_used_at DESC);
CREATE INDEX idx_entries_type ON entries(type);
```

排序规则：`pinned DESC, last_used_at DESC`（固定置顶，其次最近使用）。

设置存储：`settings.json`（原子写）。

## 5. 核心功能设计

### 5.1 剪贴板监听
- 轮询 `changeCount`，500ms 间隔，变化才读内容
- 类型支持：Text / RichText(RTF) / Image / File / Folder / URL
- 分类优先级：Image > RTF > File/URL > Text（同一复制同时含多种类型时取优先级最高）
- GIF 按静态图处理

### 5.2 历史记录
- 默认保留 **15 条**，可配置，**最大 100 条**
- 超过上限自动删除最旧非固定条目
- 固定条目永不被自动清理、永不被数量上限淘汰

### 5.3 搜索
- 输入即过滤（实时），大小写不敏感、子串模糊匹配
- 匹配文字高亮
- 100 条量级前端内存过滤，目标 <50ms（实测预期 <1ms）
- 拼音搜索列为后续迭代（M3+）

### 5.4 快捷键
- 默认 `Cmd+Shift+V`，设置中录制式自定义
- 唤起弹窗于**鼠标位置**，靠近屏幕边缘自动翻转避让

### 5.5 图片
- 复制时生成 512px JPEG 缩略图（列表展示）
- 原图按原始格式存磁盘，仅粘贴/预览时解码
- 列表缩略图通过懒加载（IntersectionObserver）避免一次解码过多

### 5.6 固定（Pin）
- 右键/`⌘P` 固定或取消固定
- 固定项进入"固定"Tab；**无固定数据时隐藏"固定"Tab**
- "收藏"与"固定"合并为单一 Pin 概念

### 5.7 自动清理
- **数量模式**：达到条数上限删除最旧非固定条目
- **时间模式**：删除创建时间超过 N 天的非固定条目（7 天 / 30 天 / 永久）
- 固定条目不受任何自动清理影响

### 5.8 排除应用
- 黑名单（bundle id 列表）：1Password、KeePassXC、Bitwarden 等可配置
- 监听时读取前台应用 bundle id，命中黑名单则跳过记录
- 设置中支持"添加当前前台应用"一键加入

### 5.9 右键菜单
- **复制**：把该项写回剪贴板（不粘贴）
- **固定 / 取消固定**
- **导出**：文本 → .txt；RTF → .rtf；图片 → 原格式；文件 → 复制到所选目录
- **删除**：移除该条目（固定项需先取消固定或二次确认）

## 6. UI / UX 设计

### 6.1 弹窗（主界面）
- 快捷键触发，跟随鼠标位置；宽约 360px，圆角 12，毛玻璃（vibrancy 风格）
- 顶部：搜索框 + Tab（剪贴板 / 固定；无固定数据时仅显示"剪贴板"）
- 列表行：图片显示 30px 缩略图；文本/URL/文件显示类型小图标 + 单行省略文本；**不显示时间**
- 行高约 30px，选中态蓝色高亮，最近使用置顶排序
- 键盘：`↑↓` 选择 · `⏎` 粘贴并关闭 · `⌘P` 固定 · `⌫` 删除 · `Esc` 关闭 · `⌘1/⌘2` 切换 Tab
- 打开时自动聚焦搜索框
- 空状态：居中提示"暂无剪贴板记录"
- 底部：键盘操作提示条

### 6.2 设置弹窗
- 菜单栏图标点击弹出（含"退出 ClipboardX"入口）
- 分组：**常规**（历史条数滑块 5–100 默认 15；自动清理模式：数量/时间/永久）、**快捷键**（录制式）、**排除应用**（黑名单列表 + 添加当前前台应用）、**存储**（路径、占用量、"清空历史"保留固定项且二次确认）

## 7. 性能与内存控制

- 前端列表快照仅含必要字段，≤100 条
- 图片缩略图懒加载，原图绝不全量常驻内存
- Rust 侧不做无界缓存，数据即写即落盘
- 定期清理失效缩略图（文件已删、记录已清）
- M3 阶段做内存实测对比（对比 Maccy 与原生 Swift 基线）

## 8. 项目目录结构（规划）

```
ClipboardX/
├── src/                      # Svelte 5 前端
│   ├── routes/               # 弹窗视图 / 设置视图
│   ├── components/           # 条目行、缩略图、Tab、右键菜单
│   └── lib/                  # Tauri IPC 封装、stores、search
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # Tauri 入口、托盘
│   │   ├── listener.rs       # 剪贴板轮询、分类、排除、去重
│   │   ├── store.rs          # SQLite、缩略图缓存、清理
│   │   ├── paste.rs          # 写回剪贴板 + Cmd+V 模拟
│   │   └── commands.rs       # IPC 命令
│   ├── capabilities/         # Tauri 权限
│   └── Cargo.toml
└── docs/superpowers/specs/
```

## 9. 里程碑

- **M1 MVP**：脚手架（Tauri 2 + Svelte 5 + Rust 分层）；监听（文本/URL/文件）；SQLite；弹窗列表 + 搜索 + 键盘导航；粘贴（Cmd+V）；全局快捷键；设置（条数/快捷键）
- **M2 核心增强**：图片（缩略图/粘贴/预览）、RTF、固定 Tab、右键菜单、排除应用、自动清理
- **M3 打磨发布**：导出完善、拼音搜索、空状态/动画打磨、开机自启、DMG 打包、内存实测优化

## 10. 后续待定项（不阻塞开发）

- 拼音搜索的具体实现方案（词库 vs 拼音标注）
- 导出格式是否需要增加 markdown
- 是否提供深色/浅色主题切换（跟随系统即可）
