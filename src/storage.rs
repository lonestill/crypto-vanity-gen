use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use crate::types::GeneratedWallet;

pub struct Storage {
    output_dir: String,
    lock: Mutex<()>,
}

impl Storage {
    pub fn new(output_dir: &str) -> Self {
        fs::create_dir_all(output_dir).unwrap_or(());

        for pattern in &[
            "repdigits",
            "ladders",
            "binary_matrix",
            "combos",
            "bookends",
            "dictionary",
            "status",
            "protocol",
            "hex_constants",
            "targets",
        ] {
            fs::create_dir_all(format!("{}/patterns/{}", output_dir, pattern)).unwrap_or(());
        }

        for rarity in &["tier_0_ex", "tier_1_s", "tier_2_a", "tier_3_b", "tier_4_c"] {
            fs::create_dir_all(format!("{}/rarity/{}", output_dir, rarity)).unwrap_or(());
        }

        Self {
            output_dir: output_dir.to_string(),
            lock: Mutex::new(()),
        }
    }

    pub fn save(&self, wallet: &GeneratedWallet) {
        let _guard = self.lock.lock().unwrap();

        let all_json_path = format!("{}/wallets_all.json", self.output_dir);
        let mut all_wallets: Vec<GeneratedWallet> = if Path::new(&all_json_path).exists() {
            let content = fs::read_to_string(&all_json_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        all_wallets.push(wallet.clone());
        if let Ok(serialized) = serde_json::to_string_pretty(&all_wallets) {
            let _ = fs::write(&all_json_path, serialized);
        }

        let pattern_folder = wallet.theme.folder_name();
        let pattern_json_path = format!("{}/patterns/{}/wallets.json", self.output_dir, pattern_folder);
        let mut pattern_wallets: Vec<GeneratedWallet> = if Path::new(&pattern_json_path).exists() {
            let content = fs::read_to_string(&pattern_json_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        pattern_wallets.push(wallet.clone());
        if let Ok(serialized) = serde_json::to_string_pretty(&pattern_wallets) {
            let _ = fs::write(&pattern_json_path, serialized);
        }

        let pattern_txt_path = format!("{}/patterns/{}/wallets.txt", self.output_dir, pattern_folder);
        self.append_card(&pattern_txt_path, wallet);

        let rarity_folder = wallet.rarity.folder_name();
        let rarity_json_path = format!("{}/rarity/{}/wallets.json", self.output_dir, rarity_folder);
        let mut rarity_wallets: Vec<GeneratedWallet> = if Path::new(&rarity_json_path).exists() {
            let content = fs::read_to_string(&rarity_json_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        rarity_wallets.push(wallet.clone());
        if let Ok(serialized) = serde_json::to_string_pretty(&rarity_wallets) {
            let _ = fs::write(&rarity_json_path, serialized);
        }

        let rarity_txt_path = format!("{}/rarity/{}/wallets.txt", self.output_dir, rarity_folder);
        self.append_card(&rarity_txt_path, wallet);
    }

    fn append_card(&self, file_path: &str, wallet: &GeneratedWallet) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(file_path) {
            let _ = writeln!(file, "--------------------------------------------------------------------------------");
            let _ = writeln!(file, "timestamp:    {}", wallet.timestamp);
            let _ = writeln!(file, "network:      {}", wallet.network);
            let _ = writeln!(file, "address:      {}", wallet.address);
            let _ = writeln!(file, "private_key:  {}", wallet.private_key);
            let _ = writeln!(file, "tier:         {} (score: {})", wallet.rarity.badge(), wallet.score);
            let _ = writeln!(file, "category:     {}", wallet.theme.badge());
            let _ = writeln!(file, "title:        {}", wallet.title);
            let _ = writeln!(file, "pattern:      {}", wallet.pattern);
            let _ = writeln!(file, "--------------------------------------------------------------------------------\n");
        }
    }
}
