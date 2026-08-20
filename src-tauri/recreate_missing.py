with open('src-tauri/src/commands_original.rs', 'r') as f:
    lines = f.readlines()

common_imports = lines[:14]

# 重新创建缺失的模块文件
modules = {
    'execution': [(331, 838), (991, 1249)],
    'db_commands': [(2643, 2765)],
}

for name, ranges in modules.items():
    with open(f'src-tauri/src/commands/{name}.rs', 'w') as f:
        f.write(''.join(common_imports))
        f.write('use super::*;\n')
        for start, end in ranges:
            for i in range(start-1, end):
                f.write(lines[i])

print('Done recreating missing modules.')
