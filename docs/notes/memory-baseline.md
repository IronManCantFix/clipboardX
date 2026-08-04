# ClipboardX 内存基线

> 记录时间：2026-08-04 · 测量方式：`ps -o rss`（常驻内存）

| 构建 | 常驻内存 (RSS) | 说明 |
|---|---|---|
| debug（M1） | ~86.8 MB | Tauri 2 + SvelteKit，dev 下含 Vite 热更新开销 |

## 对比参考

- Maccy（Swift 原生，同类产品）：长期使用后内存会持续增长（用户反馈的问题）
- 原生 Swift 空应用：约 20–50 MB
- Tauri WebView 空应用（release）：通常 60–120 MB

## 优化方向（M2/M3）

- release 构建 + 关闭 dev server 后复测（预期明显下降）
- 前端列表快照仅保留必要字段（已按设计实现：≤100 条）
- 图片缩略图懒加载 + 原图按需解码（M2 实现后复测）
- 监控长期运行后的内存漂移（对比 Maccy 的核心目标）
