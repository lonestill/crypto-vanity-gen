# crypto-vanity-gen

High-performance multi-chain vanity wallet generator written in Rust.
Supports EVM (Ethereum / Base / BSC / Polygon), Solana (Ed25519), and Bitcoin (Native SegWit bech32).

## Features

- Multi-core multithreaded architecture (Rayon + native thread workers)
- Cryptographic engines: secp256k1 (EVM), Ed25519 (Solana), Bech32 v0 P2WPKH (Bitcoin)
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

### 2. Target Address Search
Search for a specific prefix or exact sequence across compatible chains:
```bash
./target/release/crypto-vanity-gen --target 0x7777777
./target/release/crypto-vanity-gen --target Aye777
```

### 3. Omni Search (All Networks)
Continuously scan for mathematically and structurally rare addresses:
```bash
./target/release/crypto-vanity-gen --network all
```

### 4. Single Network Scan
```bash
# EVM only
./target/release/crypto-vanity-gen --network evm

# Solana only
./target/release/crypto-vanity-gen --network sol

# Bitcoin only
./target/release/crypto-vanity-gen --network btc
```

### 5. Terminal Explorer (TUI)
Inspect generated wallets, view private keys, filter by category, and copy addresses:
```bash
./target/release/crypto-vanity-gen --view
```

Keybindings in TUI:
- `↑` / `↓` / `j` / `k`: Navigate records
- `Tab`: Switch category tabs
- `c` / `Enter`: Copy address to clipboard
- `p`: Copy private key to clipboard
- `Space`: Toggle private key mask
- `q` / `Esc`: Exit

## Output Structure

All discovered addresses and keys are saved locally in the `output/` directory (ignored by git):
- `output/wallets_all.json`: Complete indexed vault
- `output/patterns/<category>/`: Organized by pattern classification
- `output/rarity/<tier>/`: Organized by difficulty tier

## License

MIT
