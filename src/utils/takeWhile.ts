function iter<A, T extends A>(
  items: A[],
  fun: (a: A) => a is T,
  acc: T[] = [],
): T[] {
  if (items.length === 0) {
    return acc;
  }
  const [head, ...tail] = items;
  if (fun(head)) {
    return iter(tail, fun, [...acc, head]);
  }

  return acc;
}

export function takeWhile<A, T extends A>(
  arr: A[],
  predicate: (a: A) => a is T,
): T[] {
  return iter<A, T>(arr, predicate);
}
