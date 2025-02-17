/**
 * Extracts and formats the last part of a path with optional prefix.
 * @param path - Full path to extract from
 * @param prefix - Optional prefix to add before the extracted name (e.g., ".../")
 * @param suffix - Optional suffix to add after the extracted name (e.g., "/")
 * @returns Formatted path name with optional prefix
 * @example
 * formatPathName("/user/projects/myproject/file.txt") // "file.txt"
 * formatPathName("/user/projects/myproject/file.txt", ".../") // ".../file.txt"
 * formatPathName("C:\\Users\\name\\project", ".../") // ".../project"
 */
export function formatPathName(path: string, prefix = "", suffix = ""): string {
  if (!path) return "";

  // Split by both forward and backward slashes
  const parts = path.split(/[/\\]/);
  const lastPart = parts[parts.length - 1];

  return prefix + lastPart + suffix;
}
