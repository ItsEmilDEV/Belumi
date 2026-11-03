# ADR-007: Launcher Technology Evaluation

**Status:** Pending
**Date:** 2026-10-30

## Context

Belumi needs its own standalone launcher. Multiple UI technologies are candidates.

## Candidates

1. **Tauri (Rust + TypeScript):** Flexible modern desktop UIs, web technologies
   for complex layouts, Rust for installation logic.
2. **Slint (Rust):** Declarative UI with native Rust integration, cross-platform.
3. **Iced (Rust):** Rust-oriented architecture, async support.
4. **egui (Rust):** Fast prototyping for settings and developer tools.

## Evaluation Criteria

1. Startup time and idle RAM usage
2. Smoothness of animations and interactions
3. Quality and freedom of UI design
4. Windows, Linux, and macOS support
5. Maintainability and ecosystem size
6. Accessibility and keyboard navigation
7. System integration and process control
8. Testability and automated builds
9. Dependencies and license conditions
10. Development effort for the existing project

## Decision

Pending. A documented comparison and small prototype per serious candidate
will be created before the final decision.

## Rationale

Technology is not selected based on language or hype alone. The launcher needs
a high-quality UI and reliable system integration. Evaluation must be based on
measurable results and documented trade-offs.