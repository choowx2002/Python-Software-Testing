# Testmate FYP — UML 图源文件（PlantUML）

对应论文 Ch.4 / Ch.5 的全部 UML 图，内容与当前代码（git HEAD `6958681`）一致。
梳理方案见 `docs/thesis-uml-plan-2026-08-29.md`。

## 渲染方法

**方式 A（本地）**：需要 Java + plantuml.jar（已下载到 `.docx-tools/plantuml.jar`）

```bash
# 全部渲染为 PNG（200 DPI，Word 缩小后依然清晰）
mkdir -p docs/uml/png
java -jar .docx-tools/plantuml.jar -charset UTF-8 -Sdpi=200 -tpng -o png docs/uml/*.puml

# 或渲染为 SVG（Word 可插入、缩放不糊）
java -jar .docx-tools/plantuml.jar -charset UTF-8 -tsvg -o svg docs/uml/*.puml
```

**Word 排版建议（避免"宽/糊"问题）**：
- 序列图、ERD、用例图：宽度设 **15–16 cm**（接近页宽），200 DPI 下清晰锐利；
- 活动图（4.2–4.6）为竖长条比例（宽:高 ≈ 0.3–0.45）：宽度设 **9–11 cm**，高度自然约 22–25 cm，文字仍清晰；
- 若某张图仍嫌宽，可先精简 `.puml` 里参与者名/消息文本再重渲染（图 4.9 已按此方式从 1484px 收窄到 806px@96dpi）。

**方式 B（在线）**：打开 https://www.plantuml.com/plantuml 或 VS Code 插件（PlantUML），
把任意 `.puml` 内容粘贴即可渲染。

## 文件清单与图号

### Chapter 4（新增 UC05，序列图号顺延）
| 文件 | 图号 | 内容 |
|---|---|---|
| fig4-1-usecase.puml | Figure 4.1 | Use-Case Diagram（UC01–UC05） |
| fig4-2-activity-setup.puml | Figure 4.2 | Activity: Setup Project Environment |
| fig4-3-activity-execute.puml | Figure 4.3 | Activity: Execute Tests |
| fig4-4-activity-view-results.puml | Figure 4.4 | Activity: View Test Results |
| fig4-5-activity-generate.puml | Figure 4.5 | Activity: Generate Unit Tests |
| fig4-6-activity-coverage.puml | Figure 4.6 | Activity: Analyze Code Coverage |
| fig4-7-activity-history.puml | Figure 4.7 | Activity: View History and Trends（新增） |
| fig4-8-sequence-setup.puml | Figure 4.8 | Sequence: Setup Project Environment |
| fig4-9-sequence-execute.puml | Figure 4.9 | Sequence: Execute Tests |
| fig4-10-sequence-view-results.puml | Figure 4.10 | Sequence: View Test Results |
| fig4-11-sequence-generate.puml | Figure 4.11 | Sequence: Generate Unit Tests |
| fig4-12-sequence-coverage.puml | Figure 4.12 | Sequence: Analyze Code Coverage |
| fig4-13-sequence-history.puml | Figure 4.13 | Sequence: View History and Trends（新增） |

### Chapter 5
| 文件 | 图号 | 内容 |
|---|---|---|
| fig5-1-architecture.puml | Figure 5.1 | High-Level Architecture（三运行时 + 8 表 + 事件通道） |
| fig5-2-sequence-env-setup.puml | Figure 5.2 | Sequence: Test Environment Setup |
| fig5-3-sequence-generation.puml | Figure 5.3 | Sequence: Automated Test Generation |
| fig5-4-sequence-execution.puml | Figure 5.4 | Sequence: Test Execution |
| fig5-5-sequence-coverage.puml | Figure 5.5 | Sequence: Automated Coverage Analysis |
| fig5-6-erd.puml | Figure 5.6 | ERD（8 张表 + 9 条关系） |
| fig5-x-component-optional.puml | 可选 | UML Component Diagram（答辩加分项） |

## 注意事项

1. **图号**：因新增 UC05 与两张新图（活动 4.7、序列 4.13），Ch.4 原活动图 4.2–4.6 保持、
   原序列图 4.7–4.11 顺延为 **4.8–4.13** —— 论文正文图号需同步改（对照 `thesis-uml-plan` §4）。
2. **§5.4 已删除**：论文原有两个 Figure 5.6（ERD 与 Dashboard 原型）；已决定删除 Ch.5 §5.4（UI 移入 Ch.6 §6.4），
   Ch.5 图号止于 5.6，重复编号问题随之消失（见 checklist A6）。
3. **用例描述表**（Table 4.4–4.8）不是图，按 `thesis-uml-plan` §2.2 在 Word 中更新文字即可。
4. 每个图的内容都有代码对应（文件:函数），评审追问时可指回具体实现。
