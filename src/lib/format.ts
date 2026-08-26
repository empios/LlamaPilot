const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

export function formatBytes(bytes: number, fractionDigits = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }

  const exponent = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    BYTE_UNITS.length - 1,
  );
  const value = bytes / 1024 ** exponent;
  const unit = BYTE_UNITS[exponent] ?? "B";

  return `${value.toFixed(exponent === 0 ? 0 : fractionDigits)} ${unit}`;
}

export function formatMebibytes(mib: number): string {
  return formatBytes(mib * 1024 * 1024);
}

export function formatCommit(commit: string | null | undefined): string {
  if (!commit) {
    return "—";
  }
  return commit.slice(0, 7);
}

export function formatTimestamp(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return parsed.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatRelativeTime(value: string | null | undefined): string {
  if (!value) {
    return "never";
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  const seconds = Math.round((Date.now() - parsed.getTime()) / 1000);
  const thresholds: Array<[number, Intl.RelativeTimeFormatUnit]> = [
    [60, "second"],
    [3600, "minute"],
    [86400, "hour"],
    [2592000, "day"],
    [31536000, "month"],
  ];

  const formatter = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

  if (seconds < 45) {
    return "just now";
  }

  for (const [limit, unit] of thresholds) {
    if (Math.abs(seconds) < limit * 60 || unit === "month") {
      const divisor =
        unit === "second"
          ? 1
          : unit === "minute"
            ? 60
            : unit === "hour"
              ? 3600
              : unit === "day"
                ? 86400
                : 2592000;

      if (Math.abs(seconds) < limit) {
        return formatter.format(-Math.round(seconds / divisor), unit);
      }
    }
  }

  return formatter.format(-Math.round(seconds / 31536000), "year");
}

export function pluralize(count: number, singular: string, plural?: string): string {
  return count === 1 ? singular : (plural ?? `${singular}s`);
}
