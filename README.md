# Sessionizer

A Windows desktop application for parental screen-time management. Sessionizer launches on boot, displays an 800x600 session panel with a countdown timer, and can shut down the PC when time runs out. Adults can enter a password to dismiss the panel and use the PC normally.

## Features

- **Screen Time Control**: Automatically shutdown/sleep the computer after a configurable timeout period
- **Visual Warnings**: Color-coded countdown (blue → orange → red) as time runs low
- **Recovery Key**: 16-character alphanumeric recovery key for password reset
- **System Tray**: Minimize to tray when unlocked; access settings, re-lock, or about
- **Persistent Timer**: Timer survives app crashes, sleep, and hibernation via timestamp-based tracking

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
4. The session panel appears and the timer starts

### Normal Operation

- On boot, the app launches automatically (if autostart is enabled)
- A timer panel displays remaining time until the PC is automatically shut down
- Enter the password to stop the timer
- Access system tray options: Settings, Re-lock, About, Quit
- Re-locking requires the password so the timer cannot be reset accidentally or by a child
- Session time, action when the time runs out (sleep, shutdown, restart) and resetting password can be configured via the settings panel accessed from the system tray

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

Config file location: `%APPDATA%/sessionizer/config.json`

```json
{
  "password_hash": "argon2 hash",
  "recovery_key_hash": "argon2 hash",
  "timeout_minutes": 60,
  "warning_minutes": 5,
  "action": "shutdown",
  "autostart_enabled": true,
  "first_run_complete": true,
  "timer_start_timestamp": null
}
```

## System Requirements

- Windows 10/11

## License

MIT
