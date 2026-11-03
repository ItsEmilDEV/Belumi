# Implementation Plan

## Overview

This document provides a prioritized implementation plan for Belumi based on
the product vision and architecture. Each task includes prerequisites, expected
results, and verifiable acceptance criteria.

**Status:** Planning document. Tasks are not yet implemented unless marked.

---

## Phase 0: Audit and Foundation (Current — 0.1.0)

### Task 0.1: Repository Analysis ✅

**Status:** Completed
**Prerequisites:** None
**Description:** Analyze Horizon upstream repository structure, crates, engine,
rendering, networking, UI, asset management, and extension systems.
**Acceptance criteria:**
- [x] Horizon upstream cloned and analyzed
- [x] `docs/UPSTREAM.md` created with verified findings
- [x] Integration strategy documented (Git submodule)

### Task 0.2: Project Scaffolding ✅

**Status:** Completed
**Prerequisites:** Task 0.1
**Description:** Set up professional Rust project structure.
**Acceptance criteria:**
- [x] Cargo workspace configured
- [x] `.gitignore`, `.editorconfig`, `rustfmt.toml`, `clippy.toml` created
- [x] `README.md`, `LICENSE`, `CHANGELOG.md`, `CONTRIBUTING.md` created
- [x] GitHub Actions CI configured
- [x] `docs/` directory with ARCHITECTURE, ROADMAP, UPSTREAM, DECISIONS

### Task 0.3: belumi-core Foundation ✅

**Status:** Completed
**Prerequisites:** Task 0.2
**Description:** Implement core crate with configuration, logging, and paths.
**Acceptance criteria:**
- [x] `belumi-core` crate compiles
- [x] TOML configuration with validation and defaults
- [x] `tracing` logging with JSON support
- [x] Platform-independent path resolution
- [x] 13 unit tests passing
- [x] Clippy clean, format check passing

### Task 0.4: Documentation Completion ✅

**Status:** Completed
**Prerequisites:** Task 0.3
**Description:** Complete all required documentation.
**Acceptance criteria:**
- [x] `docs/product-vision.md` created
- [x] `docs/decisions/` directory with 9 ADRs
- [x] `docs/decisions/README.md` index created
- [x] `docs/implementation-plan.md` created

---

## Phase 1: Launcher Technology Evaluation (0.2.0)

### Task 1.1: Technology Comparison Document

**Status:** Not started
**Prerequisites:** Task 0.4
**Description:** Create a documented comparison of launcher UI technologies
(Tauri, Slint, Iced, egui) based on the 10 evaluation criteria.
**Acceptance criteria:**
- [ ] `docs/decisions/ADR-010-launcher-tech-selection.md` with comparison matrix
- [ ] Each technology evaluated against all 10 criteria

---

## Phase 2: belumi-qol Crate (0.2.0)

### Task 2.1: QoL Feature Design

**Status:** Not started
**Prerequisites:** Task 0.4
**Description:** Design QoL features (minimap, zoom, FPS counter, coordinates HUD)
as a modular crate.
**Acceptance criteria:**
- [ ] Feature list with priorities
- [ ] Module boundaries defined
- [ ] Configuration schema designed
- [ ] Integration points with Horizon identified

### Task 2.2: belumi-qol Crate Implementation

**Status:** Not started
**Prerequisites:** Task 2.1, Horizon integration (path deps)
**Description:** Implement `belumi-qol` crate with selected QoL features.
**Acceptance criteria:**
- [ ] `belumi-qol` crate created and compiles
- [ ] At least 2 QoL features implemented
- [ ] Unit tests for each feature
- [ ] Clippy clean, format check passing
- [ ] Documentation for each feature

---

## Phase 3: Horizon Integration (0.2.0-0.3.0)

### Task 3.1: Path Dependency Setup

