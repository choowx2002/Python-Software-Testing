# Testmate User Manual

> 版本：v0.1.0 · 对应 FYP 论文 Ch.6（Documentation and Packaging）
> 适用系统：Linux（deb/rpm 安装包）/ Windows / macOS（源码构建）

---

## 1. 简介

Testmate 是一款面向 Python 小型项目的自动化测试工具，整合三个核心开源库：

| 能力 | 引擎 |
|------|------|
| 单元测试执行 | pytest |
| 自动化测试生成 | Pynguin（搜索式测试生成） |
| 代码覆盖率分析 | coverage.py |

应用基于 Tauri v2 构建：Vue 3 提供界面，Rust 承担业务逻辑与进程编排（类型安全 IPC），Python 作为按需启动的隔离子进程引擎。所有长任务（测试执行、测试生成）通过事件流式推送实时日志，界面不会阻塞。

## 2. 系统要求

- **操作系统**：Linux（x86_64）、Windows 10/11、macOS 12+
- **Python**：3.10 及以上（测试生成器 Pynguin 需要 Python 3.10+）
- **Git**：仅克隆远程仓库功能需要
- **磁盘空间**：约 300 MB（含运行时与依赖）

## 3. 安装

### 3.1 使用安装包（Linux）

```bash
# Debian / Ubuntu / Mint
sudo dpkg -i Testmate_0.1.0_amd64.deb

# Fedora / RHEL / openSUSE
sudo rpm -i Testmate-0.1.0-1.x86_64.rpm
```

安装后可在应用菜单中启动 Testmate。

### 3.2 从源码构建（开发模式）

```bash
pnpm install
pnpm tauri dev        # 开发模式
pnpm tauri build      # 生产构建（输出 deb/rpm/AppImage）
```

> 注意：AppImage 打包在 Arch Linux 上因 linuxdeploy 已知问题可能失败，属环境问题，不影响 deb/rpm。

## 4. 快速上手

1. 启动应用，进入仪表盘（Dashboard）。
2. 点击侧栏 **Import Project**，选择本地 Python 项目目录。
3. 向导自动检测虚拟环境与依赖；缺失依赖可一键安装（`Install Missing`）。
4. 若无虚拟环境，点击 **Create .venv** 自动创建。
5. 环境就绪后点击 **Import Project** 保存，项目出现在列表中。
6. 点击项目行进入工作区，即可使用 执行 / 生成 / 覆盖率 三个模块。

也可以点击 **Clone Repository**，输入 Git 远程地址（https/ssh）并选择目标文件夹，Testmate 会自动克隆并导入。

## 5. 功能模块

### 5.1 执行（Execute）

- **测试范围**：全部测试 / 单个测试文件 / 手动勾选用例。
- **运行配置**：内置预设（Standard / Quick / Debug / Stop on Failure），或在"高级选项"中输入自定义 pytest 参数（将覆盖预设）。
- **回归套件（FR007）**：点击 **Save as Suite** 保存当前选择与参数；套件列表支持一键重跑与删除。执行类型记录为 `MANUAL` / `REGRESSION`。
- **实时反馈**：执行进度、当前用例、实时 pytest 输出（可复制/清空）、通过/失败/跳过统计。
- **失败重跑**：结果区 **Rerun Failed** 一键重跑失败用例。
- **停止**：执行中可点击 **Stop**，应用先发送 SIGTERM、2 秒后未退出再 SIGKILL。
- **自动覆盖率（FR011）**：测试有通过项时，会询问是否继续覆盖率分析；确认后自动跳转并运行。

### 5.2 生成（Generate）

- **源文件选择**：树形浏览项目源码（排除 tests/ 与 venv），支持搜索、全选。
- **生成配置**：
  - 算法：MOSA / DYNAMOSA / WSPA / RANDOM
  - 最大搜索时间（秒，每文件）
  - 断言生成开关、最大测试用例数
  - 输出文件夹（相对项目根，默认 `tests/generated`）
  - 高级：随机种子、染色体长度、种群大小
- **结果**：生成文件列表（成功/空/失败状态），可打开文件、在文件夹中显示、复制路径；实时日志可复制。
- 生成的测试仅写入输出目录，不修改源码（FR009）。

### 5.3 覆盖率（Coverage）

- **测量范围**：勾选源文件（自动派生 `--source` 目录）与测试文件（不选则运行全部测试）。
- 首次使用若提示 coverage.py 未安装，点击 **Install coverage.py** 自动安装到项目虚拟环境。
- **结果**：总体覆盖率、语句统计、文件列表（按覆盖率排序，>80% / 50-80% / <50% 筛选）。
- **源码查看器**：点击文件展开逐行视图，绿色=已覆盖、红色=未覆盖、灰色=不可执行。
- **导出报告**：JSON / CSV 导出到项目数据目录；也可在文件管理器中打开。

## 6. 数据存储

- 数据库：SQLite，位于应用配置目录 `pytest_auto.db`（四张表：`projects`、`test_execution_history`、`regression_suites`、`coverage_results`）。
- 覆盖率明细 JSON 与导出的 CSV/JSON 报告保存在各项目独立子目录中。

## 7. 界面语言

应用默认英文界面，可在仪表盘左下角切换 **EN / 中文**，选择会记住（下次启动保持）。

## 8. 故障排查

| 现象 | 处理 |
|------|------|
| 导入时提示 Python 未安装 | 安装 Python 3.10+ 后重启应用 |
| 执行页无测试用例 | 确认项目含 `test_*.py` / `*_test.py` 且已安装 pytest |
| 生成结果为空 | 确认所选模块可被 import（无外部依赖/语法错误），可增大搜索时间 |
| 覆盖率按钮禁用 | 先在 Coverage 页点击 Install coverage.py 安装 |
| Stop 无效 | 罕见情况下请稍候再试；应用退出时会自动清理残留进程 |
| 打包/构建问题 | 见本手册 3.2 节说明 |
