# Sessionizer

A Windows desktop application for parental screen-time management. Sessionizer launches on boot, displays an 800x600 session panel with a countdown timer, and can shut down the PC when time runs out. Adults can enter a password to dismiss the panel and use the PC normally.

## Features

- **Screen Time Control**: Automatically shutdown/sleep the computer after a configurable timeout period
- **Visual Warnings**: Color-coded countdown (blue → orange → red) as time runs low
- **Recovery Key**: 16-character alphanumeric recovery key for password reset
- **System Tray**: Minimize to tray when unlocked; access settings, re-lock, or about
- **Persistent Timer**: Timer survives app crashes, sleep, and hibernation via timestamp-based tracking
- **Desktop Notifications**: Shows a one-time warning notification when time is almost up
- **Protected Pause**: Adults can pause a child session after entering the password
- **Remote Admin Panel**: Open a phone-friendly control panel over the local network using the same app password

## Tech Stack

- **Tauri 2.x** — Rust backend with system integration
- **React 19** + TypeScript — Frontend UI
- **Tailwind CSS 4** — Styling
- **Vite 6** — Build tool
- **Argon2id** — Password hashing

## Installation

- Go to the [releases page](https://github.com/ZeuxMD/sessionizer/releases)

### Prerequisites

- Node.js (LTS)
- pnpm
- Rust toolchain
- Windows OS

### Build from Source

```bash
# Install dependencies
pnpm install

# Development mode
pnpm dev

# Build for production
pnpm build:tauri
```

The built executable will be at `src-tauri/target/release/sessionizer.exe`.

## Usage

### First Launch

1. Run the application — the setup wizard appears
2. Set a password and configure the timeout duration (default: 60 minutes)
3. A recovery key is generated — save this in a safe place
4. Finishing setup starts the first session immediately

### Normal Operation

- On Windows login, the app launches automatically (if autostart is enabled) and starts a fresh child session
- A timer panel displays remaining time until the PC is automatically shut down
- Enter the password to clear the current child session and dismiss the panel
- Use "Pause Session" to freeze the current child session after password confirmation
- Resume a paused session from the tray with "Resume Session"
- Sessionizer sends one desktop notification when the remaining time first reaches the warning window
- Access system tray options: Settings, Re-lock, About, Quit
- Open the remote admin panel from **Settings → Remote Admin** on your phone using the same password as the desktop app
- Re-locking requires the password and starts a fresh child session
- Logging out or putting the PC to sleep pauses the session and preserves the remaining time
- Shutting down or restarting the PC resets the current child session instead of resuming it later
- Session time, action when the time runs out (sleep, shutdown, restart) and resetting password can be configured via the settings panel accessed from the system tray
- The remote admin panel can pause/resume the timer, add or subtract time, unlock/re-lock the session, and update the runtime settings over the LAN

### Timer States

| State | Time Remaining | Theme |
|-------|----------------|-------|
| Normal | > warning_minutes | Blue/Slate |
| Warning | ≤ warning_minutes | Orange (pulsing) |
| Urgent | ≤ 1 minute | Red (fast pulse) |
| Expired | 0 | Action executed |

### Recovery

1. Click "Forgot password?" on the session panel
2. Enter the 16-character recovery key
3. Set a new password

## Configuration

Machine state file location: `%ProgramData%/Sessionizer/device-state.json`

Legacy installs that still use `%APPDATA%/sessionizer/config.json` are migrated to the
machine-scoped state file automatically the next time the app starts.

```json
{
  "password_hash": "argon2 hash",
  "recovery_key_hash": "argon2 hash",
  "timeout_minutes": 60,
  "warning_minutes": 5,
  "action": "shutdown",
  "autostart_enabled": true,
  "first_run_complete": true,
  "session_start_pending": false,
  "timer_start_timestamp": null,
  "timer_paused_at": null,
  "pause_reason": null,
  "session_expired": false,
  "warning_notification_sent": false,
  "remote_admin_enabled": true
}
```

## System Requirements

- Windows 10/11

## License

MIT
