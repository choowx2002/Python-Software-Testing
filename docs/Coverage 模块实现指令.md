```markdown
# Coverage 模块实现指令

## 架构约束（必须遵守）

Testmate 自身**不包含**任何 Python 依赖。所有 Python 工具（pytest / coverage.py / pynguin）安装在**用户导入的目标项目的 venv** 中。
```

Testmate（本项目）
  └── 只做：UI + Rust 编排 + 调用子进程

用户导入的 Python 项目（目标项目）
  └── .venv/
        ├── pytest
        ├── coverage.py
        └── pynguin

```
因此：
- 解释器路径：使用 `detect_python_env` 返回的用户项目 venv 路径
- 子进程 cwd：设为用户项目根目录
- 如果用户 venv 中没有 coverage.py，提示安装（复用 `install_dependencies` 模式）
- **禁止**使用系统 python 或 Testmate 自身路径

---

## Phase 1：Rust 后端

### 1.1 新增 `check_coverage_installed` 命令

```rust
#[tauri::command]
async fn check_coverage_installed(
    interpreter_path: String,
) -> Result<bool, String>
```

- 执行：`{interpreter_path} -m coverage --version`
- 成功返回 `true`，失败返回 `false`
- 供前端判断是否显示"Install coverage.py"按钮

### 1.2 新增 `run_coverage` 命令

```rust
#[tauri::command]
async fn run_coverage(
    app: tauri::AppHandle,
    project_id: i64,
    project_path: String,        // 用户项目根目录
    interpreter_path: String,    // 用户 venv 的 python 路径
    test_files: Vec<String>,     // 要执行的测试文件
    source_dirs: Vec<String>,    // 要统计覆盖率的源码目录
) -> Result<CoverageSummary, String>
```

执行流程：

1. **检查 coverage 可用性**
   
   - `{interpreter_path} -m coverage --version`
   
   - 失败则 emit `coverage-error`，返回 "coverage.py not found in project venv"

2. **清除旧数据**
   
   - `{interpreter_path} -m coverage erase`
   
   - cwd = project_path

3. **执行覆盖率采集**
   
   - `{interpreter_path} -m coverage run --source={source_dirs.join(",")} -m pytest {test_files.join(" ")}`
   
   - cwd = project_path
   
   - 异步读取 stdout/stderr，通过 `coverage-output` 事件流式推送

4. **导出 JSON 报告**
   
   - `{interpreter_path} -m coverage json -o {coverage_json_path}`
   
   - `coverage_json_path` = `{app_data_dir}/{project_id}/coverage.json`

5. **解析 JSON**
   
   - 读取导出的 JSON 文件
   
   - 提取：
     - `totals.percent_covered`（总体 statement 覆盖率）
     - 每个文件的：`summary.percent_covered`、`summary.num_statements`、`summary.num_hits`
     - 每个文件的：`executed_lines`、`missing_lines`、`excluded_lines`

6. **写入数据库**
   
   - INSERT INTO `coverage_results`

7. **返回结构化结果**
   
   - emit `coverage-finished` 携带摘要

### 1.3 新增 `get_coverage_detail` 命令

```rust
#[tauri::command]
async fn get_coverage_detail(
    app: tauri::AppHandle,
    project_id: i64,
    file_path: String,
) -> Result<FileCoverageDetail, String>
```

- 从缓存的 coverage.json 中读取指定文件的行级数据
- 返回：源码内容 + 每行覆盖状态（covered / uncovered / excluded）

### 1.4 新增 `export_coverage_report` 命令

```rust
#[tauri::command]
async fn export_coverage_report(
    app: tauri::AppHandle,
    project_id: i64,
    format: String,  // "json" | "csv" | "html"
) -> Result<String, String>
```

- 导出覆盖率报告到用户指定位置
- 复用 `reveal_in_folder` 模式

### 1.5 数据结构

```rust
#[derive(Serialize)]
struct CoverageSummary {
    total_statement_coverage: f64,
    total_branch_coverage: Option<f64>,
    file_count: usize,
    covered_file_count: usize,
    files: Vec<FileCoverage>,
}

#[derive(Serialize)]
struct FileCoverage {
    file_path: String,
    percent_covered: f64,
    num_statements: usize,
    num_hits: usize,
    num_misses: usize,
}

#[derive(Serialize)]
struct FileCoverageDetail {
    file_path: String,
    source_code: String,
    lines: Vec<LineCoverage>,
}

