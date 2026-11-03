# ADR-009: Four Logical Areas

**Status:** Accepted
**Date:** 2026-10-30

## Context

Belumi is a complex ecosystem with multiple components.

## Decision

The product is divided into four logical areas:

1. **Belumi Launcher:** Installation, authentication, updates, profiles, instances
2. **Belumi Client:** Minecraft communication, game state, rendering, client features
3. **Belumi Platform:** Plugins, configuration, extension interfaces, developer tools
4. **Belumi Community:** Themes, extensions, resources, Easter eggs, community contributions

## Rationale

- Clear separation of concerns
- Independent development and testing
- Well-defined interfaces between areas
- Scalable architecture

## Consequences

- Need to define and maintain interfaces between areas
- Some shared functionality may require common libraries
- Shared libraries should only exist when they represent genuine shared functionality