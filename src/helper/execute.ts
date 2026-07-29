export function parseArguments(input: string): string[] {
  return input
    .trim()
    .match(/(?:[^\s"]+|"[^"]*")+/g)
    ?.map((arg) => arg.replace(/^"|"$/g, "")) ?? [];
}