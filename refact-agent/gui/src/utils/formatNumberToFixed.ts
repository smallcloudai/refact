export function formatNumberToFixed(num: number, toFixed = 2): string {
  const MILLION = 1_000_000;
  const THOUSAND = 1_000;

  if (num >= MILLION) {
    const millions = num / MILLION;
    return (
      (num % MILLION === 0 ? millions.toFixed(0) : millions.toFixed(toFixed)) +
      "M"
    );
  } else if (num >= THOUSAND) {
    const thousands = num / THOUSAND;
    return (
      (num % THOUSAND === 0
        ? thousands.toFixed(0)
        : thousands.toFixed(toFixed)) + "k"
    );
  } else {
    return num.toString();
  }
}
