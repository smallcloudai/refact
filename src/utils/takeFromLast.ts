function lastIndex<A>(arr: A[], predicate: (a: A) => boolean): number {
  return arr.reduce<number>((acc, cur, index) => {
    if (predicate(cur)) return index;
    return acc;
  }, -1);
}

export function takeFromLast<A>(arr: A[], predicate: (a: A) => boolean): A[] {
  const start = lastIndex(arr, predicate);
  if (start === -1) return [];
  return arr.slice(start + 1);
}
