# UTAR FYP 海报制作与展示全套指南
**Student: Choo Wei Xiang (ID: 2400056) | Supervisor: Dr Sneha Kanchan**
**项目: Automated Testing Software (Testmate) — Unit & Regression Testing with Code Coverage Analysis**
**赛事: LKC FES FYP Poster Competition | 格式: Portrait A1 (594 mm × 841 mm)**

> 本指南基于你的 FYP 报告《Design and Development of an Automated Testing Software for Unit and Regression Testing with Code Coverage Analysis》与官方模板 `2. FYP-Poster-Template-202506 Ver 1.0.pptx` 生成。
> ⚠️ 注意：你在任务里写的 Author Info 是 "Student: 天龠wx"，但根据你的说明（学生姓名 Choo Wei Xiang）和报告署名，海报应使用 **CHOO WEI XIANG**。

---

## 1. 海报核心文案提取（直接用于排版）

*总字数约 400 词（不含标题与作者行），符合"300–500 词"要求。全部为可直接粘贴的英文文案。*

### Title（标题，≤20 词，加粗）

**Recommended（推荐）:**
> **Testmate: Automated Unit & Regression Testing with Code Coverage Analysis for Python Projects**

*(12 词。把产品名 "Testmate" 放前面做记忆点，副标题准确反映主题——符合规则 "The title must accurately reflect the subject matters"。)*

备用官方全称（16 词，较长但更正式）:
> Design and Development of an Automated Testing Software for Unit and Regression Testing with Code Coverage Analysis

### Author Info（作者信息）

> **Choo Wei Xiang** | Supervisor: **Dr Sneha Kanchan**
> Lee Kong Chian Faculty of Engineering and Science, Universiti Tunku Abdul Rahman
> Bachelor of Software Engineering with Honours
> (如规则需要可加 Co-supervisor 姓名)

### Introduction（引言 / Problem & Motivation，4 点，编号 P1–P4）

> 🎯 **1:1 映射设计（评委一眼可见的逻辑闭环）：** Problem 与 Objective **同序编号**（P1↔O1、P2↔O2、P3↔O3、P4↔O4），海报上两节用**同色编号徽章 ①②③④** 一一呼应，评委扫一眼就知道"每个障碍都有对应解法"。

- **Lead-in（项目 overview，对应规则 "clear overview of the project"）:** **Testmate** is a GUI-based automated testing software for small Python projects, integrating unit & regression testing (pytest), automated test generation (Pynguin) and coverage analysis (coverage.py).

- **P1 — Fragmented toolchains:** pytest, coverage.py and Pynguin demand complex, manual configuration that rarely interoperates.
- **P2 — Test-design knowledge gap:** Manual test design needs advanced skills → duplicate, happy-path-only test suites.
- **P3 — Invisible feedback:** Regression & coverage results stay buried in CLI output; beginners misread coverage as quality.
- **P4 — Adoption gap（研究空白/动机）:** Students often skip testing — most were ineffective at finding known defects (Bai et al., 2021).

> 💡 **超短一段话版（若想再省空间，四行合一）:**
> *"Testing ensures reliability, yet students skip it: tools are fragmented, test design is hard, and coverage feedback is invisible (Bai et al., 2021). Testmate lowers this barrier with one GUI."*（39 词）

### Objectives（目标，4 点，与 Problem 一一对应，动词开头）

- **O1** ↔ **P1** — **Design & implement** an integrated unit & regression testing workflow that reduces configuration overhead for small Python projects.
- **O2** ↔ **P2** — **Integrate** automated unit test generation (Pynguin) that never modifies the original source code.
- **O3** ↔ **P3** — **Develop** a GUI (Tauri v2) providing real-time, visual feedback on test results and code coverage.
- **O4** ↔ **P4** — **Evaluate** usability and effectiveness with target students and novice developers, validating the impact on testing adoption.

### Methodology（方法）

