#!/usr/bin/env python3
"""将 tag 版本号写入 Cargo.toml 与 tauri.conf.json（CI 打标签发布时自动注入）。

用法：bump-version.py <tag>          # 例如 v0.1.19

- 校验 tag 为 vX.Y.Z 格式（支持 pre-release / build 元数据，如 v0.1.19-beta.1）
- 把 version 同步写入两个文件（Tauri 要求两者一致，缺一会构建失败）：
    src-tauri/Cargo.toml       version = "x.y.z"
    src-tauri/tauri.conf.json  "version": "x.y.z"

本地开发不需要运行此脚本；CI 在打 v* 标签时自动注入。
"""

from __future__ import annotations

import json
import re
import sys

VERSION_RE = re.compile(r"v(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)")


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit("usage: bump-version.py <tag>  (e.g. v0.1.19)")
    tag = sys.argv[1]
    m = VERSION_RE.fullmatch(tag)
    if not m:
        sys.exit(f"invalid tag: {tag!r} (expected vX.Y.Z, e.g. v0.1.19)")
    version = m.group(1)

    # 1) Cargo.toml
    cargo_path = "src-tauri/Cargo.toml"
    with open(cargo_path, encoding="utf-8") as f:
        cargo = f.read()
    new_cargo, n = re.subn(r'(?m)^version\s*=\s*"[^"]+"', f'version = "{version}"', cargo, count=1)
    if n != 1:
        sys.exit(f"could not find version field in {cargo_path}")
    with open(cargo_path, "w", encoding="utf-8") as f:
        f.write(new_cargo)

    # 2) tauri.conf.json
    conf_path = "src-tauri/tauri.conf.json"
    with open(conf_path, encoding="utf-8") as f:
        conf = json.load(f)
    conf["version"] = version
    with open(conf_path, "w", encoding="utf-8") as f:
        json.dump(conf, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"version -> {version} (Cargo.toml + tauri.conf.json)")


if __name__ == "__main__":
    main()
