# Dev startup speed tips

## Reuse the Vite dev server
1) Terminal A: `npm run dev -- --host 127.0.0.1 --port 1420`
2) Terminal B: `powershell -ExecutionPolicy Bypass -File scripts/tauri-dev-reuse.ps1`
   - The script checks port 1420, sets `TAURI_SKIP_DEVSERVER=1` and `TAURI_DEV_URL=http://127.0.0.1:1420`, then runs `npm run tauri -- dev`.
- One-key start (single terminal): `powershell -ExecutionPolicy Bypass -File scripts/dev-onekey.ps1`
  - Will auto-start Vite if port 1420 is free, wait until ready, then launch Tauri with `TAURI_SKIP_DEVSERVER=1`.

## Rust build cache (optional)
- Install `sccache`. Before starting, set `RUSTC_WRAPPER=sccache`:
  - PowerShell: `$env:RUSTC_WRAPPER="sccache"`
  - CMD: `set RUSTC_WRAPPER=sccache`
- If you want it persistent, create `.cargo/config.toml`:
  ```
  [build]
  rustc-wrapper = "sccache"
  ```
- Do not set the wrapper if `sccache` is not installed.

## Notes
- If port 1420 is not listening, start Vite first (`npm run dev -- --host 127.0.0.1 --port 1420`).
- To fall back to default behavior, run `npm run tauri dev` (without `TAURI_SKIP_DEVSERVER`).
