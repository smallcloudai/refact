export function parseOrElse<T>(
  str: string,
  fallback: T,
  guard?: (a: T) => boolean,
): T {
  try {
    const result = JSON.parse(str) as T;
    if (guard && !guard(result)) return fallback;
    return result;
  } catch {
    return fallback;
  }
}