- **Process:** Iterative development model — 6 phases: requirements → test execution module → test generation module → coverage analysis module → usability evaluation → packaging.
- **Architecture:** Tauri v2 desktop app — **Vue 3** frontend ⇄ **Rust** backend (type-safe IPC) ⇄ **Python 3.10+** subprocess engine (**pytest**, **Pynguin**, **coverage.py**).
- **Workflow:** Import project → auto-detect venv & dependencies → run tests with live-streamed logs → auto-trigger coverage → interactive code viewer (line-level highlight, file & function filters) → save regression suites for one-click re-runs.
- **Validation:** Real project demo (3 modules, 103 statements, 48 branches) — full workflow timed at **5 minutes**.

**建议画成流程图的 3 处内容（详见第 2 节）：**
1. 四阶段用户旅程图 (Setup → Generate → Execute → Coverage)
2. 三层架构图 (Vue ⇄ Rust ⇄ Python)
3. 覆盖率反馈闭环图 (Test → Coverage → Highlight → Improve)

### 排版字号速查（A1 纵向，全部模块）

| 模块 | 字号 | 字体 |
|------|------|------|
| Title | 72–90pt | Montserrat ExtraBold / Arial Black |
| Author 行 | 28–32pt | Calibri Bold |
| 模块标题（INTRODUCTION 等） | 40–44pt | Montserrat SemiBold |
| 正文 Bullet | 24–28pt | Calibri / Arial |
| 编号徽章 ①②③④ | 与正文同号，强调色 | Bold |
| 图表内文字/表格 | 20–22pt | Calibri |
| 页脚/QR 说明 | 18–20pt | Calibri |

**定稿文案总字数 ≈ 470 词**（Title + 各模块正文，符合 300–500 词约束）。

### Results & Discussion（结果与讨论）— ✅ 已用实测数据定稿（2026-08 实测）

> 实测记录：演示项目 testmate-demo-shop（3 模块 / 103 条可执行语句 / 48 个分支），2026-08 通过 Testmate 完成全流程。

**定稿文案（直接粘贴到海报）：**

- **Delivered system:** "Testmate" — a working Tauri v2 desktop app integrating test execution (pytest), automated test generation (Pynguin) and coverage analysis (coverage.py) in a single GUI.
- **Functional verification:** All 13 functional requirements (FR001–FR013) implemented & verified — venv auto-detection, real-time test logs, pass/fail summary, one-click regression suites, safe test generation, interactive coverage viewer with file & function filters, report export.
- **Automated test generation:** 35 unit tests generated for 3 modules in 180 s (15 / 12 / 8 tests for cart, pricing, utils) — 34 passed, 1 expected failure (xfail), 0 unexpected failures.
- **Coverage improvement:** Statement coverage rose from **59.2% → 93.2%** (61 → 96 of 103 lines) and branch coverage from **41.7% → 89.6%** (20 → 43 of 48 branches) after adding generated tests.
- **Usability:** A complete first workflow (import → generate → execute → coverage) was completed in **5 minutes** — well within the 15-minute design target (NFR001).
- **Discussion:** An integrated GUI removes configuration overhead and CLI interpretation barriers; generated tests complement hand-written happy-path suites by reaching untested modules and branches. Limitations: Python-only, small-to-medium codebases, basic-level generation.

*对应图表（数字已定）：*
- **覆盖率前后对比进度条**：语句 59.2% → 93.2%（两条粗进度条 + 箭头）+ 分支 41.7% → 89.6%（第二对进度条）
- **环形图**：35 个生成测试 — 34 绿（passed）/ 1 琥珀（xfail），0 红
- **柱状图**：每模块生成测试数 cart 15 / pricing 12 / utils 8（或 180s 总耗时）
- **趋势折线图**：History 页 CombinedTrendChart 截图（多次运行覆盖率上升曲线）
- **1–2 张高分辨率截图**：测试结果卡片 + 覆盖率摘要卡片（含语句/分支双数字）
- **FR001–FR013 功能验证打勾矩阵**（13/13）

