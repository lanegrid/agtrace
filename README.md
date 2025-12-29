<div align="center">
  <img src="https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/images/agtrace-icon.png" width="96" alt="agtrace logo">
  <h1>agtrace</h1>
  <p><strong>top + tail -f for AI Coding Agent Sessions.</strong></p>

  [![npm version](https://img.shields.io/npm/v/@lanegrid/agtrace.svg?style=flat)](https://www.npmjs.com/package/@lanegrid/agtrace)
  [![crates.io](https://img.shields.io/crates/v/agtrace.svg)](https://crates.io/crates/agtrace)
</div>

---

![agtrace watch demo](https://raw.githubusercontent.com/lanegrid/agtrace/main/docs/assets/demo.gif)

## What it does

- Live dashboard for context pressure, tool activity, and costs
- Session history you can query and compare
- Works with Claude Code, Codex, and Gemini
- 100% local, no cloud

## Install

```bash
npm install -g @lanegrid/agtrace
```

## Usage

```bash
agtrace init      # once
agtrace watch     # in project dir, then start your agent
```

## Learn More

- [Why agtrace?](docs/motivation.md) - Understanding the problem and solution
- [Getting Started](docs/getting-started.md) - Detailed installation and usage guide
- [Full Documentation](docs/README.md) - Commands, architecture, and FAQs

## Supported Providers

- **Claude Code** (Anthropic)
- **Codex** (OpenAI)
- **Gemini** (Google)

## License

Dual-licensed under the MIT and Apache 2.0 licenses.