#[derive(Serialize)]
struct LineCoverage {
    line_number: usize,
    status: LineStatus,  // Covered / Uncovered / Excluded / NonExecutable
    hits: Option<usize>,
}
```

### 1.6 事件命名

| 事件                  | 时机   | payload               |
| ------------------- | ---- | --------------------- |
| `coverage-started`  | 开始采集 | `{ project_id }`      |
| `coverage-output`   | 实时日志 | `{ line: String }`    |
| `coverage-finished` | 完成   | `CoverageSummary`     |
| `coverage-error`    | 错误   | `{ message: String }` |

---

## Phase 2：数据库

### 2.1 确认/创建 `coverage_results` 表

在 `src/utils/db.ts` 中确认存在：

```sql
CREATE TABLE IF NOT EXISTS coverage_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id INTEGER NOT NULL,
  execution_id INTEGER,
  total_statement_coverage REAL NOT NULL,
  total_branch_coverage REAL,
  file_count INTEGER NOT NULL,
  covered_file_count INTEGER NOT NULL,
  detail_json_path TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (project_id) REFERENCES projects(id),
  FOREIGN KEY (execution_id) REFERENCES test_execution_history(id)
);
```

### 2.2 索引

```sql
CREATE INDEX IF NOT EXISTS idx_coverage_project ON coverage_results(project_id);
CREATE INDEX IF NOT EXISTS idx_coverage_execution ON coverage_results(execution_id);
```

---

## Phase 3：前端 Coverage.vue

替换 `src/views/projects/[id]/Coverage.vue` 的 `<template>111</template>`。

### 3.1 页面结构

```
┌─────────────────────────────────────────────────┐
│ 顶部操作区                                       │
│ [源码目录选择] [测试文件选择] [Run Coverage] [Install coverage.py] │
├─────────────────────────────────────────────────┤
│ 摘要卡片区                                       │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│ │ Statement │ │ Branch   │ │ Files    │        │
│ │  87.3%   │ │  72.1%   │ │  24/31   │        │
│ └──────────┘ └──────────┘ └──────────┘        │
├─────────────────────────────────────────────────┤
│ 文件列表区（可排序、可过滤）                       │
│ ┌─────────────────────────────────────────────┐ │
│ │ src/main.py        ████████████░░  87%     │ │
│ │ src/utils.py       ██████░░░░░░░░  52%     │ │
│ │ src/parser.py      ███░░░░░░░░░░░  23%     │ │
│ └─────────────────────────────────────────────┘ │
│ 过滤：[All] [>80%] [50-80%] [<50%]  排序：[▼覆盖率] │
├─────────────────────────────────────────────────┤
│ 代码查看器（点击文件展开）                         │
│ ┌─────────────────────────────────────────────┐ │
│ │ 1  │ import os          │ ← 绿色（已覆盖）   │ │
│ │ 2  │                    │ ← 灰色（非可执行） │ │
│ │ 3  │ def unused():      │ ← 红色（未覆盖）   │ │
│ │ 4  │     pass           │ ← 红色（未覆盖）   │ │
│ └─────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────┤
│ 底部操作区                                       │
│ [Export JSON] [Export CSV] [Open in File Manager] │
└─────────────────────────────────────────────────┘
```

### 3.2 功能要求

| 功能               | 实现方式                                       |
| ---------------- | ------------------------------------------ |
| 检查 coverage 是否已装 | 进入页面时 invoke `check_coverage_installed`    |
| 安装 coverage      | 按钮触发，复用 `install_dependencies` 模式          |
| 选择源码目录           | 复用 `scan_source_files` + TreeItem 组件       |
| 选择测试文件           | 复用 `scan_test_files`                       |
| 运行覆盖率            | invoke `run_coverage`，监听 `coverage-*` 事件   |
| 实时日志             | 监听 `coverage-output`，显示在日志面板               |
| 摘要展示             | 监听 `coverage-finished`，渲染卡片                |
| 文件列表             | 从 summary.files 渲染，支持排序/过滤                 |
| 代码查看器            | 点击文件 → invoke `get_coverage_detail` → 行级高亮 |
| 导出               | invoke `export_coverage_report`            |
| 历史记录             | 从 `coverage_results` 表读取，显示时间线             |

### 3.3 代码查看器行级高亮规则

| 行状态            | 背景色                                    | 说明                    |
| -------------- | -------------------------------------- | --------------------- |
| covered        | `bg-green-100` / `bg-green-900` (dark) | 已执行                   |
| uncovered      | `bg-red-100` / `bg-red-900` (dark)     | 未执行                   |
| excluded       | `bg-gray-100`                          | 被排除（pragma: no cover） |
| non-executable | 无背景                                    | 空行/注释/import          |

### 3.4 样式约束

- 使用项目现有的 Tailwind 类
- 卡片样式参考 `Execute.vue` 的结果汇总卡片
- 进度条颜色：≥80% `bg-emerald-500`，50-79% `bg-amber-500`，<50% `bg-red-500`
- 日志面板样式参考 `Execute.vue` 的实时日志区域

---

## Phase 4：与测试执行串联（FR011）

### 4.1 自动触发

在 `Execute.vue` 的 `test-finished` 事件处理中：

```typescript
// 测试执行成功后，自动触发覆盖率采集
if (result.status === 'SUCCESS' || result.passed > 0) {
  // 询问用户是否立即采集覆盖率
  const shouldRunCoverage = await confirm(
    'Tests completed. Run coverage analysis?',
    'Coverage'
  );
  if (shouldRunCoverage) {
    router.push(`/projects/${projectId}/coverage`);
    // 自动触发 run_coverage
  }
}
```

### 4.2 手动触发

用户也可以直接在 Coverage Tab 手动选择文件并运行。

---

## Phase 5：路由与导航

### 5.1 确认路由

确保 `src/router/index.ts` 中有：

```typescript
{
  path: '/projects/:id/coverage',
  name: 'Coverage',
  component: () => import('@/views/projects/[id]/Coverage.vue'),
}
```

### 5.2 确认 Layout.vue Tab

确保 `Layout.vue` 中的 Coverage Tab 正确链接到该路由。

---

## Phase 6：注册命令

在 `src-tauri/src/lib.rs` 中注册新命令：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 现有命令 ...
    check_coverage_installed,
    run_coverage,
    get_coverage_detail,
    export_coverage_report,
])
```

