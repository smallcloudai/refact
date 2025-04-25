export function hasProperty<T extends string>(
  obj: object,
  prop: T,
): obj is { [K in T]: unknown } {
  return prop in obj;
}
