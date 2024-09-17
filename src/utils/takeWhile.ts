function iter<A>(items: A[], fun: (a: A) => boolean, acc: A[] = []): A[] {
  if (items.length === 0) {
    return acc;
  }
  const [head, ...tail] = items;
  if (fun(head)) {
    return iter<A>(tail, fun, [...acc, head]);
  }

  return acc;
}

export function takeWhile<A>(arr: A[], predicate: (a: A) => boolean): A[] {
  return iter<A>(arr, predicate);
}
