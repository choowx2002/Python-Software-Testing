with open('src-tauri/src/commands_original.rs', 'r') as f:
    lines = f.readlines()

common_imports = lines[:14]

# 所有模块区域（1-based, inclusive）
modules = {
    'env': [(75, 330), (1781, 1835)],
    'execution': [(331, 838), (991, 1249)],
    'suites': [(839, 989)],
    'files': [(1836, 1924)],
    'generation': [(1250, 1780)],
    'coverage': [(1925, 2642)],
    'db_commands': [(2643, 2765)],
}

for name, ranges in modules.items():
    with open(f'src-tauri/src/commands/{name}.rs', 'w') as f:
        f.write(''.join(common_imports))
        f.write('use super::*;\n')
        for start, end in ranges:
            for i in range(start-1, end):
                f.write(lines[i])

print('Done extracting all modules.')
