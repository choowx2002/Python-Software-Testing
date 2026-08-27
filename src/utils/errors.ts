/**
 * 将底层（Rust/pytest/python）抛出的原始错误转成对用户友好的提示。
 * 保留原始 message，并在可识别时附上 hint。
 */
export interface InterpretedError {
  message: string;
  hint?: string;
}

const RULES: { pattern: RegExp; hint: string }[] = [
  {
    pattern: /(pytest|pynguin|coverage).*(not found|no such|does not exist)/i,
    hint: "相关工具未安装或不在 PATH 中，请在环境中安装依赖（pip install pytest/pynguin/coverage）。",
  },
  {
    pattern: /no module named (.+)/i,
    hint: "缺少 Python 依赖：$1。请在项目虚拟环境中安装该包。",
  },
  {
    pattern: /(python|interpreter).*(not found|does not exist|no such file)/i,
    hint: "Python 解释器路径已失效。请重新检测环境或使用\"一键修复\"。",
  },
  {
    pattern: /permission denied/i,
    hint: "没有文件访问权限。请检查目录权限后重试。",
  },
  {
    pattern: /failed to start/i,
    hint: "进程启动失败。请确认解释器路径正确且相关工具已安装。",
  },
  {
    pattern: /connection|network|pypi|timeout/i,
    hint: "可能是网络问题。请检查网络或配置 pip 镜像源后重试。",
  },
  {
    pattern: /No data to report/i,
    hint: "本次运行的测试没有导入所选源文件。请选择测试实际使用到的文件，或全选源文件。",
  },
  {
    pattern: /import file mismatch/i,
    hint: "存在同名测试模块冲突。已尝试通过 __init__.py 修复，请重新扫描。",
  },
];

export function interpretError(raw: unknown): InterpretedError {
  const message = raw instanceof Error ? raw.message : String(raw ?? "");
  for (const rule of RULES) {
    if (rule.pattern.test(message)) {
      const hint = rule.hint.includes("$1")
        ? message.replace(rule.pattern, rule.hint)
        : rule.hint;
      return { message, hint };
    }
  }
  return { message };
}
