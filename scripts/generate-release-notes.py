#!/usr/bin/env python3
"""根据 git 提交日志生成结构化 release notes。

用法：generate-release-notes.py <tag>

若仓库根目录存在 RELEASE_NOTES.md，则直接使用其内容（人工覆盖优先）；
否则从上一个 tag 到 HEAD 的提交中按 conventional-commit 类型分组生成。
"""

import subprocess
import sys
from collections import OrderedDict

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
    print(f"## ClipboardX {tag} 更新日志")
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
            ["git", "log", "--pretty=format:%B%x1f", f"{prev}..HEAD"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        print(f"**变更范围**：{prev} → {tag or 'HEAD'}")
    else:
        raw = subprocess.run(
            ["git", "log", "--pretty=format:%B%x1f", "-30"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        commits_count = len([m for m in raw.split("\x1f") if m.strip()])
        print(f"**变更范围**：最近 {commits_count} 条提交")
    print()

    grouped: dict[str, list[str]] = {k: [] for k in CATEGORIES}
    other: list[str] = []
    for message in raw.split("\x1f"):
        # %B 输出以换行结尾，与分隔符粘连后会残留前导换行
        message = message.strip("\n")
        if not message.strip():
            continue
        subject, bullets = parse_commit(message)
        # 有明细时展示明细，否则展示 subject
        for item in bullets if bullets else [subject]:
            cat, text = categorize(item)
            if cat:
                grouped[cat].append(text)
            else:
                other.append(text)

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


if __name__ == "__main__":
    main()
