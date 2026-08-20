import os

# 读取文件
with open('src/commands.rs', 'r') as f:
    lines = f.readlines()

# 保留在commands.rs中的范围（1-based, inclusive）
commands_ranges = [
    (1, 74),      # imports + 顶部数据结构
    (607, 647),   # Test events/result (TestOutputEvent, TestStartedEvent, TestResult, TestFinishedEvent)
    (842, 850),   # RegressionSuite
    (1227, 1307), # build_python_path + Generation数据结构
    (1927, 2008), # Coverage数据结构
]

# 模块区域（1-based, inclusive）
modules = {
    'env': [(75, 330), (1781, 1835)],
    'execution': [(331, 606), (648, 838), (998, 1177)],
    'suites': [(839, 841), (851, 997)],
    'files': [(1178, 1226), (1839, 1924)],
    'generation': [(1308, 1780)],
    'coverage': [(2009, 2642)],
    'db_commands': [(2644, 2765)],
}

# 公共imports
common_imports = lines[:14]

# 提取模块
for name, ranges in modules.items():
    with open(f'src/commands/{name}.rs', 'w') as f:
        f.write(''.join(common_imports))
        f.write('use super::*;\n')
        for start, end in ranges:
            for i in range(start-1, end):
                f.write(lines[i])

# 构建commands.rs
commands_content = []
for start, end in commands_ranges:
    commands_content.extend(lines[start-1:end])

# 添加模块声明和re-export
mod_decls = [
    'mod env;\n', 'mod execution;\n', 'mod suites;\n', 'mod generation;\n',
    'mod files;\n', 'mod coverage;\n', 'mod db_commands;\n', '\n',
    'pub use env::*;\n', 'pub use execution::*;\n', 'pub use suites::*;\n',
    'pub use generation::*;\n', 'pub use files::*;\n', 'pub use coverage::*;\n',
    'pub use db_commands::*;\n', '\n',
]

with open('src/commands.rs', 'w') as f:
    f.write(''.join(commands_content))
    f.write(''.join(mod_decls))

print('Done splitting commands.rs into modules.')
