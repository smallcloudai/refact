export function trimIndent(str: string): string {
  const firstLine = str.match(/(?:^```.*\n)(\W+)/);
  if (!firstLine) return str;
  if (firstLine.length < 2) return str;
  const indent = firstLine[1];
  const regex = new RegExp("\n" + indent, "gm");
  const result = str.replace(regex, "\n");
  return result;
}
