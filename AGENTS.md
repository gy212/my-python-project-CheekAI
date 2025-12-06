# Repository Guidelines

## Project Structure & Module Organization
- `backend/`: FastAPI service logic, segmentation heuristics, config store, and schemas. Tests live under `backend/tests/*.py`, with fixtures and paragraph block checks covering key preprocess helpers.
- `desktop/`: Electron shell plus renderer assets (`renderer/index.js`, `style.css`, `index.html`) and preload/main entry points. `scripts/` and `samples/` provide auxiliary data and helpers (e.g., `scripts/verify_paragraphs.py`, `samples/test_paragraphs.txt`).
- `docs/` contains human-facing notes (UI spec). `start.py` orchestrates backend+desktop startup, and config metadata lives in `backend/config/api_config.json`.

## Build, Test, and Development Commands
- `python start.py`: launches backend (uvicorn) and desktop (npm run start or bundled electron) together, installing desktop dependencies on demand.
- `python -m pip install -r backend/requirements.txt`: installs Python deps for FastAPI, stylometry, file parsing, and multipart handling.
- `python -m pytest backend/tests/test_paragraph_blocks.py`: runs the paragraph block suite. Use this target when validating preprocessing behavior after changes.
- Electron-specific work: `cd desktop && npm install` then `npm run start`. These commands are triggered automatically by `start.py` if electron packages are missing.

## Coding Style & Naming Conventions
- Python follows standard 4-space indentation, snake_case for functions/variables, CamelCase for Pydantic models, and module names matching functionality (e.g., `backend/app/service.py`).
- JavaScript/renderer follows modern ES modules, prefers `const`/`let`, and keeps DOM queries at top of `renderer/index.js`. Keep translations/labels in place to match UI (Chinese strings).
- No formal linting tool is configured; rely on descriptive function/variable names and consistent spacing. Keep commits small and diff-friendly.

## Testing Guidelines
- Tests live under `backend/tests/`. Naming uses `test_*` functions and `pytest` fixtures for isolated units (e.g., `test_paragraph_blocks.py` reflects the block builder’s heuristics).
- Run `python -m pytest backend/tests/test_paragraph_blocks.py` after any change to paragraph segmentation logic or preprocess helpers.
- If adding new backend features that interact with segmentation or config storage, add targeted pytest cases in the same directory. Keep tests concise and deterministic.

## Commit & Pull Request Guidelines
- Commit messages should describe the change succinctly (e.g., “Fix heading merging in block builder” or “Add python-multipart dependency”). Use present-tense verbs and reference relevant files.
- PRs should include: purpose overview, impacted areas (backend, renderer, config), and mention any required manual steps (npm install, pip install). Attach screenshots only if UI changes are visible.
- For config/API updates, describe how to regenerate `backend/config/api_config.json` (via UI or API) and any needed migrations.
