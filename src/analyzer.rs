use crate::types::{Network, Rarity, Theme};

pub struct BeautyMatch {
    pub rarity: Rarity,
    pub theme: Theme,
    pub score: u32,
    pub title: String,
    pub pattern: String,
}

pub struct Analyzer;

impl Analyzer {
    pub fn evaluate(network: Network, address: &str) -> Option<BeautyMatch> {
        let clean = match network {
            Network::Evm => address.trim_start_matches("0x"),
            Network::Bitcoin => address.trim_start_matches("bc1q"),
            Network::Solana => address,
        };

        if clean.len() < 8 {
            return None;
        }

        let lower = clean.to_ascii_lowercase();

        if let Some(m) = Self::check_cult_bookends(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_leading_zeros(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_monolith_repdigits(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_pure_ladders(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_binary_pulse(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_street_elite(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_status_elite(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_crypto_elite(clean, &lower) {
            return Some(m);
        }

        if let Some(m) = Self::check_hex_elite(clean, &lower) {
            return Some(m);
        }

        None
    }

    fn check_cult_bookends(clean: &str, lower: &str) -> Option<BeautyMatch> {
        let n = lower.len();
        if n < 10 {
            return None;
        }

        if lower.starts_with("7777") && lower.ends_with("7777") {
            let start: String = clean.chars().take(4).collect();
            let end: String = clean.chars().skip(n - 4).take(4).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::Mirrors,
                score: 999,
                title: format!("Symmetric quad-7 frame: {}...{}", start, end),
                pattern: format!("{}...{}", start, end),
            });
        }

        if lower.starts_with("777") && lower.ends_with("777") {
            let start: String = clean.chars().take(3).collect();
            let end: String = clean.chars().skip(n - 3).take(3).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Mythic,
                theme: Theme::Mirrors,
                score: 950,
                title: format!("Symmetric triple-7 frame: {}...{}", start, end),
                pattern: format!("{}...{}", start, end),
            });
        }

        if lower.starts_with("0000") && lower.ends_with("0000") {
            let start: String = clean.chars().take(4).collect();
            let end: String = clean.chars().skip(n - 4).take(4).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::Mirrors,
                score: 999,
                title: format!("Symmetric quad-zero frame: {}...{}", start, end),
                pattern: format!("{}...{}", start, end),
            });
        }
        if lower.starts_with("000") && lower.ends_with("000") {
            let start: String = clean.chars().take(3).collect();
            let end: String = clean.chars().skip(n - 3).take(3).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Mythic,
                theme: Theme::Mirrors,
                score: 940,
                title: format!("Symmetric triple-zero frame: {}...{}", start, end),
                pattern: format!("{}...{}", start, end),
            });
        }

