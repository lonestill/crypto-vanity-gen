#!/usr/bin/env bash

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
cd "$DIR"

echo -e "\033[1;36m"
echo "vanity-gen :: high performance derivation engine"
echo "------------------------------------------------"
echo -e "\033[0m"

echo "Select operation mode:"
echo "  1) Target search (e.g. 0x7777777, Aye777, SATOSHI)"
echo "  2) Omni search (all chains: EVM, Solana, Bitcoin)"
echo "  3) Terminal explorer (TUI vault)"
echo "  4) Apple Metal GPU Turbo (EVM Personal Wallet with Private Key)"
echo "  5) Apple Metal GPU Turbo (CREATE2 Contract Vanity)"
echo "  6) Web report (viewer.html)"
echo "  7) High-difficulty only (Tier-2+)"
echo "  8) EVM only (Ethereum / Base / Polygon / BSC)"
echo "  9) Solana only (Ed25519 / Base58)"
echo " 10) Bitcoin only (Native SegWit bech32)"
echo ""
read -p "Option [1-10, default 3]: " choice

choice=${choice:-3}

case "$choice" in
  1)
    read -p "Target pattern [0x7777777]: " tgt
    tgt=${tgt:-0x7777777}
    ./target/release/crypto-vanity-gen --target "$tgt"
    ;;
  2)
    ./target/release/crypto-vanity-gen --network all --min-rarity epic
    ;;
  3)
    ./target/release/crypto-vanity-gen --view
    ;;
  4)
    read -p "Target prefix or zeros [hex 0-9, a-f, leave blank for >= 6 zeros]: " tgt
    if [ -n "$tgt" ]; then
      ./target/release/crypto-vanity-gen --gpu --target "$tgt"
    else
      ./target/release/crypto-vanity-gen --gpu --min-rarity legendary
    fi
    ;;
  5)
    read -p "Target prefix or zeros [hex 0-9, a-f, leave blank for >= 6 zeros]: " tgt
    if [ -n "$tgt" ]; then
      ./target/release/crypto-vanity-gen --create2 --target "$tgt"
    else
      ./target/release/crypto-vanity-gen --create2 --min-rarity legendary
    fi
    ;;
  6)
    open viewer.html || xdg-open viewer.html
    ;;
  7)
    ./target/release/crypto-vanity-gen --network all --min-rarity legendary
    ;;
  8)
    ./target/release/crypto-vanity-gen --network evm --min-rarity epic
    ;;
  9)
    ./target/release/crypto-vanity-gen --network sol --min-rarity epic
    ;;
  10)
    ./target/release/crypto-vanity-gen --network btc --min-rarity epic
    ;;
  *)
    ./target/release/crypto-vanity-gen --view
    ;;
esac
