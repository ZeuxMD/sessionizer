// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { SettingsPanel } from "./SettingsPanel";
import { getConfig, updateSettings, verifyPassword } from "../lib/invoke";

vi.mock("../lib/invoke", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../lib/invoke")>()),
  getConfig: vi.fn(),
  getAdminPanelInfo: vi.fn(async () => ({
    running: true,
    urls: [],
    error: null,
  })),
  updateSettings: vi.fn(),
  changePassword: vi.fn(),
  verifyPassword: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-autostart", () => ({
  enable: vi.fn(),
  disable: vi.fn(),
  isEnabled: vi.fn(async () => true),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(getConfig).mockResolvedValue({
    timeout_minutes: 60,
    warning_minutes: 5,
    action: "shutdown",
    autostart_enabled: true,
    first_run_complete: true,
    session_start_pending: false,
    timer_start_timestamp: 100,
    timer_paused_at: null,
    pause_reason: null,
    session_expired: false,
    warning_notification_sent: false,
  });
  vi.mocked(verifyPassword).mockResolvedValue(true);
});
afterEach(cleanup);

it("re-authenticates after settings authorization expires and saves the preserved edits", async () => {
  vi.mocked(updateSettings)
    .mockRejectedValueOnce(
      "Service request failed (401): Local authorization expired or is invalid",
    )
    .mockResolvedValueOnce(undefined);
  render(<SettingsPanel onClose={() => {}} />);
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: "Save Settings",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  fireEvent.change(screen.getAllByRole("slider")[0], {
    target: { value: "90" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save Settings" }));
  const password = await screen.findByPlaceholderText("Enter password");
  fireEvent.change(password, { target: { value: "parent-secret" } });
  fireEvent.submit(password.closest("form") as HTMLFormElement);
  await waitFor(() => expect(updateSettings).toHaveBeenCalledTimes(2));
  expect(updateSettings).toHaveBeenLastCalledWith(90, 5, "shutdown", true);
  expect(verifyPassword).toHaveBeenCalledWith("parent-secret");
  await screen.findByText("Settings saved successfully");
});
