#!/usr/bin/env python3
"""生成 tauri-plugin-updater 的 latest.json 更新清单。

用法: generate-update-manifest.py <tag> <app-tar-gz-path> <sig-path>
将清单 JSON 输出到 stdout（发布时作为 latest.json 上传到 GitHub Release）。

注意: 清单中的 URL 指向 `releases/latest/download/...`，该地址只有在 GitHub Release
发布后才能访问——CI 已改为创建即发布，因此流水线跑完即可被应用内「检查更新」拉到。
"""
import json
import sys
from datetime import datetime, timezone

tag, tgz_path, sig_path = sys.argv[1], sys.argv[2], sys.argv[3]
version = tag[1:] if tag.startswith("v") else tag
filename = tgz_path.rsplit("/", 1)[-1]
url = f"https://github.com/IronManCantFix/clipboardX/releases/latest/download/{filename}"
with open(sig_path, encoding="utf-8") as f:
    signature = f.read().strip()

manifest = {
    "version": version,
    "notes": "",
    "pub_date": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    "platforms": {
        # tauri-plugin-updater 识别的平台键（v1 兼容格式；写成 Rust target triple
        # 如 "aarch64-apple-darwin" 会报 "None of the fallback platforms ... found"）
        "darwin-aarch64": {
            "signature": signature,
            "url": url,
        }
    },
}
print(json.dumps(manifest, indent=2))
