use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Evm,
    Solana,
    Bitcoin,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Evm => write!(f, "EVM"),
            Network::Solana => write!(f, "SOL"),
            Network::Bitcoin => write!(f, "BTC"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Rarity {
    Rare = 1,
    Epic = 2,
    Legendary = 3,
    Mythic = 4,
    Godlike = 5,
}

impl Rarity {
    pub fn badge(&self) -> &'static str {
        match self {
            Rarity::Godlike => "TIER-0 [EX]",
            Rarity::Mythic => "TIER-1 [S]",
            Rarity::Legendary => "TIER-2 [A]",
            Rarity::Epic => "TIER-3 [B]",
            Rarity::Rare => "TIER-4 [C]",
        }
    }

    pub fn folder_name(&self) -> &'static str {
        match self {
            Rarity::Godlike => "tier_0_ex",
            Rarity::Mythic => "tier_1_s",
            Rarity::Legendary => "tier_2_a",
            Rarity::Epic => "tier_3_b",
            Rarity::Rare => "tier_4_c",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Repdigits,
    Ladders,
    Alternating,
    Poker,
    Mirrors,
    Street,
    LuxuryStatus,
    CryptoCult,
    HackerHex,
    CustomTarget,
}

impl Theme {
    pub fn badge(&self) -> &'static str {
        match self {
            Theme::Repdigits => "REPDIGIT",
            Theme::Ladders => "LADDER",
            Theme::Alternating => "BINARY",
            Theme::Poker => "COMBINATION",
            Theme::Mirrors => "BOOKEND",
            Theme::Street => "DICTIONARY",
            Theme::LuxuryStatus => "STATUS",
            Theme::CryptoCult => "PROTOCOL",
            Theme::HackerHex => "HEX_CONST",
            Theme::CustomTarget => "TARGET",
        }
    }

    pub fn folder_name(&self) -> &'static str {
        match self {
            Theme::Repdigits => "repdigits",
            Theme::Ladders => "ladders",
            Theme::Alternating => "binary_matrix",
            Theme::Poker => "combos",
            Theme::Mirrors => "bookends",
            Theme::Street => "dictionary",
            Theme::LuxuryStatus => "status",
            Theme::CryptoCult => "protocol",
            Theme::HackerHex => "hex_constants",
            Theme::CustomTarget => "targets",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedWallet {
    pub network: Network,
    pub address: String,
    pub private_key: String,
    pub rarity: Rarity,
    pub theme: Theme,
    pub score: u32,
    pub title: String,
    pub pattern: String,
    pub timestamp: String,
}
