# Prompt: Build "Sessionizer" — A Windows Startup Lock App

You are building a complete Windows desktop application called **Sessionizer** using Tauri 2.x, React 19, TypeScript, Tailwind CSS 4, and Vite 6. Follow every specification below exactly. Do not skip any file. Produce the entire project in one pass, fully functional.

---

## Purpose

A parental screen-time limiter for young children on Windows. The app launches on boot, shows a fullscreen lock screen with a countdown timer, and shuts down the PC when time runs out. An adult can enter a password to dismiss the lock and use the PC normally.

---

## Tech Stack

- **Tauri 2.x** (Rust backend)
- **React 19 + TypeScript** (frontend)
- **Tailwind CSS 4.x** (styling)
- **Vite 6.x** (bundler)
- **Rust crates:** argon2, serde, serde_json, rand, tauri, tauri-plugin-autostart, tauri-plugin-notification, tauri-plugin-single-instance
- **Package manager:** pnpm

---

## App Flow

### First Launch (no config file exists)
1. App detects no config at `%APPDATA%/sessionizer/config.json`
2. Opens a fullscreen setup wizard (same always-on-top fullscreen window style as lock screen)
3. Adult sets a password and configures timeout duration
4. A random 16-character alphanumeric recovery key is generated, displayed once, and the user must acknowledge they saved it
5. Password hash and recovery key hash (both Argon2id) are saved to config
6. App transitions to the lock screen with the timer running

### Subsequent Launches (on boot)
1. App starts, reads config, shows the fullscreen lock screen immediately
2. Timer starts counting down from the configured `timeout_minutes`
3. The lock screen displays remaining time and a password input
4. At `warning_minutes` remaining (default 5), the UI shifts to an orange warning theme and a system notification is sent
5. At 1 minute remaining, the UI shifts to a red urgent theme
6. At 0 minutes, the configured action is executed (`shutdown -s -t 0` by default)
7. If the correct password is entered at any point, the lock screen is dismissed, the timer is cancelled, and the app minimizes to the system tray

### System Tray (after unlock)
- **Settings** — requires password re-entry, then opens a centered 500×600 non-resizable settings window
- **Re-lock** — shows the lock screen again and resets the timer
- **About** — shows app name and version
- There is NO "Quit" option

### Recovery Flow
- On the lock screen, a small "Forgot password?" link opens a recovery key input
- If the recovery key is correct, the settings panel opens so the password can be reset

### Timer Persistence
- When the timer starts, persist `timer_start_timestamp` (Unix epoch) to config
- On app launch or wake from sleep, calculate remaining time from `now - timer_start_timestamp` rather than relying on an in-memory countdown
- This ensures the timer survives app crashes, sleep, and hibernation
- When the lock is dismissed by password, clear `timer_start_timestamp` from config

---

## File Structure

Create every file listed below:

```
sessionizer/
├── src/
│   ├── components/
│   │   ├── LockScreen.tsx
│   │   ├── PasswordInput.tsx
│   │   ├── CountdownTimer.tsx
│   │   ├── WarningBanner.tsx
│   │   ├── SetupWizard.tsx
│   │   ├── SettingsPanel.tsx
│   │   └── RecoveryPrompt.tsx
│   ├── hooks/
│   │   ├── useCountdown.ts
│   │   └── useConfig.ts
│   ├── lib/
│   │   └── invoke.ts
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands.rs
│   │   ├── config.rs
│   │   ├── password.rs
│   │   └── shutdown.rs
│   ├── icons/
│   │   └── (default Tauri icons are fine)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
├── vite.config.ts
├── tailwind.config.ts
├── tsconfig.json
├── tsconfig.node.json
└── index.html
```

---

## Rust Backend Specifications

### `config.rs`
- Define an `AppConfig` struct:
```rust
pub struct AppConfig {
    pub password_hash: String,
    pub recovery_key_hash: String,
    pub timeout_minutes: u64,       // default: 60
    pub warning_minutes: u64,       // default: 5
    pub action: String,             // "shutdown" | "restart" | "sleep"
    pub autostart_enabled: bool,    // default: true
    pub first_run_complete: bool,   // default: false
    pub timer_start_timestamp: Option<u64>, // Unix epoch or None
}
```
- Config file path: `%APPDATA%/sessionizer/config.json`
- Implement `load_config()` — returns default config if file doesn't exist
- Implement `save_config(config: &AppConfig)` — creates directory if needed, writes JSON
- Use `serde` and `serde_json` for serialization

### `password.rs`
- `hash_password(plain: &str) -> Result<String>` — uses Argon2id with default params
- `verify_password(plain: &str, hash: &str) -> Result<bool>`
- `generate_recovery_key() -> String` — 16 random alphanumeric characters using `rand`

### `shutdown.rs`
- `execute_action(action: &str) -> Result<()>`:
  - `"shutdown"` → runs `shutdown -s -t 0`
  - `"restart"` → runs `shutdown -r -t 0`
  - `"sleep"` → runs `rundll32.exe powrprof.dll,SetSuspendState 0,1,0`