**Q&A 引用句（答辩可用）：**
> "On a 103-statement demo project, Pynguin-generated tests lifted statement coverage from 59.2% to 93.2% and branch coverage from 41.7% to 89.6% — 35 tests in 180 seconds, all green except one expected failure."

**Plan B（若后续补了用户对照实验，可再加）：**
- **Usability (comparative):** GUI cut setup/execution time from `[X]`→`[Y]` min vs. manual CLI; error rate ↓`[ ]%`; success rate ↑`[ ]%`; ease-of-use score `[ ]/5`.

### Conclusion（结论，2–3 点）

- **Contribution:** Testmate integrates unit testing, regression testing and code coverage analysis into a single low-configuration GUI — bridging professional testing standards and educational needs.
- **Impact:** Empowers students and novice developers to adopt testing as a routine, quality-driven practice instead of skipping it.
- **Future work:** CI/CD pipeline integration, multi-language support, advanced generation for complex business logic, and larger-scale evaluation.

---

## 2. 视觉设计与排版建议 (Portrait A1: 594mm × 841mm)

### 2.1 布局规划（最终版式，mm 级坐标，A1 = 594 × 841）

```
┌──────────────────────────────────────────────────────────────┐
│ HEADER  y 0–95    UTAR 校徽(左,40×40) + 标题 72–90pt 居中      │
│                   作者行 28–32pt + 深蓝分隔线 3mm              │
├──────────────────────────┬───────────────────────────────────┤
│ 左栏 x 25–205 (180mm)    │ 右栏 x 225–569 (344mm)             │
│ y115–350 INTRODUCTION    │ y115–240 METHODOLOGY (3 bullets)   │
│   P1–P4 徽章①②③④        │ y245–285 架构小图条(3层+箭头)       │
│ y365–610 OBJECTIVES      │ y295–465 ⭐HERO 四阶段流程图        │
│   O1–O4 徽章①②③④(同色)  │   (344×170 全海报最大视觉)          │
│ y625–705 截图① 执行结果   │ y475–545 RESULTS (4 compact bullets│
│   卡片 (180×80)          │   + "13/13 FR ✅" 绿色徽章条)       │
│ y715–770 截图② 覆盖率摘要 │ y555–645 前后对比进度条             │
│   卡片 (180×55)          │   (语句 59.2→93.2 / 分支 41.7→89.6) │
│                          │ y655–770 图表行: 环形图(35测试)     │
│                          │   150×115 + 趋势折线图截图 180×115  │
├──────────────────────────┴───────────────────────────────────┤
│ FOOTER y 770–841: CONCLUSION 3 点(24pt, 2行) + QR码(右,40×40) │
└──────────────────────────────────────────────────────────────┘
```

- **眼动法则：** 评委视线 = 标题 → **HERO 流程图（右栏上部 y295–465，全海报最大图）** → 左右栏正文 → 底部结论。所有数字用强调色放大 1.5 倍。
- **P↔O 呼应：** 左栏上 P1–P4 与 O1–O4 的徽章用**同色同号**（①蓝 ②青 ③琥珀 ④绿），纵向对齐阅读，映射一目了然。
- **留白：** 模块间 15–20mm；右栏 RESULTS 与图表区之间保持 10mm 呼吸空间。
- **三栏备选：** 若截图横版更宽，可把左栏底部两截图移到右栏底部做横排（每张 165×65），左栏仅留 Intro+Objectives——二选一，不要都做。

### 2.2 图表/视觉元素建议（直接对应你的文档）