        if lower.starts_with("888") && lower.ends_with("888") {
            let start: String = clean.chars().take(3).collect();
            let end: String = clean.chars().skip(n - 3).take(3).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Mythic,
                theme: Theme::Mirrors,
                score: 940,
                title: format!("Symmetric triple-8 frame: {}...{}", start, end),
                pattern: format!("{}...{}", start, end),
            });
        }

        if lower.starts_with("666") && lower.ends_with("666") {
            let start: String = clean.chars().take(3).collect();
            let end: String = clean.chars().skip(n - 3).take(3).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Mythic,
                theme: Theme::Mirrors,
                score: 930,
                title: format!("Symmetric triple-6 frame: {}...{}", start, end),
                pattern: format!("{}...{}", start, end),
            });
        }

        if lower.starts_with("dead") && lower.ends_with("beef") {
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::HackerHex,
                score: 999,
                title: "Hex split constant: DEAD...BEEF".to_string(),
                pattern: "dead...beef".to_string(),
            });
        }
        if lower.starts_with("cafe") && lower.ends_with("babe") {
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::HackerHex,
                score: 999,
                title: "Hex split constant: CAFE...BABE".to_string(),
                pattern: "cafe...babe".to_string(),
            });
        }
        if lower.starts_with("face") && lower.ends_with("dead") {
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::HackerHex,
                score: 990,
                title: "Hex split constant: FACE...DEAD".to_string(),
                pattern: "face...dead".to_string(),
            });
        }

        if (lower.starts_with("aye") && lower.ends_with("777")) ||
           (lower.starts_with("aye") && lower.ends_with("aye")) {
            let start: String = clean.chars().take(3).collect();
            let end: String = clean.chars().skip(n - 3).take(3).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::Street,
                score: 995,
                title: format!("Word frame: {}...{}", start.to_uppercase(), end.to_uppercase()),
                pattern: format!("{}...{}", start, end),
            });
        }
        if lower.starts_with("boss") && lower.ends_with("boss") {
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::LuxuryStatus,
                score: 990,
                title: "Word frame: BOSS...BOSS".to_string(),
                pattern: "boss...boss".to_string(),
            });
        }

        None
    }

    fn check_leading_zeros(clean: &str, lower: &str) -> Option<BeautyMatch> {
        let mut zero_count = 0;
        for c in lower.chars() {
            if c == '0' {
                zero_count += 1;
            } else {
                break;
            }
        }

        if zero_count >= 8 {
            let pat: String = clean.chars().take(zero_count).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::HackerHex,
                score: 999,
                title: format!("Zero prefix: {} consecutive zeros", zero_count),
                pattern: pat,
            });
        }
        if zero_count == 7 {
            let pat: String = clean.chars().take(7).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::HackerHex,
                score: 980,
                title: "Zero prefix: 7 consecutive zeros".to_string(),
                pattern: pat,
            });
        }
        if zero_count == 6 {
            let pat: String = clean.chars().take(6).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Mythic,
                theme: Theme::HackerHex,
                score: 940,
                title: "Zero prefix: 6 consecutive zeros".to_string(),
                pattern: pat,
            });
        }
        if zero_count == 5 {
            let pat: String = clean.chars().take(5).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Epic,
                theme: Theme::HackerHex,
                score: 800,
                title: "Zero prefix: 5 consecutive zeros".to_string(),
                pattern: pat,
            });
        }

        None
    }

    fn check_monolith_repdigits(clean: &str, lower: &str) -> Option<BeautyMatch> {
        let first_char = lower.chars().next()?;
        if first_char == '0' {
            return None;
        }

        let mut count = 0;
        for c in lower.chars() {
            if c == first_char {
                count += 1;
            } else {
                break;
            }
        }

        if count >= 7 {
            let pat: String = clean.chars().take(count).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Godlike,
                theme: Theme::Repdigits,
                score: 999,
                title: format!("Monolith repdigit: {}x '{}'", count, first_char),
                pattern: pat,
            });
        }
        if count == 6 {
            let pat: String = clean.chars().take(6).collect();
            return Some(BeautyMatch {
                rarity: Rarity::Mythic,
                theme: Theme::Repdigits,
                score: 930,
                title: format!("Monolith repdigit: 6x '{}'", first_char),
                pattern: pat,
            });
        }

        None
    }

    fn check_pure_ladders(clean: &str, lower: &str) -> Option<BeautyMatch> {
        const TIER0_LADDERS: &[&str] = &[
            "1234567", "7654321", "9876543", "0123456"
        ];
        const TIER1_LADDERS: &[&str] = &[
            "123456", "654321", "987654", "abcdef", "fedcba", "012345", "543210"
        ];

        for &lad in TIER0_LADDERS {
            if lower.starts_with(lad) {
                let matched: String = clean.chars().take(lad.len()).collect();
                return Some(BeautyMatch {
                    rarity: Rarity::Godlike,
                    theme: Theme::Ladders,
                    score: 990,
                    title: format!("Sequential ladder 7 chars: '{}'", matched),
                    pattern: matched,
                });
            }
        }

        for &lad in TIER1_LADDERS {
            if lower.starts_with(lad) {
                let matched: String = clean.chars().take(lad.len()).collect();
                return Some(BeautyMatch {
                    rarity: Rarity::Mythic,
                    theme: Theme::Ladders,
                    score: 920,
                    title: format!("Sequential ladder 6 chars: '{}'", matched),
                    pattern: matched,
                });
            }
        }

        None
    }

    fn check_binary_pulse(clean: &str, lower: &str) -> Option<BeautyMatch> {
        const PULSES: &[&str] = &[
            "01010101", "10101010", "69696969", "00110011", "11001100"
        ];

        for &p in PULSES {
            if lower.starts_with(p) {
                let matched: String = clean.chars().take(p.len()).collect();
                return Some(BeautyMatch {
                    rarity: Rarity::Mythic,
                    theme: Theme::Alternating,
                    score: 910,
                    title: format!("Binary pulse pattern: '{}'", matched),
                    pattern: matched,
                });
            }
        }

        None
    }

    fn check_street_elite(clean: &str, lower: &str) -> Option<BeautyMatch> {
        const ELITE_STREET: &[(&str, Rarity, u32)] = &[
            ("aye777aye", Rarity::Godlike, 999),
            ("avtoritet", Rarity::Godlike, 999),
            ("bratva777", Rarity::Godlike, 990),
            ("obshak777", Rarity::Godlike, 980),
            ("obwak777",  Rarity::Godlike, 980),
            ("pahan777",  Rarity::Godlike, 980),
            ("stvol777",  Rarity::Godlike, 980),
            ("smotr777",  Rarity::Godlike, 980),
            ("blat777",   Rarity::Mythic,  940),
            ("fart777",   Rarity::Mythic,  940),
            ("kush777",   Rarity::Mythic,  930),
            ("acab777",   Rarity::Mythic,  930),
            ("aye777",    Rarity::Mythic,  930),
            ("bratva",    Rarity::Mythic,  930),
            ("obshak",    Rarity::Mythic,  930),
            ("vor777",    Rarity::Mythic,  920),
            ("obwak",     Rarity::Legendary, 880),
            ("pahan",     Rarity::Legendary, 880),
        ];

        for &(w, rarity, score) in ELITE_STREET {
            if lower.starts_with(w) {
                let matched: String = clean.chars().take(w.len()).collect();
                return Some(BeautyMatch {
                    rarity,
                    theme: Theme::Street,
                    score,
                    title: format!("Dictionary: '{}'", matched.to_uppercase()),
                    pattern: matched,
                });
            }
        }

        None
    }

    fn check_status_elite(clean: &str, lower: &str) -> Option<BeautyMatch> {
        const ELITE_STATUS: &[(&str, Rarity, u32)] = &[
            ("gigachad", Rarity::Godlike, 999),
            ("jackpot",  Rarity::Godlike, 980),
            ("billion",  Rarity::Godlike, 980),
            ("million",  Rarity::Godlike, 970),
            ("fortune",  Rarity::Godlike, 970),
            ("emperor",  Rarity::Godlike, 970),
            ("boss777",  Rarity::Godlike, 960),
            ("king777",  Rarity::Godlike, 960),
            ("rich777",  Rarity::Godlike, 950),
            ("gold777",  Rarity::Godlike, 950),
            ("1337b055", Rarity::Godlike, 970),
            ("badb055",  Rarity::Mythic,  940),
            ("b055777",  Rarity::Mythic,  930),
            ("b055000",  Rarity::Mythic,  930),
            ("vip777",   Rarity::Mythic,  930),
        ];

        for &(w, rarity, score) in ELITE_STATUS {
            if lower.starts_with(w) {
                let matched: String = clean.chars().take(w.len()).collect();
                return Some(BeautyMatch {
                    rarity,
                    theme: Theme::LuxuryStatus,
                    score,
                    title: format!("Status token: '{}'", matched.to_uppercase()),
                    pattern: matched,
                });
            }
        }

        None
    }

    fn check_crypto_elite(clean: &str, lower: &str) -> Option<BeautyMatch> {
        const ELITE_CRYPTO: &[(&str, Rarity, u32)] = &[
            ("nakamoto", Rarity::Godlike, 990),
            ("ethereum", Rarity::Godlike, 990),
            ("satoshi",  Rarity::Godlike, 980),
            ("bitcoin",  Rarity::Godlike, 980),
            ("moon777",  Rarity::Mythic,  940),
            ("pump777",  Rarity::Mythic,  940),
            ("hodl777",  Rarity::Mythic,  940),
            ("solana",   Rarity::Mythic,  930),
        ];

        for &(w, rarity, score) in ELITE_CRYPTO {
            if lower.starts_with(w) {
                let matched: String = clean.chars().take(w.len()).collect();
                return Some(BeautyMatch {
                    rarity,
                    theme: Theme::CryptoCult,
                    score,
                    title: format!("Protocol token: '{}'", matched.to_uppercase()),
                    pattern: matched,
                });
            }
        }

        None
    }

    fn check_hex_elite(clean: &str, lower: &str) -> Option<BeautyMatch> {
        const ELITE_HEX: &[(&str, Rarity, u32)] = &[
            ("deadbeef", Rarity::Godlike, 999),
            ("cafebabe", Rarity::Godlike, 999),
            ("deadc0de", Rarity::Godlike, 995),
            ("def11337", Rarity::Godlike, 990),
            ("1337c0de", Rarity::Godlike, 990),
            ("badbabe",  Rarity::Godlike, 980),
            ("c0ffee",   Rarity::Mythic,  940),
            ("bada55",   Rarity::Mythic,  930),
        ];

        for &(w, rarity, score) in ELITE_HEX {
            if lower.starts_with(w) {
                let matched: String = clean.chars().take(w.len()).collect();
                return Some(BeautyMatch {
                    rarity,
                    theme: Theme::HackerHex,
                    score,
                    title: format!("Hex magic constant: '{}'", matched.to_uppercase()),
                    pattern: matched,
                });
            }
        }

        None
    }
}