- Uses `std::process::Command`

### `commands.rs`
Expose these Tauri commands:
```rust
#[tauri::command] fn get_config() -> Result<AppConfig, String>
#[tauri::command] fn save_config(config: AppConfig) -> Result<(), String>
#[tauri::command] fn is_first_run() -> Result<bool, String>
#[tauri::command] fn setup_password(password: String, timeout_minutes: u64) -> Result<String, String>
    // Hashes password, generates recovery key, saves config, returns recovery key plaintext
#[tauri::command] fn verify_password(password: String) -> Result<bool, String>
#[tauri::command] fn verify_recovery_key(key: String) -> Result<bool, String>
#[tauri::command] fn change_password(current: String, new_password: String) -> Result<bool, String>
#[tauri::command] fn execute_shutdown(action: String) -> Result<(), String>
#[tauri::command] fn start_timer() -> Result<(), String>
    // Sets timer_start_timestamp to current Unix epoch and saves config
#[tauri::command] fn clear_timer() -> Result<(), String>
    // Sets timer_start_timestamp to None and saves config
#[tauri::command] fn get_remaining_seconds() -> Result<Option<u64>, String>
    // Calculates remaining seconds from timer_start_timestamp + timeout_minutes vs now
    // Returns None if no timer active, 0 if expired
```

### `main.rs`
- Initialize Tauri app with plugins: autostart, notification, single-instance
- Register all commands
- Configure system tray with menu items: Settings, Re-lock, About (no Quit)
- Handle tray menu events:
  - "Settings" → emit event to frontend to show settings (frontend must prompt for password first)
  - "Re-lock" → emit event to frontend to show lock screen and reset timer
  - "About" → emit event to frontend or show a simple dialog
- Configure the main window as the lock screen window:
  - Label: `main`
  - Fullscreen: true
  - Always on top: true
  - Decorations: false
  - Resizable: false
  - Skip taskbar: true
  - Center: true
- Handle window close event: prevent close, minimize to tray instead (only after unlocked)

### `tauri.conf.json`
- App identifier: `com.sessionizer.app`
- App name: `Sessionizer`
- Window config matching the lock screen properties above
- Include permissions for autostart, notification, shell, and all defined commands
- Disable the default close behavior

### `Cargo.toml`
- Include dependencies: tauri, serde, serde_json, argon2, rand, tauri-plugin-autostart, tauri-plugin-notification, tauri-plugin-single-instance, chrono (for timestamp handling)

---

## Frontend Specifications

### `lib/invoke.ts`
- Create typed wrapper functions around `@tauri-apps/api/core` invoke for every Tauri command
- Export typed functions like:
  - `getConfig(): Promise<AppConfig>`
  - `isFirstRun(): Promise<boolean>`
  - `setupPassword(password: string, timeoutMinutes: number): Promise<string>`
  - `verifyPassword(password: string): Promise<boolean>`
  - `verifyRecoveryKey(key: string): Promise<boolean>`
  - `changePassword(current: string, newPassword: string): Promise<boolean>`
  - `executeShutdown(action: string): Promise<void>`
  - `startTimer(): Promise<void>`
  - `clearTimer(): Promise<void>`
  - `getRemainingSeconds(): Promise<number | null>`
- Define an `AppConfig` TypeScript interface matching the Rust struct

### `hooks/useCountdown.ts`
- Custom hook that polls `getRemainingSeconds()` every second
- Returns `{ remainingSeconds: number | null, isWarning: boolean, isUrgent: boolean, isExpired: boolean }`
- `isWarning` = remaining ≤ warning_minutes * 60
- `isUrgent` = remaining ≤ 60
- `isExpired` = remaining ≤ 0
- When expired, automatically calls `executeShutdown(config.action)`

### `hooks/useConfig.ts`
- Fetches config on mount via `getConfig()`
- Returns `{ config: AppConfig | null, loading: boolean, refetch: () => void }`

### `App.tsx`
- State machine with views: `"loading" | "setup" | "lock" | "unlocked"`
- On mount: check `isFirstRun()`
  - If true → show `SetupWizard`
  - If false → show `LockScreen` and call `startTimer()`
- After setup completes → show `LockScreen` and call `startTimer()`
- After successful unlock → set view to `"unlocked"`, call `clearTimer()`, minimize window to tray
- Listen for tray events (Tauri event listener):
  - `"show-settings"` → prompt for password, then show `SettingsPanel`
  - `"re-lock"` → set view to `"lock"`, call `startTimer()`, show window fullscreen
- Prevent keyboard shortcuts: intercept `Alt+F4`, `Ctrl+W`, `Alt+Tab` (best effort) in a `useEffect` with `keydown` listener that calls `e.preventDefault()` on those combos during lock view

### `components/LockScreen.tsx`
- Fullscreen dark overlay (`bg-slate-900`)
- Centered card containing:
  - Lock icon and "Sessionizer" title
  - `CountdownTimer` component
  - `PasswordInput` component
  - Small "Forgot password?" text button at the bottom that toggles `RecoveryPrompt`