| # | 图表 | 数据来源 | 长什么样（拿高分的关键） |
|---|------|----------|--------------------------|
| 1 | **四阶段工作流程图**（主图） | Ch.4 活动图 / Ch.5.2 组件交互 | 横向 4 个圆角方块：`Import Project → Generate Tests (Pynguin) → Run Tests (pytest) → Coverage (coverage.py)`，用箭头串联，每个方块配小图标 + 一行字；下方用细箭头标出"实时日志流 / 反馈闭环" |
| 2 | **三层架构图** | Ch.5.1 图 5.1 | 三块竖排：Vue 3 (UI) ⇄ Rust (IPC/业务逻辑) ⇄ Python 子进程引擎；右侧标注 SQLite + JSON 持久化；配色用同色系不同深浅 |
| 3 | **柱状图：每模块生成测试数**（已定稿） | 2026-08 实测 | 三根柱：cart 15 / pricing 12 / utils 8，顶部标 180s 总耗时；柱色用强调青 |
| 4 | **环形图/仪表盘：测试结果分布** | 2026-08 实测 | donut 图：34 passed(绿) / 1 xfail(琥珀) / 0 failed，中间显示总数 35；旁边放"0 unexpected failures"小字 |
| 5 | **覆盖率"前后对比"进度条**（视觉核心） | 2026-08 实测 | 两对粗进度条：语句 59.2% → 93.2%（61→96/103 行）；分支 41.7% → 89.6%（20→43/48），用箭头连接，数字用强调色放大 |
| 6 | **折线图：覆盖率历史趋势** | History 页 CombinedTrendChart 截图 | 展示多次运行语句/分支覆盖率上升曲线——直接截应用内趋势图，保证"真实截图"可信度 |
| 7 | **软件截图 1–2 张**（高分辨率） | 你的 Dashboard / Coverage 视图 | 带红框标注关键区域；截图分辨率至少 300 DPI，不能模糊（规则明确要求 high resolution） |
| 8 | **小表格：工具选型** | Ch.2 Table 2.3 | 3 行小表：pytest / Pynguin / coverage.py + 各自作用。表格字号可小，但 ≥20pt |

**主图选择原则：** 全海报只允许 1 个大主图（推荐流程图 #1），其余图表是"证据型"配角。宁缺毋滥——规则说 "REMOVE ALL THE EXTRA / UNWANTED PART(S)"。

### 2.3 配色与字体

