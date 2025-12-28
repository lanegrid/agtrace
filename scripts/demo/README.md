# Demo GIF Generation

This directory contains the scripts to generate the demo.gif for the README.

## Prerequisites

- Rust and Cargo (for building agtrace)
- [VHS](https://github.com/charmbracelet/vhs) - Terminal recording tool

Install VHS:
```bash
brew install vhs
```

## Usage

From the project root, run:

```bash
./scripts/demo/generate.sh
```

This will:
1. Install agtrace from local source if not already installed
2. Generate demo.gif using VHS
3. Place the resulting demo.gif in the project root

## Files

- `demo.tape` - VHS tape file that defines the demo recording
- `generate.sh` - Shell script to automate demo.gif generation
- `README.md` - This file

## Customization

To modify the demo:
1. Edit `demo.tape` to change the recording (size, timing, commands)
2. Run `./scripts/demo/generate.sh` to regenerate demo.gif
