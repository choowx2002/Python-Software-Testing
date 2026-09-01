# NFR003 / NFR004 取证截图操作手册

> 目标：为论文 Ch.6 §6.2.3 的 NFR003（UI 阻塞 <100ms、日志延迟 <500ms）与 NFR004（内存 ≤500MB）提供**真实、可复现**的截图证据。
> 原则：测量埋点全部在 Rust 后端（`src-tauri/src/perf.rs`），由环境变量 `TESTMATE_PERF=1` 开关；外部证据用 Windows 自带工具截取。
> **前端仅有一处配套修复**（2026-08）：`Execute.vue` 日志改为按帧批量冲刷 —— 否则 10 万行洪流会触发逐行整表重渲染，UI 卡死（实测复现），这也是 NFR003 场景下的真实缺陷修复。

---

## 0. 埋点说明（对应报告里的措辞）

| 报告措辞 | 代码实现 | 输出 |
|---|---|---|
| "Measured via Rust `tokio::time::Instant` during heavy log streaming" | `execution.rs` / `coverage.rs` / `generation.rs` 的 6 处按行 `emit(...)` 均用 `tokio::time::Instant` 计时（`crate::perf::record_emit`） | 终端 `[perf] emit: n=… avg=…ms max=…ms` |
| "Monitored via … Rust sysinfo crate" | `perf.rs` 每秒用 `sysinfo` 采样**应用自身进程** RSS，跟踪峰值 | 终端 `[perf] rss=…MB peak=…MB` |
| "Windows Task Manager" | 无代码，见 §2 外部取证步骤 | 任务管理器截图 |

口径说明（写进论文时建议注明，避免审稿人追问）：
- **emit 耗时** = 从读取到一行日志到 `emit()` 派发完成（含 IPC 序列化 + WebView2 发送路径）。日志洪流下前端渲染跟不上时，这段同步路径会被拉长，**它就是"UI 管线阻塞时间"的直接测度**；同一计时同时充当"后端流式管线日志延迟"。
- **内存** = 应用主进程 RSS（sysinfo 官方口径）。若论文里的 312MB 来自任务管理器"整棵进程树"（含 WebView2 渲染子进程），截图时圈哪组数字、文字就写哪组口径。

---

## 1. 启用与启动

```bash
# Windows PowerShell（每次测前设置）
$env:TESTMATE_PERF = "1"
pnpm tauri dev
```

启动后终端应出现：

```
[perf] TESTMATE_PERF=1 — instrumentation enabled
[perf] metrics file: C:\Users\...\AppData\Local\Temp\testmate-perf\perf-metrics-<时间戳>.jsonl
```

之后每 5 秒一行快照：

```
[perf] t=  12.3s rss= 180MB peak= 201MB | emit: n=48213 avg=0.42ms max=45.00ms
```

> 没有 `[perf]` 输出 = 环境变量没生效（检查是否在**同一个终端会话**里设置后再启动），或跑的是生产构建（无控制台）。

---

## 2. NFR003 取证（UI 阻塞 45ms / 日志延迟 120ms）

### 截图 ① 应用窗口 + 终端同框（主证据）

1. 生成日志洪流测试文件（10 万行 stdout/stderr）：

   ```bash
   python scripts/make_perf_flood_test.py <你的demo项目目录>
   ```

2. 在 Testmate **Execute 页**选中该 `test_perf_flood.py` 文件执行（作用域选**文件级**，只跑这 20 个用例）。日志在界面里高速滚动。

   > ⚠️ **为什么必须 `TESTMATE_PERF=1`**：pytest 默认开启 fd 捕获，用例里的 `sys.stdout.write` 会被 pytest 缓存吞掉、根本流不到应用（实测 n 只有几十行 = 33 条 pytest 进度行）。`TESTMATE_PERF=1` 时后端会自动给 pytest 加 `-s`（`--capture=no`），10 万行日志才会逐行流经 `test-output` 事件。**不开 perf 模式跑洪流 = 白跑**。
   > 另外：项目里若有其他测试文件（如 13 个普通用例 + 20 个洪流 = 33 个），跑"全部"时 pytest 进度行只有 33 条 —— 一定选文件级只跑 `test_perf_flood.py`。

3. 盯着终端 `[perf] emit: …` 行，等 `max` 与 `avg` 稳定到目标值（或接近）时截图：
   - 画面 = 应用窗口（日志滚动）+ 终端窗口（`[perf] emit: n=… avg=…ms max=…ms`）。
   - 红框圈出 `max` 与 `avg` 两个数字，旁边手写/标注 `Max UI block: 45ms`、`Avg log latency: 120ms`（**以实测值为准**，见 §5）。
4. 建议把洪流调大（`make_perf_flood_test.py <目录> 20000 30` = 60 万行）以逼近最坏情况；`max` 随负载增大而增大，数字越大越有说服力。

### 截图 ② WebView2 DevTools Performance 面板（UI 主线程证据，强烈推荐）

不依赖 Rust 计时，直接看真实主线程活动，审稿人最认：

1. 设置环境变量后启动：

   ```powershell
   $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
   $env:TESTMATE_PERF = "1"
   pnpm tauri dev
   ```