**配色（软件工程/工具类项目，清爽专业风）：**
- 背景：**纯白或极浅灰 (#FAFAFA)**——规则明确提示彩色背景会降低可读性，不要花哨背景。
- 主色（标题/模块头）：**深海军蓝 #1B3A6B**
- 强调色（图标/图表主色）：**科技青 #00A6A6** 或 **亮蓝 #2E86DE**
- 语义色（仅用于图表）：绿 #27AE60（通过）、红 #E74C3C（失败）、琥珀 #F39C12（跳过）
- 比例建议：70% 白 + 20% 深蓝 + 10% 强调色。全海报不超过 4 种颜色。

**字体（Windows 环境，模板用 PowerPoint 制作）：**
- 标题：**Montserrat ExtraBold**（无则用 Arial Black / Calibri Bold），**72–90pt**
- 模块标题：同字族 SemiBold，**36–44pt**
- 正文：**Calibri / Arial**，**24–28pt**（1–2 米外可读的下限）
- 图表标注/表内文字：**20–22pt**（最小下限，不能再小）
- 数字和百分比：用粗体 + 强调色放大 1.5 倍，制造视觉锚点

---

## 3. 现场展示 (Pitch) 与 Q&A 准备

### 3.1 演讲大纲（常规赛道 10 分钟版）

| 时间 | 段落 | 内容要点 |
|------|------|----------|
| 0:00–1:00 | **开场 Hook** | "Testing finds bugs — but most students never test their own code. Bai et al. found students were ineffective at finding known defects. I built Testmate to fix that." 一句话点出问题 + 你的解决方案 |
| 1:00–2:30 | 背景与 3 大障碍 | 配置繁琐 / 测试设计知识门槛 / 回归与覆盖率反馈不可见（配合海报 Introduction 逐条指） |
| 2:30–3:30 | 目标与方案 | 4 个 Objectives 快速过；点明"一个 GUI 集成三件事" |
| 3:30–6:30 | **现场演示（核心）** | 打开软件实机演示（若允许）或对着海报流程图讲：导入项目 → 自动检测环境 → 生成测试(Pynguin) → 运行(pytest 实时日志) → 覆盖率视图。**边指海报边讲**，动作要慢 |
| 6:30–8:00 | 结果与展示 | 系统验证成果：13 项功能需求全部实现 + 演示数据（生成测试数、覆盖率提升）+ 现场指截图；讨论 1 句局限性（Python-only、小型项目） |
| 8:00–9:00 | 结论与未来 | 贡献 1 句 + Future work 2 点（CI/CD、多语言） |
| 9:00–10:00 | 收尾 | "To summarize: Testmate turns testing from a chore into a one-click routine." + 感谢评委，邀请提问 |

**T1 赛道（15 分钟）调整：** ① 开场 Hook 延至 1:30（加一段 30 秒的"为什么不直接用 VS Code 插件/pytest CLI"的对比）；② 演示延至 4:30–9:30（展示 Rerun Failed、回归套件保存/一键重跑、覆盖率过滤与导出等进阶功能）；③ 结果讨论加 1–2 分钟（深入讲覆盖率不等于质量的理念）；④ 结尾保留 1 分钟。

**演示铁律（Pitch 成败关键）：**
- 提前录好 30 秒备份演示视频——现场网络/环境出问题立刻切视频，绝不冷场。
- 演示项目用**小而精**的样例（如一个计算器/工具模块），10 秒内能跑完。
- 每个屏幕动作都说一句话解释"为什么这对学生重要"。

### 3.2 高频 Q&A 预测（5 个核心问题 + 回答思路）

**Q1: "覆盖率并不等于测试有效性（Inozemtseva & Holmes 2014），你的工具为什么还主打覆盖率？"**
→ 答：我们刻意把覆盖率定位为"引导"而非"目标"。工具提供行级高亮 + 覆盖率历史 + 回归套件，帮用户看到"哪些代码没被测"，而不是只看一个数字；评估指标也以缺陷发现能力为准，不只以覆盖率为准。

**Q2: "Pynguin 本身就能生成测试，你的工具和直接跑 Pynguin CLI 有什么区别？"**
→ 答：①零配置集成（自动检测 venv/依赖，自动串联 pytest→coverage）；②GUI 实时可视化（进度、日志、结果卡片）；③生成结果直接落入测试目录并可一键运行、一键存为回归套件；④对新手友好的错误提示。Pynguin 是引擎，Testmate 是让引擎可用的工作流。

**Q3: "为什么只支持 Python？为什么不做成 VS Code 插件？"**
→ 答：范围聚焦教育场景的小型 Python 项目（报告 Scope 明确限定）；插件会受宿主 IDE 生态限制，我们的独立桌面应用能完全控制教学体验。多语言支持列入 Future Work。

**Q4: "你的可用性评估怎么做的？样本量多少？指标是什么？"**
→ 答：对照组设计——同一批学生分别用纯 CLI（pytest+coverage.py 手动配置）和 Testmate 完成相同任务，记录完成时间、错误频率、成功率，并发放问卷（含学习曲线、满意度维度）。[填入实际人数 N 和具体数字——这是你被追问最多的地方，必须提前准备好]。
→ **若你确实没做评估，坦诚回答，绝不编数字**："Usability evaluation is part of the ongoing work; this poster focuses on system verification results — all 13 functional requirements are implemented and demonstrated."（同时把话题引回演示截图和覆盖率数据）

**Q5: "为什么选 Tauri/Rust 而不是 Electron？安全模型怎么考虑？"**
→ 答：Tauri 体积小（安装包仅数 MB vs Electron 100MB+）、Rust 提供类型安全 IPC、Python 以隔离子进程方式按需启动（不常驻、干净退出）；Tauri 安全策略用最小 allowlist + 路径校验（对应风险表）。这让教学工具安装零负担。

**备用 Q6: "Pynguin 对复杂代码生成失败怎么办？"** → 答：优雅降级——超时/空结果时给出友好警告并建议缩小目标范围（报告 UC03 异常流 9.2 + 风险表已覆盖）。

**Q&A 通用话术：** 每答必用"**一句话结论 → 证据/数据 → 拉回主题**"三段式；被问倒就大方说 "That's exactly our future work"，不要硬编数字。

---

## 4. 强制性任务指导：2 份他人海报摘要 (Summary)

> 规则要求现场提交 2 份其他参赛者的海报摘要。评委看重的是：**你看懂了别人的研究 + 你有批判性思考 + 你能联系自己的项目**。下面这份"万能模板"帮你 15 分钟内高质量完成一份。

### 万能摘要模板（打印多份带去现场，每份一张）

```
════════════════════════════════════════════════════
SUMMARY OF PEER POSTER  (No. ___)    日期: ________
════════════════════════════════════════════════════
1. Poster Title（照抄海报标题）
   ______________________________________________

2. Presenter / Track（作者名 + 所属方向）
   ______________________________________________

3. One-line takeaway（一句话核心贡献，用我的话）
   ______________________________________________

4. Problem & Motivation（他们解决了什么问题？）
   ______________________________________________

5. Methodology（用了什么方法/工具/数据？1-2 行）
   ______________________________________________

6. Key Results（抄下具体数字！没有数字就写"定性结论"）
   ______________________________________________

7. Strength（哪里做得好？设计/方法/可视化）
   ______________________________________________

8. Weakness / Open question（哪里薄弱？或我没听懂的地方）
   ______________________________________________

9. My inspiration（对我/Testmate 的启发，至少写 1 点）
   ______________________________________________

10. Question I asked the presenter（现场提问记录）
   ______________________________________________
```

### 现场操作 SOP（每人 10–15 分钟）

1. **先拍照**（征求同意），再快速扫读标题、摘要、图表。
2. **搭话破冰**：提一个具体问题（模板第 10 栏），比如 "How did you measure X?" —— 提问是加分项，也让你拿到论文里没有的信息。
3. **抄数字**：Results 区所有百分比、时间、样本量一律照抄，这是摘要含金量的来源。
4. **当场填完 9 栏**，第 9 栏"对我的启发"一定要写——这是评委区分"抄写员"和"思考者"的地方。
5. **两份摘要选差异大的海报**（如一个硬件/AI 方向 + 一个软件方向），展示你的视野广度。
6. 提交前检查：标题准确、数字无误、英文拼写正确。

### 启发句速成公式（第 9 栏直接套用）
> "This poster inspired me to [行动]，because their [做法] shows that [洞察]。For my project, I would [具体改进]。"
> 例："This poster inspired me to add a before/after comparison chart, because their visual comparison made the impact instantly clear. For Testmate, I would show coverage before and after generated tests side by side."

---

## 附：规则符合性检查清单（提交前逐项打勾）

- [ ] 尺寸：Portrait **A1 (594 mm × 841 mm)**（仅此版式允许）
- [ ] 标题加粗、准确反映主题；含学生姓名 + 导师姓名
- [ ] 必备模块齐全：Introduction / Objectives / Methodology / **Results & Discussion** / Conclusion
- [ ] 海报**不依赖口头讲解**即可读懂（评委只看海报也能明白）
- [ ] 图形/公式高分辨率（≥300 DPI），无模糊截图
- [ ] 背景简洁（规则提示彩色背景降低可读性）
- [ ] 已删除模板中所有多余占位内容
- [ ] 2 份他人海报摘要已按模板完成
- [ ] 截止日期按官方通知（Submission Deadline: As announced）
- [ ] 熟悉比赛流程：导师初审 → 评审委员会/FYPC 筛选 → 短名单公示 → 现场展示（特设评委团）→ 各方向颁奖；另有 Most-Liked Poster 网上投票（留意 LKC FES FYP 官网投票链接，海报贴 QR 码可拉票）

---

*本指南由 AI 根据你的报告与官方规则生成；Results 数据与可用性数字请以你的实测为准并替换占位符。*
