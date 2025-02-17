export function formatDateToHumanReadable(
  date: string,
  timeZone: string,
  locale?: string,
): string {
  const utcDate = new Date(date);
  return new Intl.DateTimeFormat(locale, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    timeZone: timeZone,
  })
    .format(utcDate)
    .replace(",", "");
}

export const formatDateOrTimeBasedOnToday = (
  isoString: string | null,
  timezone: string,
) => {
  if (!isoString) return "";

  const date = new Date(isoString);
  const now = new Date();
  const isToday =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();

  if (isToday) {
    return new Intl.DateTimeFormat(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
      timeZone: timezone,
    }).format(date);
  }

  return formatDateToHumanReadable(isoString, timezone);
};
