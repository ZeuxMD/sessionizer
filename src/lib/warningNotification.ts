export function getWarningNotificationBody(remainingSeconds: number): string {
  if (remainingSeconds <= 60) {
    return "Less than a minute remaining";
  }

  const remainingMinutes = Math.ceil(remainingSeconds / 60);
  return `${remainingMinutes} minute${remainingMinutes === 1 ? "" : "s"} remaining`;
}
