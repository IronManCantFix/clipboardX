#!/usr/bin/env bash
# 自动递增 ClipboardX 版本号（patch/minor/major），并同步三个版本文件。
# 用法：
#   ./scripts/bump-version.sh            # 基于 GitHub 最新 release bump patch
#   ./scripts/bump-version.sh minor      # bump minor
#   ./scripts/bump-version.sh major      # bump major
#   ./scripts/bump-version.sh 1.2.3      # 指定具体版本
#   ./scripts/bump-version.sh --offline  # 不查询 GitHub，直接在本地版本上 bump
set -euo pipefail

REPO="IronManCantFix/clipboardX"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OFFLINE=0
KIND="patch"

for arg in "$@"; do
  case "$arg" in
    --offline) OFFLINE=1 ;;
    patch|minor|major) KIND="$arg" ;;
    *) KIND="$arg" ;;
  esac
done

current="$(python3 -c "import json;print(json.load(open('$ROOT/src-tauri/tauri.conf.json'))['version'])")"

if [[ "$OFFLINE" == "0" ]]; then
  latest_tag=""
  if command -v gh >/dev/null 2>&1; then
    latest_tag="$(gh release view --repo "$REPO" --json tagName --jq .tagName 2>/dev/null || true)"
  fi
  if [[ -z "$latest_tag" ]]; then
    latest_tag="$(curl -fsSL --max-time 15 \
      "https://api.github.com/repos/$REPO/releases/latest" \
      -H "User-Agent: ClipboardX" \
      | python3 -c "import json,sys;print(json.load(sys.stdin).get('tag_name',''))" 2>/dev/null || true)"
  fi
  latest="${latest_tag#v}"
else
  latest=""
fi

new_version="$(LATEST="$latest" CURRENT="$current" KIND="$KIND" python3 << 'PYEOF'
import os, re

def parse(v: str):
    m = re.match(r'^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$', v.strip())
    if not m:
        return None
    return tuple(int(x) for x in m.groups())

def fmt(v):
    return f"{v[0]}.{v[1]}.{v[2]}"

current = parse(os.environ["CURRENT"])
latest = parse(os.environ.get("LATEST", ""))
kind = os.environ["KIND"]

if re.fullmatch(r'\d+\.\d+\.\d+', kind):
    print(kind)
    raise SystemExit

base = current
if latest and latest > current:
    base = latest

if kind == "major":
    nv = (base[0] + 1, 0, 0)
elif kind == "minor":
    nv = (base[0], base[1] + 1, 0)
else:
    nv = (base[0], base[1], base[2] + 1)
print(fmt(nv))
PYEOF
)"

echo "当前版本: $current"
[[ -n "$latest" ]] && echo "GitHub 最新 release: v$latest"
echo "新版本: $new_version"

# 同步三处版本号
python3 - "$ROOT" "$new_version" << 'PYEOF'
import json, re, sys
root, ver = sys.argv[1], sys.argv[2]

# tauri.conf.json
p = f"{root}/src-tauri/tauri.conf.json"
cfg = json.load(open(p))
cfg["version"] = ver
json.dump(cfg, open(p, "w"), indent=2, ensure_ascii=False)
open(p, "a").write("\n")

# Cargo.toml
p = f"{root}/src-tauri/Cargo.toml"
lines = open(p).read().splitlines(keepends=True)
for i, line in enumerate(lines):
    if line.startswith("version ="):
        lines[i] = re.sub(r'version\s*=\s*"[^"]*"', f'version = "{ver}"', line)
        break
open(p, "w").writelines(lines)

# package.json
p = f"{root}/package.json"
pkg = json.load(open(p))
pkg["version"] = ver
json.dump(pkg, open(p, "w"), indent=2, ensure_ascii=False)
open(p, "a").write("\n")
PYEOF

echo "已同步 src-tauri/tauri.conf.json、src-tauri/Cargo.toml、package.json"
