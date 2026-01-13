# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CheekAI is an AI-generated text detection desktop application built with Rust/Tauri backend and Vue 3 frontend. It analyzes documents (DOCX, TXT) to detect AI-generated content using multiple AI providers (GLM, Deepseek).

## Commands

```bash
# Run the application (development)
npm run tauri dev

# Build for production
npm run tauri build

# Install dependencies
npm install
cd src-tauri && cargo build

# Check Rust code
cd src-tauri && cargo check

# Run Rust tests
cd src-tauri && cargo test
```

## Architecture

**Backend (Rust/Tauri)** - `src-tauri/`
- `src/main.rs` - Application entry point
- `src/lib.rs` - Tauri command registration
- `src/api/detect.rs` - Detection commands (detect_text, detect_dual_mode, preprocess_file)
- `src/api/config.rs` - Configuration commands (get_config, save_config, API key management)
- `src/services/detection/` - Detection module (refactored into submodules):
  - `segment_builder.rs` - Builds detection segments from text blocks
  - `aggregation.rs` - Aggregates segment results, contrast sharpening
  - `comparison.rs` - Compares paragraph and sentence detection results
  - `dual_mode.rs` - Dual mode detection entry point
  - `llm_analyzer.rs` - LLM-based text analysis
- `src/services/text_processor.rs` - Text normalization, sentence splitting, token estimation
- `src/services/providers.rs` - AI provider integration (GLM/Deepseek API calls)
- `src/services/config_store.rs` - Configuration storage
- `src/models/mod.rs` - Data structures and types

**Frontend (Vue 3/TypeScript)** - `src/`
- `src/App.vue` - Main application component (orchestrates child components)
- `src/main.ts` - Vue app entry point
- `src/components/` - UI Components:
  - `TitleBar.vue` - Window title bar with controls
  - `ControlPanel.vue` - Detection settings and controls
  - `TextInput.vue` - Text input area
  - `ResultsPanel.vue` - Detection results display
  - `SettingsModal.vue` - API key configuration modal
  - `LoadingMask.vue` - Loading overlay
- `src/composables/` - Vue composition functions:
  - `useDetection.ts` - Detection logic and state
  - `useProviders.ts` - Provider and API key management
  - `useFileHandler.ts` - File upload handling
  - `useWindow.ts` - Window control functions
- `src/types/` - TypeScript type definitions
- `src/styles/` - CSS styles and variables

## Key Tauri Commands

- `detect_text` - Single text detection with full analysis
- `detect_dual_mode` - Dual mode detection (paragraph + sentence level)
- `preprocess_file` - File preprocessing (DOCX, TXT extraction)
- `get_config` / `save_config` - Configuration management
- `get_providers` - Available AI providers
- `store_api_key` / `get_api_key` / `delete_api_key` - API key management

## Detection Flow

1. Text preprocessing (normalization, language detection)
2. Chunking with overlap
3. Segment-level analysis (LLM judgment, perplexity, stylometry)
4. Aggregation with thresholds (low: 0.65, medium: 0.75, high: 0.85, veryHigh: 0.90)
5. Decision derivation (pass/review/flag)

## Environment Variables

- `GLM_API_KEY` / `DEEPSEEK_API_KEY` - API keys
- `CHEEKAI_GLM_API_KEY` / `CHEEKAI_DEEPSEEK_API_KEY` - Alternative API key env vars

## Coding Conventions

- Rust: snake_case for functions/variables, CamelCase for structs, 4-space indent
- TypeScript/Vue: camelCase for functions/variables, PascalCase for components
- UI strings are in Chinese - preserve when editing
- Tauri commands use `#[tauri::command]` attribute
