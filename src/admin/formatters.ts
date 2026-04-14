import type { AdminSessionSnapshot } from "./api";

export function formatRemainingTime(remainingSeconds: number | null) {
  if (remainingSeconds === null) {
    return "Idle";
  }

  const total = Math.max(0, remainingSeconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds
      .toString()
      .padStart(2, "0")}`;
  }

  return `${minutes.toString().padStart(2, "0")}:${seconds
    .toString()
    .padStart(2, "0")}`;
}

export function sessionStateLabel(session: AdminSessionSnapshot) {
  switch (session.session_state) {
    case "setup":
      return "Setup Required";
    case "unlocked":
      return "Unlocked";
    case "locked":
      return "Running";
    case "paused":
      return session.pause_reason === "system"
        ? "Paused by System"
        : "Paused by Adult";
    case "expired":
      return "Expired";
  }
}

export function sessionStateCopy(session: AdminSessionSnapshot) {
  switch (session.session_state) {
    case "setup":
      return "Finish setup on the desktop app before using remote controls.";
    case "unlocked":
      return "The computer is available. Start a fresh lock to begin a new child session.";
    case "locked":
      return "The timer is running. Use the controls below to intervene remotely.";
    case "paused":
      return "The timer is frozen. Resume it or adjust the remaining time.";
    case "expired":
      return "Time is up. Unlock the session or start a fresh lock.";
  }
}
