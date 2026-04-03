import { describe, expect, it } from "vitest";

import { getWarningNotificationBody } from "./warningNotification";

describe("getWarningNotificationBody", () => {
  it("switches to urgent copy at one minute or less", () => {
    expect(getWarningNotificationBody(60)).toBe("Less than a minute remaining");
    expect(getWarningNotificationBody(12)).toBe("Less than a minute remaining");
  });

  it("rounds up to whole minutes for longer durations", () => {
    expect(getWarningNotificationBody(61)).toBe("2 minutes remaining");
    expect(getWarningNotificationBody(121)).toBe("3 minutes remaining");
  });
});
