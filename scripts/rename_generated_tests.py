#!/usr/bin/env python3
"""将 Pynguin 生成的 test_case_n 模板名重命名为语义化名字。

用法:
    python rename_generated_tests.py <模块输出目录> <模块名>

例:
    python rename_generated_tests.py tests/generated/src_services_user src.services.user

命名规则:
  - 命中目标函数: test_<目标函数>_<该函数序号>   (test_add_0, test_add_1, test_sub_0)
  - 找不到目标:   test_<模块末段>_case_<序号>    (test_user_case_0)
  - 幂等: 只处理匹配 ^test_case_\\d+$ 的函数；手工改名/已改名的跳过。
  - 只改 def 名，函数体 / docstring / import / 顺序保持不变。
"""

import ast
import re
import sys
from pathlib import Path

TEMPLATE_NAME = re.compile(r"^test_case_\d+$")


def _iter_calls(node):
    """按源码顺序递归产出所有 Call 节点。"""
    for child in ast.iter_child_nodes(node):
        if isinstance(child, ast.Call):
            yield child
        else:
            yield from _iter_calls(child)


def _dotted_path(expr):
    """把 Attribute/Name 表达式还原为点分路径，如 src.services.user.add。"""
    parts = []
    cur = expr
    while isinstance(cur, ast.Attribute):
        parts.append(cur.attr)
        cur = cur.value
    if isinstance(cur, ast.Name):
        parts.append(cur.id)
    return ".".join(reversed(parts))


def detect_sut(module_name, tree):
    """返回 (aliases, direct_names)：被测模块在本文件中的绑定名。

    - `import src.services.user as module_0` -> aliases={module_0}
    - `import src.services.user`             -> aliases={src.services.user}
    - `from src.services.user import add`    -> direct_names={add}
    - `from src.services.user import *`      -> 无法解析，返回 (set(), set())
    """
    aliases = set()
    direct_names = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name == module_name:
                    aliases.add(alias.asname or alias.name)
        elif isinstance(node, ast.ImportFrom):
            if node.module == module_name:
                for alias in node.names:
                    if alias.name == "*":
                        return set(), set()
                    direct_names.add(alias.asname or alias.name)
    return aliases, direct_names


def target_function_of(body, aliases, direct_names, module_name):
    """返回测试体第一个「调用被测模块函数」的目标名；找不到返回 None。"""
    for stmt in body:
        for call in _iter_calls(stmt):
            path = _dotted_path(call.func)
            if not path:
                continue
            if path in direct_names:
                return path
            for alias in aliases:
                if path.startswith(alias + ".") and "." in path:
                    return path[len(alias) + 1 :]
            if path.startswith(module_name + ".") and "." in path:
                return path[len(module_name) + 1 :]
    return None


def _replace_def_name(lines, node, old, new):
    """在 def 语句所在行把函数名 token 替换掉；找不到则向后小范围扫描。"""
    start = node.lineno - 1
    for i in range(start, min(start + 5, len(lines))):
        if re.search(r"\bdef\s+" + re.escape(old) + r"\b", lines[i]):
            lines[i] = re.sub(r"\b" + re.escape(old) + r"\b", new, lines[i], count=1)
            return True
    return False


def rename_file(path, module_name):
    """重命名单个文件里匹配模板的函数，返回 [(旧名, 新名), ...]。"""
    src = path.read_text(encoding="utf-8")
    tree = ast.parse(src)
    aliases, direct_names = detect_sut(module_name, tree)
    if not aliases and not direct_names:
        return []

    used_names = {
        node.name
        for node in tree.body
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    }
    module_tail = re.sub(r"\W", "_", module_name.rsplit(".", 1)[-1]) or "module"
    per_target = {}
    fallback_idx = 0
    lines = src.splitlines(keepends=True)
    renames = []

    for node in tree.body:
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        old = node.name
        if not TEMPLATE_NAME.match(old):
            continue

        target = target_function_of(node.body, aliases, direct_names, module_name)
        if target:
            idx = per_target.get(target, 0)
            per_target[target] = idx + 1
            base = f"test_{target}_{idx}"
        else:
            base = f"test_{module_tail}_case_{fallback_idx}"
            fallback_idx += 1

        new = base
        n = 0
        while new in used_names:
            n += 1
            new = f"{base}_{n}"
        used_names.add(new)

        if _replace_def_name(lines, node, old, new):
            renames.append((old, new))

    if renames:
        path.write_text("".join(lines), encoding="utf-8")
    return renames


def main():
    if len(sys.argv) < 3:
        print("用法: rename_generated_tests.py <模块输出目录> <模块名>", file=sys.stderr)
        return 2
    out_dir = Path(sys.argv[1])
    module_name = sys.argv[2]
    if not out_dir.is_dir():
        print(f"目录不存在: {out_dir}", file=sys.stderr)
        return 1

    total = 0
    for py in sorted(out_dir.glob("test_*.py")):
        if py.name == "__init__.py":
            continue
        try:
            renames = rename_file(py, module_name)
        except Exception as exc:  # noqa: BLE001 - 单个文件失败不阻塞整体
            print(f"[warn] {py.name}: {exc}", file=sys.stderr)
            continue
        for old, new in renames:
            print(f"{py.name}: {old} -> {new}")
            total += 1
    print(f"renamed {total} test function(s) in {out_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
