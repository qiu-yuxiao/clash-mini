# CONTRIBUTING

Thank you for your interest in contributing to **Clash Mini**! This guide provides instructions to help you set up your development environment and start contributing effectively.



## Development Setup

Before contributing, you need to set up your development environment. Follow the steps below carefully.

### Prerequisites

1. **Install Rust and Node.js**  
   Our project requires both Rust and Node.js. Follow the official installation instructions [here](https://tauri.app/start/prerequisites/).

### Windows x64 Setup

- Ensure Rust and Node.js are added to your system `PATH`.

- Install the GNU `patch` tool.

- Use the MSVC toolchain for Rust:

```bash
rustup target add x86_64-pc-windows-msvc
rustup set default-host x86_64-pc-windows-msvc
```

### Install Node.js Package Manager

Enable `corepack`:

```bash
corepack enable
```

### Install Project Dependencies

Node.js dependencies:

```bash
pnpm install
```

### Download the Mihomo Core Binary (Automatic)

```bash
pnpm run prebuild
pnpm run prebuild --force  # Re-download and overwrite Mihomo core and service binaries
```

### Run the Development Server

```bash
pnpm dev           # Standard
pnpm dev:diff      # If an app instance already exists
pnpm dev:tauri     # Run Tauri development mode
```

### Build the Project

Standard build:

```bash
pnpm build
```

Fast build for testing:

```bash
pnpm build:fast
```

### Portable Version (Windows Only)

```bash
pnpm portable
```

## Contributing Your Changes

### Before Committing

**Code quality checks:**

```bash
# Rust backend
cargo clippy --all-targets --all-features -- -D warnings
# Frontend
pnpm lint
```

**Code formatting:**

```bash
# Rust backend
cargo fmt
# Frontend
pnpm format
```

### Signing your commit

Signed commits are required to verify authorship and ensure your contributions can be merged. Reference signing-commits [here](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits).

### Submitting Your Changes

1. Fork the repository.

2. Create a new branch for your feature or bug fix.

3. Commit your changes with clear messages and make sure it's signed.

4. Push your branch and submit a pull request.

We appreciate your contributions and look forward to your participation!

## Copyright and Licensing

Copyright (c) 2026 Qiu Yuxiao <qiuyuxiao@gmail.com> (Portions relating to modifications and custom enhancements).
Copyright (c) the original authors of Clash Verge.

This project is licensed under the GNU General Public License v3.0 (GPL-3.0-only). By contributing to **Clash Mini**, you agree that your contributions will be licensed under the project's GPL-3.0-only license.
