#!/bin/bash
# ClipboardX dev 启动脚本：退出时自动释放 1420 端口并清理应用进程
# 用法：./scripts/dev.sh   （或 npm run dev:start）
set -e

cleanup() {
  echo "[dev.sh] cleaning up vite (1420) and clipboardx..."
  lsof -ti :1420 | xargs kill 2>/dev/null || true
  pkill -f 'target/debug/clipboardx' 2>/dev/null || true
}

trap cleanup EXIT INT TERM
npm run tauri dev
