#!/usr/bin/env bash
# 发布 ClipboardX 新版本：自动 bump 版本 → 构建 → 生成 latest.json → 发布 GitHub release
#
# 用法：
#   ./scripts/release.sh                 # bump patch 并发布
#   ./scripts/release.sh minor           # bump minor 并发布
#   ./scripts/release.sh major           # bump major 并发布
#   ./scripts/release.sh 1.2.0           # 用指定版本发布
#   ./scripts/release.sh --append        # 不 bump，仅把当前平台的资产/签名追加到已发布的 release
#   ./scripts/release.sh --dry-run       # 只构建 + 生成 latest.json，不上传到 GitHub
#
# 前置条件：
#   - 已登录 GitHub CLI（gh auth login）或设置 GH_TOKEN
#   - 更新签名私钥：~/.tauri/clipboardx.key（或 TAURI_SIGNING_PRIVATE_KEY 环境变量）
set -euo pipefail

REPO="IronManCantFix/clipboardX"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

APPEND=0
DRY_RUN=0
ARGS=()
for arg in "$@"; do
  case "$arg" in
    --append) APPEND=1 ;;
    --dry-run) DRY_RUN=1 ;;
    *) ARGS+=("$arg") ;;
  esac
done

# 1. 版本号
if [[ "$APPEND" == "1" ]]; then
  echo "==> --append 模式：沿用当前版本，不执行 bump"
else
  ./scripts/bump-version.sh "${ARGS[@]:-patch}"
fi
VERSION="$(python3 -c "import json;print(json.load(open('src-tauri/tauri.conf.json'))['version'])")"
TAG="v$VERSION"
echo "==> 本次发布版本：${VERSION}（tag: ${TAG}）"

# 发布需要 updater 签名产物，确保配置已开启
CREATE_UPDATER="$(python3 -c "import json;print(json.load(open('src-tauri/tauri.conf.json')).get('bundle',{}).get('createUpdaterArtifacts',False))")"
if [[ "$CREATE_UPDATER" != "True" && "$CREATE_UPDATER" != "true" ]]; then
  echo "!! 请先将 src-tauri/tauri.conf.json 的 bundle.createUpdaterArtifacts 改为 true（发布后 CI 只构建 app/dmg，updater 增量包由本脚本产出）" >&2
  exit 1
fi

# 2. 签名密钥（tauri-cli 只认 TAURI_SIGNING_PRIVATE_KEY 环境变量）
if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  if [[ -f "$HOME/.tauri/clipboardx.key" ]]; then
    export TAURI_SIGNING_PRIVATE_KEY="$(cat "$HOME/.tauri/clipboardx.key")"
  else
    echo "!! 未找到更新签名私钥，请设置 TAURI_SIGNING_PRIVATE_KEY 或准备 ~/.tauri/clipboardx.key" >&2
    exit 1
  fi
fi
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" 

# 3. 构建（含 updater artifacts 与签名）
echo "==> 开始构建..."
npm run tauri build

# 4. 收集当前平台的 updater 资产
case "$(uname -s)" in
  Darwin)
    case "$(uname -m)" in
      arm64) TARGET="darwin-aarch64" ;;
      x86_64) TARGET="darwin-x86_64" ;;
      *) echo "!! 不支持的 macOS 架构：$(uname -m)" >&2; exit 1 ;;
    esac
    BUNDLE_DIR="$ROOT/src-tauri/target/release/bundle/macos"
    ;;
  Linux)
    case "$(uname -m)" in
      x86_64) TARGET="linux-x86_64" ;;
      aarch64) TARGET="linux-aarch64" ;;
      *) echo "!! 不支持的 Linux 架构：$(uname -m)" >&2; exit 1 ;;
    esac
    BUNDLE_DIR="$ROOT/src-tauri/target/release/bundle/appimage"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    case "$(uname -m)" in
      x86_64) TARGET="windows-x86_64" ;;
      aarch64) TARGET="windows-aarch64" ;;
      *) echo "!! 不支持的 Windows 架构：$(uname -m)" >&2; exit 1 ;;
    esac
    BUNDLE_DIR="$ROOT/src-tauri/target/release/bundle/nsis"
    ;;
  *)
    echo "!! 不支持的操作系统：$(uname -s)" >&2
    exit 1
    ;;