---

## 测试验证清单

| #   | 测试项                   | 预期结果                       |
| --- | --------------------- | -------------------------- |
| 1   | 用户 venv 无 coverage.py | 显示"Install coverage.py"按钮  |
| 2   | 点击 Install            | 成功安装到用户 venv               |
| 3   | 选择源码目录 + 测试文件，点击 Run  | 实时日志滚动，完成后显示摘要             |
| 4   | 摘要卡片                  | 显示正确的 statement/branch 覆盖率 |
| 5   | 文件列表                  | 按覆盖率排序，颜色正确                |
| 6   | 点击文件                  | 代码查看器展开，行级高亮正确             |
| 7   | 过滤                    | 按阈值过滤文件列表                  |
| 8   | 导出 JSON               | 文件保存到指定位置                  |
| 9   | 导出 CSV                | 格式正确                       |
| 10  | 测试执行后自动提示             | 弹出确认框                      |
| 11  | 用户项目无测试文件             | 友好错误提示                     |
| 12  | coverage.py 执行报错      | `coverage-error` 事件正确显示    |

---

## 文件清单

| 操作    | 文件路径                                         |
| ----- | -------------------------------------------- |
| 新建/重写 | `src/views/projects/[id]/Coverage.vue`       |
| 修改    | `src-tauri/src/commands.rs`（新增 4 个命令）        |
| 修改    | `src-tauri/src/lib.rs`（注册命令）                 |
| 修改    | `src/utils/db.ts`（确认 coverage_results 表）     |
| 修改    | `src/views/projects/[id]/Execute.vue`（串联覆盖率） |
| 确认    | `src/views/projects/Layout.vue`（Tab 路由）      |
| 确认    | `src/router/index.ts`（路由配置）                  |

---

## 实现顺序

1. Rust：`check_coverage_installed` → 前端可判断状态
2. Rust：`run_coverage` → 核心采集逻辑
3. 前端：Coverage.vue 基础版（操作区 + 摘要 + 文件列表）
4. Rust：`get_coverage_detail` → 行级数据
5. 前端：代码查看器（行级高亮）
6. Rust：`export_coverage_report` → 导出
7. 前端：过滤 + 排序 + 导出按钮
8. 串联：Execute.vue → Coverage 自动触发
9. Dashboard：`projectStore.ts` 中 coverage 字段改为真实查询
   ```
