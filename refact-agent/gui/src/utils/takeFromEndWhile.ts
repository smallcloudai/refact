function iter<T>(
  arr: T[],
  predicate: (item: T) => boolean,
  acc: T[] = [],
): T[] {
  if (arr.length === 0) return acc;

  const head = arr.slice(-1)[0];
  if (!predicate(head)) return acc;

  const tail = arr.slice(0, -1);
  return iter(tail, predicate, [head, ...acc]);
}

export function takeFromEndWhile<T>(
  arr: T[],
  predicate: (item: T) => boolean,
): T[] {
  return iter(arr, predicate);
}
