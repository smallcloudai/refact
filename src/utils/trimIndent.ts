export function trimIndentFromMarkdown(str: string): string {
  const firstLine = str.match(/(?:^```.*\n+)(\W+)/);
  if (!firstLine) return str;
  if (firstLine.length < 2) return str;
  const indent = firstLine[1];
  const regex = new RegExp("\n" + indent, "gm");
  const result = str.replace(regex, "\n");
  return result;
}

export function trimIndent(str: string) {
  const firstLine = str.match(/^[ \t]*/);
  if (!firstLine) return str;
  const [indent] = firstLine;
  if (!indent) return str;
  const regexp = new RegExp("^" + indent, "gm");
  return str.replace(regexp, "");
}
