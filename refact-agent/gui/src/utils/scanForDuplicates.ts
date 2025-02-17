export function scanFoDuplicatesWith<T>(
  arr: T[],
  predicate: (a: T, b: T) => boolean,
): boolean {
  if (arr.length === 0) return false;
  if (arr.length === 1) return false;
  const [head, ...tail] = arr;
  const hasDuplicate = tail.some((item) => predicate(head, item));
  if (hasDuplicate) return true;
  return scanFoDuplicatesWith(tail, predicate);
}