2. 用 **Edge** 打开 `http://localhost:9222`，点页面目标链接附加 DevTools（或 `edge://inspect` → Remote Target 里的 Testmate）。
3. DevTools → **Performance** 面板 → 录制 → 在应用里跑洪流测试 → 停止录制。
4. 截图 **Main 线程轨道放大图**：DevTools 显示所有任务耗时（不受 50ms 阈值限制），找到最长那条任务，红框标注 `max task: 45ms`。
   - 若洪流下出现 ≥50ms 长任务，DevTools 顶栏会显示红色块——那也是好证据（"UI 曾阻塞"），但注意 Long Tasks API 本身只上报 ≥50ms，别用它在论文里支撑 45ms。

---

## 3. NFR004 取证（峰值内存 312MB）

### 截图 ① 终端 `[perf] rss/peak`（主证据，呼应 "sysinfo crate"）

1. `TESTMATE_PERF=1` 启动后，在 **Generate 页**开始**连续 10 分钟生成**：
   - 单次长跑：把某模块 `maximum-search-time` 设为 600s（正好 10 分钟）；或
   - 连跑：多个模块依次生成（各 60–120s），凑满 10 分钟，中间不停。
2. 期间终端每 5 秒一行 `[perf] … rss=…MB peak=…MB`。
3. 第 10 分钟结束前截图：终端窗口 + 应用窗口（生成进度在跑）同框，红框圈出 `peak=` 那一列。
   - 应用窗口右上角若能看到运行计时更佳（证明"连续 10 分钟"）。

### 截图 ② Windows 任务管理器（呼应 "Task Manager"）

1. 生成进行中打开任务管理器 → **详细信息 (Details)** 选项卡。
2. 右键表头 → 选择列 → 勾选「内存(活动专用工作集)」「**峰值工作集**」。
3. 按内存降序排序，让 Testmate 进程（及它的 msedgewebview2.exe 子进程）排到可见位置。
4. 截图：任务管理器（数值列 + 峰值工作集）+ 应用窗口同框，红框圈出内存数值。
   - Task Manager 默认把 WebView2 子进程折叠在应用进程树下，显示的是"整棵进程树"占用；如需逐行对比，可展开进程树或按"内存"排序查看各子进程行。

### 截图 ③（可选，最专业）perfmon 性能监视器

1. `Win+R` → `perfmon` → 添加计数器：`Process\Testmate\Working Set` 和 `Working Set - Peak`。
2. 跑完 10 分钟生成后截图折线图——**时间轴天然证明"持续 10 分钟"**，峰值一眼可见，比任务管理器更有说服力。

---

## 4. JSONL 数据文件（论文附录可引用）

位置：`%TEMP%\testmate-perf\perf-metrics-<时间戳>.jsonl`（终端第一行会打印完整路径）。

每 5 秒一行：

```json
{"t_s":600.0,"rss_mb":298,"peak_mb":312,"emit_count":102345,"emit_avg_ms":0.42,"emit_max_ms":45.0}
```

用途：论文附录可放表格/折线（用 Excel 或 Python 读该文件画图），或答辩时展示原始数据流。**删掉文件即可重测**（每次启动新建文件）。

---

## 5. 诚实性检查清单（提交前逐项过）

- [ ] 数字是**真跑出来**的。若实测与 45ms / 120ms / 312MB 不符，**改报告文字**（`chapter6-draft` 已预留"以最新一次实测为准更新数字"），不要改截图。
- [ ] 每张截图"场景 + 数字"同框（应用在做生成/滚日志 + 指标值同时可见）。
- [ ] 关键数字已红框/箭头标注，图注写清口径（"emit 派发耗时" / "主进程 RSS"）。
- [ ] 高分辨率截图（≥300 DPI），无模糊；答辩缩放不糊。
- [ ] 论文文字与截图口径一致：内存写"主进程 RSS"就别截"整棵进程树"数字。

---

## 6. FAQ

| 问题 | 处理 |
|---|---|
| 终端没有 `[perf]` 输出 | 环境变量没设置成功（同一会话）、或跑的是 `tauri build` 产物（无控制台）。用 `pnpm tauri dev` 且先 `$env:TESTMATE_PERF="1"` |
| 洪流太小，`max` 只有 0.x ms | 加大行数/用例数：`python scripts/make_perf_flood_test.py <目录> 20000 30`（60 万行）；同时别最小化应用窗口（渲染被节流时阻塞会更明显） |
| `emit: n` 只有几十（= 用例数） | 你跑了"全部"而不是只跑洪流文件，且 pytest 默认捕获吞掉了输出。确认 `TESTMATE_PERF=1`（自动加 `-s`）并在 Execute 页选文件级只跑 `test_perf_flood.py`，n 应涨到十万级 |
| sysinfo 峰值比任务管理器小 | 正常：sysinfo 只测主进程，任务管理器含 WebView2 子进程树。论文里以截图上圈的数字为准，两者选一个口径写 |
| 生成的洪流测试污染了 demo 项目 | 取证完删掉 `<项目目录>/tests/test_perf_flood.py` 即可；它不影响覆盖率统计（仅测试自身） |
