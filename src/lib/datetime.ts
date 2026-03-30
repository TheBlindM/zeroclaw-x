import { i18n } from "@/i18n";

export function parseStoredDate(value: string) {
  if (/^\d+$/.test(value)) {
    return new Date(Number(value));
  }

  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

export function formatTimestamp(value: string, options?: Intl.DateTimeFormatOptions) {
  const date = parseStoredDate(value);
  if (!date) {
    return value;
  }

  return new Intl.DateTimeFormat(i18n.global.locale.value, options).format(date);
}
