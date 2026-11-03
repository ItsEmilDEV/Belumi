# Contributing to Belumi

Thank you for your interest in contributing to Belumi!

## Getting Started

1. Fork the repository
2. Clone with submodules: `git clone --recurse-submodules <your-fork-url>`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run clippy: `cargo clippy -- -D warnings`
7. Check formatting: `cargo fmt --check`
8. Submit a pull request

## Development Guidelines

### Code Quality

- Write idiomatic Rust
- Keep modules small and focused
- Use meaningful names
- Handle errors properly (no unnecessary `unwrap()`)
- No unused dependencies
- No artificial abstraction layers
- Prefer composition over inheritance

### Documentation

- Document architecture decisions and public APIs
- Write all project documentation in English
- Use professional, objective language
- Explain complex implementation details

### Testing

- Write tests for new functionality
- Ensure all tests pass before submitting
- Do not declare untested features as complete

### License

- Belumi is MIT licensed
- Contributions must be compatible with MIT license
- Do not include Mojang assets or decompiled Minecraft source
- Preserve third-party notices and attributions

## Horizon Upstream

Belumi builds on Horizon. When contributing:

- Do not modify files in `vendor/horizon/` directly
- Report Horizon bugs to the [Horizon repository](https://github.com/horizon-rust/horizon)
- Follow Horizon's coding conventions where applicable

## Code of Conduct

Be respectful and constructive. We follow the
[Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).