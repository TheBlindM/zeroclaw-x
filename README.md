# ZeroClawX

ZeroClawX is now scaffolded as a Tauri 2 + Vue 3 desktop workspace based on the implementation plan in `implementation_plan.md`.

## Included in this scaffold

- Vue 3 + TypeScript + Vite shell
- Sidebar layout, top bar, route structure, and settings shell
- Pinia stores for app preferences and chat state
- Chat view wired to Tauri event streaming
- Rust backend with chat commands, SQLite bootstrap, and a simulated response stream
- Tauri tray menu and capability/config placeholders

## Current behavior

- Frontend opens into a desktop-style chat workspace.
- Sending a message invokes the Rust command `send_message`.
- The Rust side emits `chat:token` events and finishes with `chat:done`.
- Messages and sessions are persisted to the local SQLite database under the app data directory.

## What is still placeholder

- The actual `zeroclawlabs` runtime integration is represented by `src-tauri/src/services/runtime.rs`.
- shadcn-vue components are implemented as local starter components rather than generated copies.
- Projects, settings, models, channels, MCP, skills, cron, and proxy flows are staged but not implemented yet.

## Suggested next steps

1. Install dependencies with `npm install` and `cargo check` once network access is available.
2. Replace the simulated runtime service with the real in-process ZeroClaw crate.
3. Extend the database schema for projects, sessions, and provider configuration.
4. Swap starter UI components for generated shadcn-vue components if desired.
