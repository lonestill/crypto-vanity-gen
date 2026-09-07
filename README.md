# crypto-vanity-gen

High-performance cryptographic vanity address generator written in Rust with native Apple Silicon Metal GPU compute acceleration.
Supports EVM (Ethereum / Base / BSC / Polygon / Arbitrum), Solana (Ed25519), Bitcoin (Native SegWit bech32), and CREATE2 contract vanity deployment.

## Features

- Native Apple Silicon Metal GPU compute shaders (13+ MH/s on M1)
- Multi-core multithreaded architecture (Rayon + native thread workers)
- Cryptographic engines: secp256k1 (EVM), Ed25519 (Solana), Bech32 v0 P2WPKH (Bitcoin), Keccak-256 (CREATE2)
- Pattern matching: leading zeros, sequential ladders, monolith repdigits, symmetric bookends, binary matrix, dictionary tokens
- Built-in terminal explorer (Ratatui TUI) with clipboard integration (`pbcopy`)
- Fully local and offline execution

## Installation

Requires Rust toolchain (1.75+):

```bash
git clone https://github.com/lonestill/crypto-vanity-gen.git
cd crypto-vanity-gen
cargo build --release
```

## Quick Start

### 1. Interactive Menu
```bash
./run.sh
```

### 2. Apple Metal GPU Turbo Mode (13+ MH/s)
Harness Apple Silicon M1/M2/M3/M4 GPU compute cores for CREATE2 contract vanity derivation:
```bash
# Search for custom target prefix on GPU (takes seconds)
./target/release/crypto-vanity-gen --gpu --target 0x777777

# Search for leading zero contracts (e.g. 0x000000...)
./target/release/crypto-vanity-gen --gpu --min-rarity mythic

# Custom factory and init_code_hash
./target/release/crypto-vanity-gen --gpu --factory 0x... --init-code-hash 0x...
```

### 3. Target Address Search (CPU)
Search for a specific prefix or exact sequence across compatible chains:
```bash
./target/release/crypto-vanity-gen --target 0x7777777
./target/release/crypto-vanity-gen --target Aye777
```

### 4. Omni Search (All Networks)
Continuously scan for mathematically and structurally rare addresses:
```bash
./target/release/crypto-vanity-gen --network all
```

### 5. Single Network Scan
```bash
# EVM only
./target/release/crypto-vanity-gen --network evm

# Solana only
./target/release/crypto-vanity-gen --network sol

# Bitcoin only
./target/release/crypto-vanity-gen --network btc
```

### 6. Terminal Explorer (TUI)
Inspect generated wallets, view private keys, filter by category, and copy addresses:
```bash
./target/release/crypto-vanity-gen --view
```

Keybindings in TUI:
- `↑` / `↓` / `j` / `k`: Navigate records
- `Tab`: Switch category tabs
- `c` / `Enter`: Copy address to clipboard
- `p`: Copy private key to clipboard
- `d` / `Del`: Delete selected address
- `x`: Purge entire vault (prompts for confirmation)
- `Space`: Toggle private key mask
- `q` / `Esc`: Exit

## Output Structure

All discovered addresses and keys are saved locally in the `output/` directory (ignored by git):
- `output/wallets_all.json`: Complete indexed vault
- `output/patterns/<category>/`: Organized by pattern classification
- `output/rarity/<tier>/`: Organized by difficulty tier

## License

MIT
