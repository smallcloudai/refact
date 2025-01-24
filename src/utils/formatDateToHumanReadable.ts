export function convertUTCToLocalTime(date: string): Date {
  const utcDate = new Date(date);
  const timezoneOffset = utcDate.getTimezoneOffset() * 60000; // Convert to milliseconds
  const localDate = new Date(utcDate.getTime() - timezoneOffset);
  return localDate;
}

export function formatDateToHumanReadable(
  date: string,
  timeZone: string,
  locale = "en-GB",
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
