# ADR-008: Standalone Launcher (Not Horizon Fork)

**Status:** Accepted
**Date:** 2026-10-30

## Context

Belumi could either fork Horizon's launcher or build its own.

## Decision

Belumi will have its own standalone launcher, not a fork of Horizon's.

## Rationale

- Own branding and design system
- Independent development and testing
- No dependency on Horizon's launcher internals
- Full control over UX and features

## Consequences

- Must implement all launcher functionality from scratch
- Need to define clear interfaces between launcher and client
- More development effort but greater long-term flexibility