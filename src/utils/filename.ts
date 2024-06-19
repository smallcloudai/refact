export function filename(fullPath: string): string {
  return fullPath.replace(/^(.*[/\\])?/, "");
}
