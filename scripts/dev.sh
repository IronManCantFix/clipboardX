#!/bin/bash
# ClipboardX dev 启动脚本：退出时自动释放 1420 端口并清理应用进程
# 用法：./scripts/dev.sh   （或 npm run dev:start）
# dev 版本使用 tauri.dev.conf.json 覆盖 identifier/productName，
# 与 release 版本的辅助功能授权完全分开。
set -e

cleanup() {
  echo "[dev.sh] cleaning up vite (1420) and clipboardx..."
  lsof -ti :1420 | xargs kill 2>/dev/null || true
  pkill -f 'target/debug/clipboardx' 2>/dev/null || true
}

trap cleanup EXIT INT TERM
npx tauri dev --config src-tauri/tauri.dev.conf.json
