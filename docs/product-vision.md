# Belumi Product Vision

## Overview

Belumi is a standalone, modular Minecraft ecosystem. It does not simply adopt
the surface or launcher of an existing project but develops its own identity,
technical foundations, and extensibility platform.

Inspiration: Linux. Beginners should get started immediately. Advanced users should
receive deep control. Developers should be able to add features without forking
the entire client.

The product consists of four logically separated areas:

1. **Belumi Launcher:** Installation, authentication, updates, profiles, and instances.
2. **Belumi Client:** Minecraft communication, game state, rendering, and client features.
3. **Belumi Platform:** Plugins, configuration, extension interfaces, and developer tools.
4. **Belumi Community:** Themes, extensions, resources, public Easter eggs, and community contributions.

These areas should work together but remain independent enough to be developed and
tested separately.

## Core Principles

1. **Performance first:** Efficient architecture with measurable performance targets.
2. **Customization without limits:** Simple settings for everyone, deep control for power users.
3. **Modular by design:** Features can be developed, activated, and extended independently.
4. **Open for creators:** Documented plugin API and tools for the community.
5. **Personality matters:** Unique identity through hidden insiders and community secrets.

These principles are equally important. Customizability must not lead to unstable
architecture, and performance must not serve as an excuse to restrict extensions.

## Two Levels of Customization

### Level A — Accessible Customization

The normal UI makes all important settings accessible without programming knowledge:

- Themes, colors, fonts, and UI scaling
- Customizable HUD with movable widgets
- Custom keybinds and configurable actions
- Profiles for different game modes and situations
- Import and export of settings
- Enabling and disabling individual modules

## Plugin System and Security Model

Belumi will receive an official, documented plugin API. Developers can add features
without modifying the entire client or maintaining a core fork.

### Plugin Fundamentals

- Versioned API with documented interfaces
- Event system and clearly defined extension points
- Module lifecycle with activation, deactivation, and error handling
- Dependency management and compatibility information
- Structured logs and traceable error messages
- Ability to update plugins independently of the core

### Sandbox and Expert Mode

Plugins receive only the permissions they actually need by default. Depending on
technical implementation, isolated runtime environments, explicit permissions,
and controlled interfaces may be used.

An optional expert mode gives advanced users access to additional APIs or
experimental interfaces.

Key rules:

- Expert mode is consciously activatable and transparent.
- Unsafe permissions are not granted silently.
- A sandbox is only called a security boundary if technically enforced.
- Plugins with extended permissions must be recognizable as such.
- Faulty plugins should not crash the client.

The concrete plugin runtime and technical sandbox implementation will be
selected based on the Rust architecture. They are not yet made implementation
decisions.

## Rendering and Shaderpacks

Belumi will develop a powerful native rendering pipeline supporting modern graphics
features and extensive customization.

### Planned Development

1. Stabilize rendering foundations and resource management.
2. Develop native graphics options and extensible shader architecture.
3. Define rendering passes, shader parameters, and compatibility interfaces.
4. Investigate and implement experimental support for selected Iris/OptiFine shaderpacks.
5. Test compatibility with concrete shaderpacks and extend step by step.

**Important:** Iris is a Java-based project and not simply a library that can be
embedded into a Rust renderer. Belumi needs its own compatibility layer or another
clearly defined technical solution. Full compatibility is a long-term goal, not a
guarantee.

Implementation must respect license conditions. Foreign code may not be adopted
if its license does not allow it.

## Easter Eggs and Community Secrets

Belumi will develop its own personality alongside its technical identity.

### Private Insiders

Developers and friends can build small secrets that are initially meaningless
to outsiders:

- Hidden texts and special animations
- Secret key combinations
- Personal references and insider jokes
- A hidden credits menu
- Optional secret UI states

### Public Community Secrets

Additionally, Easter eggs that the entire Belumi community can discover:

- Hidden clues in the interface
- Unlockable cosmetic UI effects
- Secret animations or special splash screens
- Community puzzles and seasonal surprises
- Special credits for contributors

Easter eggs should be optional, resource-efficient, and cleanly separated from
the core system. They must not collect sensitive data, impair normal functions,
or cause unexpected changes in game behavior.

The Easter egg infrastructure should be prepared early; concrete secrets can be
added independently later.

## Definition of Success

Belumi is successful when it functions not just as a Minecraft client but
as a standalone platform:

- Beginners can use it without complicated setup.
- Power users can make extensive customizations.
- Developers can create extensions via stable interfaces.
- Rendering technology can be developed independently.
- Performance and stability are measured, not just promised.
- The community can co-shape Belumi.
- Unique identity is maintained through insiders, Easter eggs, and a recognizable
  design philosophy.

**Central vision:**

Belumi should not just offer another Minecraft client. It should give
users the freedom to shape their own client experience space — from a simple,
performant installation to a deeply modified platform.

*Easy to use. Powerful to customize. Open to create.*
- Clear menus with search and command palette

### Level B — Power-User Control

Advanced users go far beyond normal settings:

- Readable, versioned configuration files
- Extended module configuration and custom presets
- Custom UI components and layouts
- Scripting and automatable workflows
- Extensible events and client hooks
- Custom plugins and experimental features
- Developer tools for diagnosis, debugging, and performance analysis

Both levels use the same underlying systems. Expert mode unlocks additional
capabilities rather than creating a second client.