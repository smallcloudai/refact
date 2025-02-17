function iter<T, Left extends T = T, Right extends T = T>(
  array: T[],
  predicate: (a: T) => boolean,
  processed: (Left[] | Right[])[] = [[], []],
): (Left[] | Right[])[] {
  if (array.length === 0) return processed;
  const [head, ...tail] = array;
  const [left, right] = processed;

  if (predicate(head)) {
    // const group = [...lastArray, head] as A[];
    const nextRight = [...right, head] as Right[];

    const next: (Left[] | Right[])[] = [left, nextRight];

    return iter<T, Left, Right>(tail, predicate, next);
  }

  const nextLeft = [...left, head] as Left[];
  return iter<T, Left, Right>(tail, predicate, [nextLeft, right]);
}

export function partition<T, Left extends T = T, Right extends T = T>(
  array: T[],
  condition: (a: T) => boolean,
): (Left[] | Right[])[] {
  return iter<T, Left, Right>(array, condition);
}
