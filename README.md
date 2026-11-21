<div align="center">

<img width="100%" src="https://capsule-render.vercel.app/api?type=waving&height=180&color=gradient&text=RyNFuscator&fontAlign=50&fontAlignY=35&fontSize=42&desc=Lua%20Obfuscator%20%E2%80%A2%20AST%20Transformations%20%E2%80%A2%20Built%20in%20Rust&descAlign=50&descAlignY=60" />

<a href="https://github.com/larpcorrupted/RyNFuscator">
  <img src="https://readme-typing-svg.demolab.com?font=JetBrains+Mono&weight=600&size=24&duration=2500&pause=900&center=true&vCenter=true&width=760&lines=Pure+Rust+Lua+Obfuscation;AST+Transforms+%E2%80%A2+Encryption+%E2%80%A2+VM+Protection;Fast+Native+Execution;Built+for+Code+Protection" alt="Typing SVG" />
</a>

<br/>

<a href="https://github.com/larpcorrupted/RyNFuscator/releases">
  <img src="https://img.shields.io/badge/DOWNLOAD-LATEST-0F766E?style=for-the-badge&logo=rust&logoColor=white" alt="Download Release" />
</a>
<a href="https://github.com/larpcorrupted/RyNFuscator/actions/workflows/ci.yml">
  <img src="https://img.shields.io/github/actions/workflow/status/larpcorrupted/RyNFuscator/ci.yml?branch=main&style=for-the-badge&label=Build" alt="Build Status" />
</a>
<a href="https://github.com/larpcorrupted/RyNFuscator/stargazers">
  <img src="https://img.shields.io/github/stars/larpcorrupted/RyNFuscator?style=for-the-badge&logo=github&label=Stars" alt="GitHub stars" />
</a>
<a href="https://discord.gg/yourinvite">
  <img src="https://img.shields.io/badge/Discord-Join%20Server-5865F2?style=for-the-badge&logo=discord&logoColor=white" alt="Discord server" />
</a>

</div>

---

<p align="center">
  <img src="assets/readme/preview.gif" alt="RyNFuscator obfuscation process preview" width="900" />
</p>

**RyNFuscator** is a Lua obfuscator built in **Rust**.
This is RyNfuscator v1. Not the current used in luauth.

Btw it only does print(1) and basic lua not whole luau code :(

It applies **AST-based transformations** to make Lua source code harder to read, analyze, and reverse engineer—combining native Rust speed with passes like **constant encryption**, **control-flow flattening**, and **VM/bytecode protection**.


---

## Features

- **Written in Rust**: Fast compilation and low-overhead CLI execution.
- **AST Transformations**: Parses and transforms Lua syntax trees with modular obfuscation passes.
- **Constant & String Protection**: Encrypts or encodes literals and strings to hide runtime values.
- **Control Flow Obfuscation**: Complicates execution paths to slow down static analysis.
- **Configurable Pipeline**: Customize passes and presets depending on your speed vs. protection trade-off.

---

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (`cargo` and `rustc`)

### Build from Source

```bash
git clone [https://github.com/larpcorrupted/RyNFuscator.git](https://github.com/larpcorrupted/RyNFuscator.git)
cd RyNFuscator
cargo build --release
```

<div align="center">

<img width="100%" src="https://capsule-render.vercel.app/api?type=waving&height=120&section=footer&color=gradient" />

</div>

