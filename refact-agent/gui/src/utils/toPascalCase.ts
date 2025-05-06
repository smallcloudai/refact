export function toPascalCase(value: string) {
  return value
    .split(/[_\-\s]+/)
    .map((str) => str.charAt(0).toUpperCase() + str.slice(1))
    .join(" ");
}