esac

UPDATER_BIN=""
UPDATER_SIG=""
shopt -s nullglob
for a in "$BUNDLE_DIR"/*.sig; do
  UPDATER_SIG="$a"
done
for a in "$BUNDLE_DIR"/*.app.tar.gz "$BUNDLE_DIR"/*.AppImage.tar.gz "$BUNDLE_DIR"/*-setup.exe; do
  [[ -f "$a" ]] && UPDATER_BIN="$a"
done
if [[ -z "$UPDATER_BIN" || -z "$UPDATER_SIG" ]]; then
  echo "!! 未找到 updater 资产（期望二进制 + .sig），请检查构建产物" >&2
  echo "   BIN=$UPDATER_BIN SIG=$UPDATER_SIG" >&2
  exit 1
fi
BIN_NAME="$(basename "$UPDATER_BIN")"
echo "==> updater 资产："
echo "   bin: $UPDATER_BIN"
echo "   sig: $UPDATER_SIG"

# 5. 生成 latest.json（--append 时合并已发布的 latest.json）
LATEST_JSON="$ROOT/latest.json"
if [[ "$APPEND" == "1" && "$DRY_RUN" == "0" ]]; then
  echo "==> 追加模式：从 GitHub release 下载 latest.json 合并"
  gh release download "$TAG" -p latest.json -R "$REPO" --dir "$ROOT" --clobber 2>/dev/null || true
fi

SIGNATURE="$(cat "$UPDATER_SIG")"
PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
TARGET="$TARGET" SIGNATURE="$SIGNATURE" PUB_DATE="$PUB_DATE" BIN_NAME="$BIN_NAME" \
python3 - "$LATEST_JSON" "$REPO" "$TAG" "$VERSION" << 'PYEOF'
import json, os, sys

latest_path, repo, tag, version = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
target = os.environ["TARGET"]

data = {}
if os.path.exists(latest_path):
    with open(latest_path) as f:
        data = json.load(f)

data["version"] = version
data["notes"] = data.get("notes", "")
data["pub_date"] = os.environ["PUB_DATE"]
platforms = data.setdefault("platforms", {})
platforms[target] = {
    "url": f"https://github.com/{repo}/releases/download/{tag}/{os.environ['BIN_NAME']}",
    "signature": os.environ["SIGNATURE"],
}

with open(latest_path, "w") as f:
    json.dump(data, f, indent=2, ensure_ascii=False)
    f.write("\n")
print(f"==> latest.json 已更新（{target}），包含平台：{', '.join(platforms)}")
PYEOF

if [[ "$DRY_RUN" == "1" ]]; then
  echo "==> --dry-run：跳过 GitHub 发布。latest.json 位于 $LATEST_JSON"
  exit 0
fi

# 6. 发布到 GitHub
if ! command -v gh >/dev/null 2>&1; then
  echo "!! 未安装 GitHub CLI。请安装 gh 后重试，或手动执行：" >&2
  echo "   gh release create $TAG -R $REPO --generate-notes" >&2
  echo "   gh release upload $TAG -R $REPO '$UPDATER_BIN' '$UPDATER_SIG' '$LATEST_JSON' --clobber" >&2
  exit 1
fi

if gh release view "$TAG" -R "$REPO" >/dev/null 2>&1; then
  echo "==> release $TAG 已存在，追加/覆盖资产"
else
  echo "==> 创建 release $TAG"
  gh release create "$TAG" -R "$REPO" --title "ClipboardX v$VERSION" --generate-notes
fi

gh release upload "$TAG" -R "$REPO" "$UPDATER_BIN" "$UPDATER_SIG" "$LATEST_JSON" --clobber
echo "==> 发布完成：https://github.com/$REPO/releases/tag/$TAG"
