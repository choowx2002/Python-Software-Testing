import os

with open('src/commands.rs', 'r') as f:
    lines = f.readlines()

# 公共imports
common_imports = lines[:14]

# 模块区域（1-based, inclusive）
modules = {
    'env': [(75, 330), (1781, 1835)],
    'execution': [(331, 838), (991, 1249)],
    'suites': [(839, 989)],
    'files': [(1836, 1924)],
    'generation': [(1250, 1780)],
    'coverage': [(1925, 2642)],
    'db_commands': [(2643, 2765)],
}

# 提取模块
for name, ranges in modules.items():
    with open(f'src/commands/{name}.rs', 'w') as f:
        f.write(''.join(common_imports))
        f.write('use super::*;\n')
        for start, end in ranges:
            for i in range(start-1, end):
                f.write(lines[i])

# 构建commands.rs（只保留共享imports + 顶部数据结构 + mod声明 + re-export）
commands_content = []
commands_content.extend(lines[:14])  # imports
commands_content.extend(lines[14:74])  # 顶部数据结构

mod_decls = [
    'pub mod env;\n',
    'pub mod execution;\n',
    'pub mod suites;\n',
    'pub mod generation;\n',
    'pub mod files;\n',
    'pub mod coverage;\n',
    'pub mod db_commands;\n',
    '\n',
]

with open('src/commands.rs', 'w') as f:
    f.write(''.join(commands_content))
    f.write(''.join(mod_decls))

print('Done.')
