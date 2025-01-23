const absolutePathRegex = /^(?:[a-zA-Z]:\\|\/|\\\\|\/\/).*/;
export function isAbsolutePath(path: string): boolean {
  return absolutePathRegex.test(path);
}
