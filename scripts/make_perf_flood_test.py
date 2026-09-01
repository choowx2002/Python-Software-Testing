#!/usr/bin/env python3
"""生成 NFR003 取证用的"日志洪流" pytest 文件（heavy log streaming 触发器）。

用法:
    python scripts/make_perf_flood_test.py <项目目录> [每用例行数] [用例数]

默认 20 个用例 × 5000 行 = 10 万行日志（stdout/stderr 各一半），足以压满
Testmate 的流式日志通道，触发可观测的 emit 阻塞与延迟。

生成文件写入 <项目目录>/tests/test_perf_flood.py，之后在 Testmate 的
Execute 页选中该文件执行即可。执行前请先按 docs/perf-evidence-capture.md
设置 TESTMATE_PERF=1 并启动应用，终端会实时打印 [perf] emit 统计。
"""
import sys
from pathlib import Path


def main() -> None:
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)
    project = Path(sys.argv[1]).resolve()
    lines = int(sys.argv[2]) if len(sys.argv) > 2 else 5000
    count = int(sys.argv[3]) if len(sys.argv) > 3 else 20

    if lines <= 0 or count <= 0:
        print("每用例行数与用例数必须为正整数", file=sys.stderr)
        sys.exit(1)

    tests_dir = project / "tests"
    tests_dir.mkdir(parents=True, exist_ok=True)
    target = tests_dir / "test_perf_flood.py"

    parts = [
        "import sys\n",
        f"_LINES = {lines}\n",
        "\n",
        "def _flood():\n",
        "    chunk = ['flood line %d/%d' % (i, _LINES) for i in range(_LINES)]\n",
        "    # stdout / stderr 各一半，同时压两条流式通道\n",
        "    half = _LINES // 2\n",
        "    sys.stdout.write('\\n'.join(chunk[:half]) + '\\n')\n",
        "    sys.stderr.write('\\n'.join(chunk[half:]) + '\\n')\n",
        "    sys.stdout.flush()\n",
        "    sys.stderr.flush()\n",
        "\n",
    ]
    for i in range(count):
        parts.append(f"def test_flood_{i:03d}():\n")
        parts.append("    _flood()\n")
        parts.append("    assert True\n\n")

    target.write_text("".join(parts), encoding="utf-8")
    total = lines * count
    print(f"已生成 {target}")
    print(f"共 {count} 个用例 × 每用例 {lines} 行 = {total:,} 行日志（stdout/stderr 各 {total // 2:,} 行）")
    print("下一步：在 Testmate Execute 页选中该测试文件执行，观察终端 [perf] emit 统计。")


if __name__ == "__main__":
    main()
