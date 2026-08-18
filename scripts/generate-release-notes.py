#!/usr/bin/env python3
"""根据 git 提交日志生成结构化 release notes（含日期、提交数与短哈希）。

用法：generate-release-notes.py <tag>

若仓库根目录存在 RELEASE_NOTES.md，则直接使用其内容（人工覆盖优先）；
否则从上一个 tag 到 HEAD 的提交中按 conventional-commit 类型分组生成。
输出 markdown 到 stdout。
"""

from __future__ import annotations

import subprocess
import sys
from collections import OrderedDict
from datetime import datetime, timezone

REPO = "IronManCantFix/clipboardX"

CATEGORIES = OrderedDict(
    [
        ("✨ 新功能", ["feat", "feature"]),
        ("🐛 修复", ["fix"]),
        ("⚡ 性能优化", ["perf"]),
        ("🔒 安全加固", ["security", "safety"]),
        ("📄 文档", ["docs", "doc"]),
        ("🧹 维护与清理", ["chore", "refactor", "cleanup", "style", "test", "build", "ci"]),
    ]
)


def sh(*args: str) -> list[str]:
    out = subprocess.run(args, capture_output=True, text=True, check=True)
    return out.stdout.splitlines()


def parse_commit(message: str) -> tuple[str, list[str]]:
    """拆分提交信息为 (subject, body bullet 列表)。"""
    lines = message.splitlines()
    subject = lines[0].strip() if lines else ""
    body = [l.strip() for l in lines[1:] if l.strip()]
    bullets = [l[2:].strip() for l in body if l.startswith("- ")]
    return subject, bullets


def categorize(line: str) -> tuple[str | None, str]:
    """按类型前缀归类，返回 (类别, 去除前缀后的描述)。"""
    lowered = line.lower()
    for cat, prefixes in CATEGORIES.items():
        for p in prefixes:
            if lowered.startswith(p + ":"):
                return cat, line.split(":", 1)[1].strip()
    return None, line


def main() -> None:
    tag = sys.argv[1] if len(sys.argv) > 1 else ""
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    print(f"## ClipboardX {tag} 更新日志")
    print()
    print(f"> 发布时间：{today} ｜ 版本：`{tag}`")
    print()

    # 人工维护的 release notes 优先（存在时完全覆盖自动生成内容）
    try:
        with open("RELEASE_NOTES.md", encoding="utf-8") as f:
            manual = f.read().strip()
        if manual:
            print(manual)
            return
    except FileNotFoundError:
        pass

    # 定位上一个 tag（排除当前 tag，兼容 workflow_dispatch 重复构建同一版本号）；
    # 找不到时回退到最近 30 条提交
    prev = None
    try:
        prev = sh("git", "describe", "--tags", "--abbrev=0", "--exclude", tag, "HEAD~1")[0].strip()
    except subprocess.CalledProcessError:
        pass

    if prev:
        raw = subprocess.run(
            ["git", "log", "--pretty=format:%h%x1f%B%x1e", f"{prev}..HEAD"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        scope = f"{prev} → {tag or 'HEAD'}"
    else:
        raw = subprocess.run(
            ["git", "log", "--pretty=format:%h%x1f%B%x1e", "-30"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        scope = "最近 30 条提交"

    commits = [c for c in raw.split("\x1e") if c.strip()]
    print(f"**变更范围**：{scope}（共 {len(commits)} 条提交）")
    print()

    grouped: dict[str, list[str]] = {k: [] for k in CATEGORIES}
    other: list[str] = []
    for commit in commits:
        parts = commit.strip("\n").split("\x1f", 1)
        sha = parts[0].strip()
        message = parts[1].strip("\n") if len(parts) > 1 else ""
        if not message:
            continue
        subject, bullets = parse_commit(message)
        # 有明细时展示明细，否则展示 subject；均附带短哈希便于追溯
        for item in bullets if bullets else [subject]:
            cat, text = categorize(item)
            entry = f"{text}（`{sha}`）"
            if cat:
                grouped[cat].append(entry)
            else:
                other.append(entry)

    printed = False
    for cat, items in grouped.items():
        if not items:
            continue
        printed = True
        print(f"### {cat}")
        for item in items:
            print(f"- {item}")
        print()
    if other:
        printed = True
        print("### 其他")
        for item in other:
            print(f"- {item}")
        print()
    if not printed:
        print("- 无显著变更")
        print()

    # 完整提交对比链接（仅有 prev 时才有意义）
    if prev:
        print(f"---")
        print(f"[查看完整提交记录](https://github.com/{REPO}/compare/{prev}...{tag})")


if __name__ == "__main__":
    main()
