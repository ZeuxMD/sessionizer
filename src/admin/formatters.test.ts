import { describe, expect, it } from "vitest";
import {
  formatRemainingTime,
  sessionStateCopy,
  sessionStateLabel,
} from "./formatters";
import type { AdminSessionSnapshot } from "./api";

const baseSession: AdminSessionSnapshot = {
  session_state: "locked",
  remaining_seconds: 3_300,
  timeout_minutes: 60,
  warning_minutes: 5,
  action: "shutdown",
  autostart_enabled: true,
  first_run_complete: true,
  session_start_pending: false,
  timer_start_timestamp: 1,
  timer_paused_at: null,
  pause_reason: null,
  session_expired: false,
  warning_notification_sent: false,
};

describe("formatRemainingTime", () => {
  it("formats long durations with hours", () => {
    expect(formatRemainingTime(3_661)).toBe("1:01:01");
  });

  it("formats short durations without hours", () => {
    expect(formatRemainingTime(59)).toBe("00:59");
  });

  it("uses Idle when no timer is active", () => {
    expect(formatRemainingTime(null)).toBe("Idle");
  });
});

describe("session labels", () => {
  it("describes paused sessions using their pause reason", () => {
    const paused = {
      ...baseSession,
      session_state: "paused" as const,
      pause_reason: "manual" as const,
    };

    expect(sessionStateLabel(paused)).toBe("Paused by Adult");
    expect(sessionStateCopy(paused)).toContain("timer is frozen");
  });
});