- Props: `onUnlock: () => void`

### `components/CountdownTimer.tsx`
- Uses `useCountdown` hook
- Displays time as `MM:SS` in large text
- Shows a progress bar (full = total time, depleting as time passes)
- Normal state: blue/slate theme
- Warning state (≤ warning_minutes): orange theme, gentle pulse animation
- Urgent state (≤ 1 min): red theme, faster pulse animation

### `components/PasswordInput.tsx`
- Password input field with show/hide toggle
- Submit button labeled "Unlock"
- On submit: calls `verifyPassword()`, shows error on failure, calls `onSuccess` on success
- Props: `onSuccess: () => void`
- Show error message for 3 seconds on wrong password, then clear

### `components/WarningBanner.tsx`
- Conditional banner shown when `isWarning` or `isUrgent` is true
- Warning: "Your session is ending soon" (orange)
- Urgent: "Shutting down in less than a minute" (red, pulsing)

### `components/SetupWizard.tsx`
- Multi-step form (use simple state, not a router):
  - Step 1: "Welcome to Sessionizer" + explanation + "Get Started" button
  - Step 2: Set password (input + confirm input) + timeout slider (5–180 minutes, default 60)
  - Step 3: Displays the recovery key in a monospace box + "I've saved this key" checkbox + "Finish" button
- On finish: calls `setupPassword()`, then calls `onComplete` prop
- Props: `onComplete: () => void`

### `components/SettingsPanel.tsx`
- Form with:
  - Timeout slider (5–180 min)
  - Warning slider (1–30 min)
  - Action radio buttons: Shutdown / Restart / Sleep
  - Change password section: current password, new password, confirm new password
  - Autostart toggle
  - Save button
- On save: validates, calls appropriate commands (`save_config`, `change_password` if password fields filled)
- Props: `onClose: () => void`

### `components/RecoveryPrompt.tsx`
- Simple modal/overlay on top of lock screen
- Input for 16-character recovery key
- Submit button
- On success: transitions to settings panel to reset password
- On failure: shows error
- "Cancel" button to go back to lock screen
- Props: `onRecovered: () => void, onCancel: () => void`

### `index.css`
- Tailwind CSS 4 imports
- Add custom animation utilities for the pulse effects on warning/urgent states
- Set `body` to `overflow: hidden`, `margin: 0`, `user-select: none`

---

## Styling Guidelines

- Dark theme throughout: `bg-slate-900` base, `text-white`
- Cards/panels: `bg-slate-800 rounded-2xl p-8 shadow-2xl`
- Primary accent: `blue-500` (normal state)
- Warning accent: `orange-500`
- Urgent accent: `red-500`
- Inputs: `bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white`
- Buttons: `bg-blue-600 hover:bg-blue-700 rounded-lg px-6 py-3 font-semibold`
- Use `backdrop-blur-sm` on the lock overlay for a modern frosted glass effect
- All text should be large and readable (this appears on a fullscreen overlay)
- Smooth transitions between normal/warning/urgent color states using CSS transitions

---

## Build Configuration

### `package.json`
- Name: `sessionizer`
- Scripts: `dev` → `tauri dev`, `build` → `tauri build`, `preview` → `vite preview`
- Dependencies: `react`, `react-dom`, `@tauri-apps/api`, `@tauri-apps/plugin-autostart`, `@tauri-apps/plugin-notification`
- Dev dependencies: `@types/react`, `@types/react-dom`, `typescript`, `vite`, `@vitejs/plugin-react`, `tailwindcss`, `@tailwindcss/vite`, `@tauri-apps/cli`

### `vite.config.ts`
- Standard Tauri 2 + React + Tailwind CSS 4 Vite config
- Set host to `false`, clearScreen to `false`
- Set server port to `1420`

### `tsconfig.json`
- Strict mode enabled
- Target: ES2021
- Module: ESNext
- JSX: react-jsx

---

## Important Implementation Rules

1. **Never store plaintext passwords.** Only Argon2id hashes go in the config file.
2. **The lock screen must be impossible to dismiss without the password** (within reason for a child — no close button, no taskbar, always on top, Alt+F4 intercepted).
3. **Timer must be timestamp-based**, not interval-based. Always calculate remaining time from `timer_start_timestamp + timeout_minutes * 60 - now`. This survives sleep, hibernate, and app restart.
4. **All Tauri commands must return `Result` types** with proper error handling. Frontend should handle errors gracefully with user-facing messages.
5. **The app must be single-instance.** If already running, the second instance should focus the existing window and exit.
6. **Use Tauri 2.x API and plugin syntax** (not Tauri 1.x). This means `@tauri-apps/api/core` for invoke, `@tauri-apps/plugin-*` for plugins, and the v2 Rust plugin registration syntax.
7. **All code must be complete and functional.** Do not leave TODO comments, placeholder functions, or unimplemented sections. Every file must be production-ready.
8. **Do not create a router or use react-router.** Use simple React state to switch between views.

---

## Produce every file listed in the file structure above with complete, working code. Start with the Rust backend, then the frontend, then configuration files.
