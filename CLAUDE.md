# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

A Rust web application for recording plant data via natural language descriptions. Uses Axum web framework, DeepSeek API for NLP, in-memory store (with SQLite module unused). Includes frontend static HTML pages.

## Key Business Rules

Based on `需求.md`:
- Only record what the user explicitly says they did or observed, no speculation.
- Every data change requires user confirmation via `/api/record/:id/confirm`.
- Data schema includes multiple tables: 品种档案 (plant archive), 生长日志 (growth logs), 产量记录 (yield records), etc.
- Plant types are identified by shorthand names (简称) from the archive.

## Development Setup

1. Install Rust via rustup.
2. Copy `.env.example` to `.env` and set `DEEPSEEK_API_KEY` (or use `mock` for mock mode).
3. Run `cargo run` to start server on port 3000.

## Common Commands

- `cargo run`: Start development server
- `cargo test`: Run tests (none currently)
- `cargo build`: Build project
- `cargo check`: Check compilation without building

## Architecture

- `src/main.rs`: Entry point, sets up Axum router, CORS, static file serving.
- `src/handlers.rs`: HTTP handlers for `/api/record`, `/api/record/:id/confirm`, `/api/records`.
- `src/deepseek.rs`: DeepSeek API client with mock mode when API key is `"mock"`.
- `src/store.rs`: In-memory store using `tokio::sync::Mutex` and `Vec`. Stores `PlantRecord` and `PlantData`.
- `src/db.rs`: SQLite module (currently unused) that could be used for persistent storage.

State is shared via `Arc<AppState>` containing `DeepSeekClient` and `Arc<Store>`.

## Project Structure

- `static/`: Frontend HTML files (index.html, login.html)
- `data/`: Empty directory possibly for future database files
- `.env.example`: Environment variable template
- `CONFIGURATION.md`: DeepSeek API configuration guide
- `需求.md`: Chinese requirements document detailing plant data schema

## Environment Variables

- `DEEPSEEK_API_KEY`: DeepSeek API key (set to `"mock"` for mock mode)
- `DEEPSEEK_BASE_URL`: API base URL (defaults to https://api.deepseek.com)
- `PORT`: Server port (default 3000)

## Testing

No tests currently implemented. Use `cargo test` when added.

## Database

Currently uses in-memory store. SQLite module exists but unused. Future persistence could integrate `db.rs` with SQLx.