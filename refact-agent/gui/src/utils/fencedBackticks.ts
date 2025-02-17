function iter(lines: string[], processed: string[] = []): string[] {
  let remainingLines = lines;
  let currentProcessed = processed;

  while (remainingLines.length > 0) {
    const [head, ...tail] = remainingLines;

    if (!head.startsWith("```")) {
      currentProcessed = [...currentProcessed, head];
    } else {
      const escapedHead = ["````", head];
      currentProcessed = [...currentProcessed, ...escapedHead];
    }

    remainingLines = tail;
  }
  return currentProcessed;
}

export function fenceBackTicks(text: string) {
  const lines = text.split("\n");
  const processed = iter(lines);
  return processed.join("\n");
}
