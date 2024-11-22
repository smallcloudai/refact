function iter(lines: string[], processed: string[] = []): string[] {
  if (lines.length === 0) return processed;
  const [head, ...tail] = lines;

  if (!head.startsWith("```")) return iter(tail, [...processed, head]);

  const escapedHead = ["````", head];

  return iter(tail, [...processed, ...escapedHead]);
}

export function fenceBackTicks(text: string) {
  const lines = text.split("\n");
  const processed = iter(lines);
  return processed.join("\n");
}
