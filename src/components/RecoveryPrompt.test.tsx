// @vitest-environment jsdom
import { act } from "react";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RecoveryPrompt } from "./RecoveryPrompt";
import { LockScreen } from "./LockScreen";
import {
  resetPasswordWithRecovery,
  verifyPassword,
  verifyRecoveryKey,
} from "../lib/invoke";

vi.mock("../lib/invoke", () => ({
  verifyRecoveryKey: vi.fn(),
  resetPasswordWithRecovery: vi.fn(),
  verifyPassword: vi.fn(),
  getRemainingSeconds: vi.fn(async () => 300),
}));

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(verifyRecoveryKey).mockResolvedValue(true);
  vi.mocked(resetPasswordWithRecovery).mockResolvedValue(true);
  vi.mocked(verifyPassword).mockResolvedValue(true);
});
afterEach(cleanup);

async function submitRecovery(onRecovered?: () => void | Promise<void>) {
  if (onRecovered) {
    render(<RecoveryPrompt onRecovered={onRecovered} onCancel={() => {}} />);
  }
  fireEvent.change(screen.getByPlaceholderText("Enter recovery key"), {
    target: { value: "ABCDEFGHIJKLMNOP" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Continue" }));
  await screen.findByPlaceholderText("New password");
  fireEvent.change(screen.getByPlaceholderText("New password"), {
    target: { value: "new-parent-secret" },
  });
  fireEvent.change(screen.getByPlaceholderText("Confirm new password"), {
    target: { value: "new-parent-secret" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Reset Password" }));
}

describe("password recovery", () => {
  it("authenticates with the new password before unlocking and waits for unlock", async () => {
    let completeUnlock: () => void = () => {};
    const pending = new Promise<void>((resolve) => {
      completeUnlock = resolve;
    });
    const onRecovered = vi.fn(() => pending);
    await submitRecovery(onRecovered);
    await waitFor(() => expect(onRecovered).toHaveBeenCalledOnce());
    expect(verifyPassword).toHaveBeenCalledWith("new-parent-secret");
    expect(vi.mocked(verifyPassword).mock.invocationCallOrder[0]).toBeLessThan(
      onRecovered.mock.invocationCallOrder[0],
    );
    expect(screen.getByRole("button", { name: "Saving..." })).toBeDefined();
    await act(async () => {
      completeUnlock();
      await pending;
    });
  });

  it("keeps recovery open if authorization fails", async () => {
    vi.mocked(verifyPassword).mockResolvedValue(false);
    const onRecovered = vi.fn();
    await submitRecovery(onRecovered);
    await screen.findByText(/could not authorize/i);
    expect(onRecovered).not.toHaveBeenCalled();
  });
});

it("keeps the lock screen recovery dialog open until the actual unlock completes", async () => {
  let finishUnlock: () => void = () => {};
  const pendingUnlock = new Promise<void>((resolve) => {
    finishUnlock = resolve;
  });
  const unlock = vi.fn(() => pendingUnlock);
  render(
    <LockScreen
      onUnlock={unlock}
      onPause={() => {}}
      timeoutMinutes={60}
      warningMinutes={5}
      isSessionExpired={false}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Forgot password?" }));
  await submitRecovery();
  await waitFor(() => expect(unlock).toHaveBeenCalledOnce());
  expect(screen.getByRole("heading", { name: "Reset Password" })).toBeDefined();
  expect(screen.getByRole("button", { name: "Saving..." })).toBeDefined();
  await act(async () => {
    finishUnlock();
    await pendingUnlock;
  });
  await waitFor(() =>
    expect(
      screen.queryByRole("heading", { name: "Reset Password" }),
    ).toBeNull(),
  );
});
