# otterup

Official installer for OtterEVM - a blockchain for payments at scale.

## Quick Install

```bash
curl -L https://otterevm.xyz/install | bash
```

## Usage

```bash
otterup                  # Install latest release
otterup -i v1.0.0        # Install specific version
otterup -v               # Print installer version
otterup --update         # Update otterup itself
otterup --help           # Show help
```

## Supported Platforms

- **Linux**: x86_64, arm64
- **macOS**: Apple Silicon (arm64)
- **Windows**: x86_64, arm64

## Installation Directory

Default: `~/.otter/bin/`

Customize with `OTTER_DIR` environment variable:
```bash
OTTER_DIR=/custom/path otterup
```

## Updating

### Update OtterEVM Binary

Simply run otterup again:

```bash
otterup
```

### Update Otterup Itself

Use the built-in update command:

```bash
otterup --update
```

This will:
1. Check the latest version available on GitHub
2. Download and replace the otterup script if a newer version exists
3. Notify you of the version change

**Note:** Otterup automatically checks for updates when you run it and will warn you if your version is outdated.

## Uninstalling

```bash
rm -rf ~/.otter
```

Then remove the PATH export from your shell configuration file (`~/.zshenv`, `~/.bashrc`, `~/.config/fish/config.fish`, etc.).