**Status:** Not started
**Prerequisites:** Task 0.4
**Description:** Configure Belumi workspace to use Horizon crates via path dependencies.
**Acceptance criteria:**
- [ ] `horizon-core` accessible from Belumi workspace
- [ ] `horizon-client-render` accessible from Belumi workspace
- [ ] Build succeeds with Horizon dependencies
- [ ] No circular dependencies
- [ ] Documented in ARCHITECTURE.md

### Task 3.2: Shared Configuration Model

**Status:** Not started
**Prerequisites:** Task 3.1, Task 2.1
**Description:** Create shared configuration models used by both launcher and client.
**Acceptance criteria:**
- [ ] Shared config crate or module created
- [ ] Configuration serializable and versioned
- [ ] Used by both launcher and client
- [ ] Tests for serialization round-trip

---

## Phase 4: Plugin Platform Design (0.4.0)

### Task 4.1: Plugin API Design

**Status:** Not started
**Prerequisites:** Task 0.4, Task 3.1
**Description:** Design the plugin API with event system, lifecycle, and permissions.
**Acceptance criteria:**
- [ ] `docs/plugin-api.md` with API specification
- [ ] Event system design documented
- [ ] Permission model defined
- [ ] Lifecycle states documented
- [ ] ADR for plugin runtime (WASM vs Lua vs both)

### Task 4.2: Sandbox Evaluation

**Status:** Not started
**Prerequisites:** Task 4.1
**Description:** Evaluate sandboxing approaches (wasmtime, process isolation).
**Acceptance criteria:**
- [ ] Security analysis document
- [ ] Prototype with wasmtime
- [ ] Permission enforcement tested
- [ ] ADR for sandbox approach

---

## Phase 5: Rendering Extensions (0.3.0)

### Task 5.1: Rendering Pipeline Analysis

**Status:** Not started
**Prerequisites:** Task 3.1
**Description:** Analyze Horizon's rendering pipeline to identify extension points.
**Acceptance criteria:**
- [ ] `docs/rendering-analysis.md` with pipeline overview
- [ ] Extension points identified
- [ ] Shader integration approach documented

### Task 5.2: belumi-render Crate

**Status:** Not started
**Prerequisites:** Task 5.1
**Description:** Implement `belumi-render` with shader integration and post-processing.
**Acceptance criteria:**
- [ ] `belumi-render` crate created and compiles
- [ ] At least one rendering extension implemented
- [ ] Tests for rendering components
- [ ] Clippy clean, format check passing

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Horizon API changes break Belumi | High | Medium | Pin submodule commit, test on updates |
| Launcher tech choice wrong | High | Medium | Prototype before deciding |
| Plugin sandbox insufficient | High | Low | Security review, wasmtime isolation |
| Shaderpack compat too complex | Medium | High | Gradual approach, document limitations |
| Performance targets not met | Medium | Medium | Benchmark early, measure continuously |

## Open Questions

1. Should `belumi-core` eventually depend on Horizon crates?
2. What is the minimum supported Minecraft version beyond 26.3?
3. Should the launcher be a separate workspace like Horizon's?
4. How should plugin permissions be granularly defined?
5. What is the Easter egg infrastructure architecture?
- [ ] Measurable criteria (startup time, RAM) have concrete numbers
- [ ] Trade-offs clearly documented
- [ ] Recommendation with justification

### Task 1.2: Launcher Prototypes

**Status:** Not started
**Prerequisites:** Task 1.1
**Description:** Build small, throwaway prototypes for top 2-3 candidates.
**Acceptance criteria:**
- [ ] At least 2 prototype implementations
- [ ] Each prototype demonstrates main view, instance list, and settings
- [ ] Startup time and RAM usage measured
- [ ] Results documented in ADR-010
- [ ] Prototypes are in a separate directory (not main codebase)

### Task 1.3: Launcher Technology Decision

**Status:** Not started
**Prerequisites:** Task 1.2
**Description:** Finalize launcher technology choice based on prototype results.
**Acceptance criteria:**
- [ ] ADR-007 updated with final decision
- [ ] Selected technology justified with evidence
- [ ] `docs/decisions/ADR-010-launcher-tech-selection.md` marked Accepted