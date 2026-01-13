# Repository Guidelines

## Project Structure & Module Organization
- `src-tauri/`: Rust backend with Tauri integration. Core detection logic lives in `src/services/detection/` (split into segment_builder, aggregation, comparison, dual_mode, llm_analyzer). AI providers in `src/services/providers.rs`, text processing in `src/services/text_processor.rs`.
- `src/`: Vue 3 + TypeScript frontend. Components in `src/components/` (TitleBar, ControlPanel, TextInput, ResultsPanel, SettingsModal, LoadingMask). Composables in `src/composables/` (useDetection, useProviders, useFileHandler, useWindow). Types in `src/types/`.
- `legacy-python-electron/`: Archived Python/FastAPI + Electron code for reference.

## Build, Test, and Development Commands
- `npm run tauri dev`: Development mode with hot reload for both frontend and backend.
- `npm run tauri build`: Build production installer.
- `cd src-tauri && cargo check`: Check Rust code for errors.
- `cd src-tauri && cargo test`: Run Rust unit tests.
- `npm run build`: Build frontend only.

## Coding Style & Naming Conventions
- Rust follows 4-space indentation, snake_case for functions/variables, CamelCase for structs, module names matching functionality (e.g., `segment_builder.rs`, `dual_mode.rs`).
- TypeScript/Vue follows camelCase for functions/variables, PascalCase for components and types. Composables use `use*` prefix.
- UI strings are in Chinese - preserve when editing.
- Each file should have a single responsibility. Split large files into focused modules.

## Testing Guidelines
- Rust tests live alongside code using `#[cfg(test)]` modules.
- Run `cargo test` in `src-tauri/` after changes to detection or service logic.
- Keep tests concise and deterministic.

## Commit & Pull Request Guidelines
- Commit messages should describe the change succinctly (e.g., "Split detection.rs into modular files" or "Add useProviders composable"). Use present-tense verbs.
- PRs should include: purpose overview, impacted areas (backend/frontend), and any breaking changes.
- For Tauri command changes, update both Rust handlers and TypeScript invoke calls.
