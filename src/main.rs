mod types;
mod chains;
mod analyzer;
mod storage;
mod tui;
#[cfg(target_os = "macos")]
mod metal;

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use std::io::Write;
use rand::RngCore;
use secp256k1::{Secp256k1, SecretKey, PublicKey, Scalar};
use colored::*;
use clap::Parser;
use chrono::Local;

use types::{Network, Rarity, Theme, GeneratedWallet};
use chains::{EvmGenerator, SolanaGenerator, BitcoinGenerator};
use analyzer::Analyzer;
use storage::Storage;

#[derive(Parser, Debug)]
#[command(name = "crypto-vanity-gen", about = "Cryptographic vanity address derivation utility")]
struct Args {
    #[arg(short, long, default_value_t = false)]
    view: bool,

    #[arg(short, long, default_value = "all")]
    network: String,

    #[arg(short, long)]
    target: Option<String>,

    #[arg(long, default_value_t = false)]
    case_sensitive: bool,

    #[arg(short, long, default_value = "rare")]
    min_rarity: String,

    #[arg(short, long, default_value_t = 0)]
    count: u32,

    #[arg(short = 'T', long, default_value_t = 0)]
    threads: usize,

    #[arg(short, long)]
    prefix: Option<String>,

    #[arg(short, long)]
    suffix: Option<String>,

    #[arg(short, long, default_value = "output")]
    output_dir: String,

    #[arg(long, default_value_t = false)]
    gpu: bool,

    #[arg(long, default_value_t = false)]
    create2: bool,

    #[arg(long)]
    factory: Option<String>,

    #[arg(long)]
    init_code_hash: Option<String>,
}

fn parse_min_rarity(s: &str) -> Rarity {
    match s.to_ascii_lowercase().as_str() {
        "godlike" | "tier0" | "tier-0" | "ex" => Rarity::Godlike,
        "mythic" | "tier1" | "tier-1" | "s" => Rarity::Mythic,
        "legendary" | "tier2" | "tier-2" | "a" => Rarity::Legendary,
        "epic" | "tier3" | "tier-3" | "b" => Rarity::Epic,
        _ => Rarity::Rare,
    }
}

fn is_evm_compatible(target: &str) -> bool {
    !target.is_empty() && target.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_solana_compatible(target: &str, case_sensitive: bool) -> bool {
    const B58_CHARS: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if target.is_empty() {
        return false;
    }
    if case_sensitive {
        target.chars().all(|c| B58_CHARS.contains(c))
    } else {
        target.chars().all(|c| {
            let lower = c.to_ascii_lowercase();
            let upper = c.to_ascii_uppercase();
            B58_CHARS.contains(lower) || B58_CHARS.contains(upper)
        })
    }
}

fn is_bitcoin_compatible(target: &str) -> bool {
    const BECH32_CHARS: &str = "023456789acdefghjklmnpqrstuvwxyz";
    !target.is_empty() && target.to_ascii_lowercase().chars().all(|c| BECH32_CHARS.contains(c))
}

fn parse_hex_20(s: &str) -> Result<[u8; 20], String> {
    let clean = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if clean.len() != 40 {
        return Err(format!("factory address must be 20 bytes (40 hex characters), got {}", clean.len()));
    }
    let mut out = [0u8; 20];
    for i in 0..20 {
        out[i] = u8::from_str_radix(&clean[i*2..i*2+2], 16)
            .map_err(|e| format!("invalid hex at byte {}: {}", i, e))?;
    }
    Ok(out)
}

fn parse_hex_32(s: &str) -> Result<[u8; 32], String> {
    let clean = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    if clean.len() != 64 {
        return Err(format!("init code hash must be 32 bytes (64 hex characters), got {}", clean.len()));
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&clean[i*2..i*2+2], 16)
            .map_err(|e| format!("invalid hex at byte {}: {}", i, e))?;
    }
    Ok(out)
}

fn main() {
    let args = Args::parse();

    if args.view {
        if let Err(e) = tui::run_tui(&args.output_dir) {
            eprintln!("TUI error: {}", e);
        }
        return;
    }

    let is_create2_mode = args.create2 || args.network.to_ascii_lowercase() == "create2";

    if is_create2_mode {
        run_create2_engine(&args);
        return;
    }

    let is_eoa_gpu_mode = args.gpu;
    if is_eoa_gpu_mode {
        #[cfg(target_os = "macos")]
        {
            run_eoa_engine(&args);
            return;
        }
        #[cfg(not(target_os = "macos"))]
        {
            eprintln!("Metal GPU is only supported on macOS Apple Silicon. Falling back to CPU workers.");
        }
    }

    run_wallet_engine(&args);
}

fn run_create2_engine(args: &Args) {
    let factory_bytes = match &args.factory {
        Some(f) => match parse_hex_20(f) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: {}", e);
                return;
            }
        },
        None => {
            let default_factory = "0000000000ffe8b47b3e21302130b649599b7940";
            let mut out = [0u8; 20];
            for i in 0..20 {
                out[i] = u8::from_str_radix(&default_factory[i*2..i*2+2], 16).unwrap();
            }
            out
        }
    };

    let init_hash_bytes = match &args.init_code_hash {
        Some(h) => match parse_hex_32(h) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: {}", e);
                return;
            }
        },
        None => {
            let default_hash = "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";
            let mut out = [0u8; 32];
            for i in 0..32 {
                out[i] = u8::from_str_radix(&default_hash[i*2..i*2+2], 16).unwrap();
            }
            out
        }
    };

    let min_rarity = parse_min_rarity(&args.min_rarity);

    let raw_target = if let Some(ref t) = args.target {
        let t_trimmed = t.trim();
        if t_trimmed.starts_with("0x") || t_trimmed.starts_with("0X") {
            t_trimmed[2..].to_string()
        } else {
            t_trimmed.to_string()
        }
    } else {
        String::new()
    };

    let mut clean_target = String::new();
    let mut stripped_non_hex = false;
    for c in raw_target.chars() {
        if c.is_ascii_hexdigit() {
            clean_target.push(c.to_ascii_lowercase());
        } else {
            stripped_non_hex = true;
        }
    }

    if stripped_non_hex && !raw_target.is_empty() {
        eprintln!("[NOTE] Stripped non-hex chars for EVM target: '{}' -> '0x{}'", raw_target, clean_target);
    }

    let mut target_nibbles = Vec::new();
    if !clean_target.is_empty() {
        for c in clean_target.chars() {
            if let Some(d) = c.to_digit(16) {
                target_nibbles.push(d as u8);
            }
        }
    }

    let min_zeros = if !clean_target.is_empty() {
        3u32.min(target_nibbles.len() as u32)
    } else {
        match min_rarity {
            Rarity::Godlike => 8u32,
            Rarity::Mythic => 7u32,
            Rarity::Legendary => 6u32,
            Rarity::Epic => 6u32,
            Rarity::Rare => 6u32,
        }
    };

    #[cfg(target_os = "macos")]
    let metal_engine = if metal::MetalEngine::is_supported() {
        metal::MetalEngine::init()
    } else {
        None
    };

    #[cfg(not(target_os = "macos"))]
    let metal_engine: Option<bool> = None;

    let storage = Arc::new(Storage::new(&args.output_dir));
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();

    let _ = ctrlc::set_handler(move || {
        eprintln!("\ninterrupt signal received. finalizing writes...");
        r_clone.store(false, Ordering::SeqCst);
    });

    let total_salts = Arc::new(AtomicU64::new(0));
    let total_found = Arc::new(AtomicU32::new(0));
    let count_godlike = Arc::new(AtomicU32::new(0));
    let count_mythic = Arc::new(AtomicU32::new(0));
    let count_legendary = Arc::new(AtomicU32::new(0));
    let count_epic = Arc::new(AtomicU32::new(0));
    let current_speed_khz = Arc::new(AtomicU64::new(0));

    let factory_hex = format!("0x{}", chains::evm::hex::encode(&factory_bytes));
    let init_hash_hex = format!("0x{}", chains::evm::hex::encode(&init_hash_bytes));

    println!("\nvanity-gen v0.2.0 [darwin/aarch64]");
    #[cfg(target_os = "macos")]
    if let Some(ref engine) = metal_engine {
        println!("engine:    Metal GPU Compute [{}]", engine.name());
    } else {
        println!("engine:    CPU Multi-Core");
    }
    #[cfg(not(target_os = "macos"))]
    println!("engine:    CPU Multi-Core");

    println!("mode:      CREATE2 Smart Contract Vanity Generator");
    println!("factory:   {}", factory_hex);
    println!("init_hash: {}", init_hash_hex);

    if !clean_target.is_empty() {
        println!("target:    0x{}", clean_target);
        println!("milestone: progressive stages starting from 3 chars");
    } else {
        println!("filter:    >= {} leading zeros [{}]", min_zeros, min_rarity.badge());
    }

    if args.count > 0 {
        println!("limit:     {} records", args.count);
    } else {
        println!("limit:     continuous (Ctrl+C to stop)");
    }
    println!("vault:     {}", args.output_dir);
    println!("--------------------------------------------------------------------------------");

    let running_stats = running.clone();
    let total_s = total_salts.clone();
    let cur_spd = current_speed_khz.clone();
    let c_g = count_godlike.clone();
    let c_m = count_mythic.clone();
    let c_l = count_legendary.clone();
    let c_e = count_epic.clone();
    let has_target = !clean_target.is_empty();
    let target_display = clean_target.clone();

    let stats_thread = thread::spawn(move || {
        let start_time = Instant::now();

        while running_stats.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(250));
            let current = total_s.load(Ordering::Relaxed);
            let spd_khz = cur_spd.load(Ordering::Relaxed);
            let speed_mhz = spd_khz as f64 / 1000.0;

            let elapsed = start_time.elapsed().as_secs();
            let elapsed_str = format!("{:02}:{:02}", elapsed / 60, elapsed % 60);

            if has_target {
                let stats_line = format!(
                    "[{}] {:>6.2} MH/s | salts: {:>12} | target: 0x{} | found: {}",
                    elapsed_str,
                    speed_mhz,
                    format_number(current),
                    target_display,
                    c_g.load(Ordering::Relaxed) + c_m.load(Ordering::Relaxed) + c_l.load(Ordering::Relaxed)
                );
                eprint!("\r{}", stats_line);
            } else {
                let stats_line = format!(
                    "[{}] {:>6.2} MH/s | salts: {:>12} | t0: {} | t1: {} | t2: {} | t3: {}",
                    elapsed_str,
                    speed_mhz,
                    format_number(current),
                    c_g.load(Ordering::Relaxed),
                    c_m.load(Ordering::Relaxed),
                    c_l.load(Ordering::Relaxed),
                    c_e.load(Ordering::Relaxed),
                );
                eprint!("\r{}", stats_line);
            }
        }
        eprintln!();
    });

    #[cfg(target_os = "macos")]
    if let Some(engine) = metal_engine {
        let threads = 65536u32;
        let iters = 40u32;
        let batch_size = (threads as u64) * (iters as u64);
        let mut base_salt = rand::random::<u64>();
        let mut rolling_speed = 0.0f64;
        let has_target = !clean_target.is_empty();
        let target_len = clean_target.len();
        let mut current_target_stage = if has_target {
            3u32.min(target_len as u32)
        } else {
            min_zeros
        };

        while running.load(Ordering::Relaxed) {
            if args.count > 0 && total_found.load(Ordering::Relaxed) >= args.count {
                running.store(false, Ordering::Relaxed);
                break;
            }

            let t0 = Instant::now();
            let matches = engine.run_create2_batch(
                &factory_bytes,
                &init_hash_bytes,
                base_salt,
                threads,
                iters,
                current_target_stage,
                &target_nibbles,
            );
            let dt = t0.elapsed().as_secs_f64();
            if dt > 0.0001 {
                let inst = (batch_size as f64 / dt) / 1000.0;
                if rolling_speed == 0.0 {
                    rolling_speed = inst;
                } else {
                    rolling_speed = rolling_speed * 0.75 + inst * 0.25;
                }
                current_speed_khz.store(rolling_speed as u64, Ordering::Relaxed);
            }

            total_salts.fetch_add(batch_size, Ordering::Relaxed);
            base_salt = base_salt.wrapping_add(batch_size);

            for m in matches {
                let addr_str = EvmGenerator::format_address_checksum(&m.address);
                let clean_addr = addr_str.trim_start_matches("0x");

                let eval_match = if has_target {
                    let match_len = calc_prefix_match(clean_addr, &clean_target, false);
                    if match_len >= current_target_stage as usize {
                        if match_len < target_len {
                            current_target_stage = (match_len as u32 + 1).min(target_len as u32);
                        }
                        let rarity = if match_len >= target_len {
                            Rarity::Godlike
                        } else if match_len >= 6 {
                            Rarity::Mythic
                        } else {
                            Rarity::Legendary
                        };
                        let matched_slice = &clean_addr[..match_len];
                        Some(analyzer::BeautyMatch {
                            rarity,
                            theme: Theme::CustomTarget,
                            score: (match_len as u32 * 110).min(1000),
                            title: format!("CREATE2 Target milestone [{}/{}]: 0x{}", match_len, target_len, matched_slice),
                            pattern: format!("Target: 0x{} (matched: 0x{})", clean_target, matched_slice),
                        })
                    } else {
                        None
                    }
                } else {
                    Analyzer::evaluate(Network::Evm, &addr_str)
                };

                let beauty = match eval_match {
                    Some(b) => b,
                    None => continue,
                };

                if beauty.rarity < min_rarity || beauty.rarity < Rarity::Legendary {
                    continue;
                }

                total_found.fetch_add(1, Ordering::Relaxed);
                match beauty.rarity {
                    Rarity::Godlike => { count_godlike.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Mythic => { count_mythic.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Legendary => { count_legendary.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Epic => { count_epic.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Rare => {}
                }

                let mut salt_bytes = [0u8; 32];
                salt_bytes[0..8].copy_from_slice(&m.salt_low.to_le_bytes());
                let salt_hex = format!("0x{}", chains::evm::hex::encode(&salt_bytes));
                let short_salt = format!("0x{}...", chains::evm::hex::encode(&salt_bytes[0..4]));

                let wallet = GeneratedWallet {
                    network: Network::Evm,
                    address: addr_str.clone(),
                    private_key: salt_hex.clone(),
                    rarity: beauty.rarity,
                    theme: beauty.theme,
                    score: beauty.score,
                    title: beauty.title.clone(),
                    pattern: format!("factory: {} | salt: {}", factory_hex, salt_hex),
                    timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                };

                storage.save(&wallet);

                let time_str = Local::now().format("%H:%M:%S").to_string();

                if has_target {
                    let match_len = calc_prefix_match(clean_addr, &clean_target, false);
                    let matched_slice = &clean_addr[..match_len];
                    if match_len >= target_len {
                        println!(
                            "\r[{}] [{}] [CREATE2/GPU] {}  {} | matched: 0x{} --> COMPLETE",
                            time_str.bright_black(),
                            "TARGET: FULL".bright_green().bold(),
                            addr_str.bright_green().bold(),
                            format!("(salt: {})", short_salt).bright_black(),
                            matched_slice.bright_yellow().bold()
                        );
                        if args.count > 0 && total_found.load(Ordering::Relaxed) >= args.count {
                            running.store(false, Ordering::Relaxed);
                            break;
                        }
                    } else {
                        println!(
                            "\r[{}] [STAGE: {}/{}] [CREATE2/GPU] {}  {} | matched: 0x{}",
                            time_str.bright_black(),
                            match_len,
                            target_len,
                            addr_str.bright_white().bold(),
                            format!("(salt: {})", short_salt).bright_black(),
                            matched_slice.bright_yellow().bold()
                        );
                    }
                } else {
                    println!(
                        "\r[{}] [{:<11}] [CREATE2/GPU] {}  {} | {}",
                        time_str.bright_black(),
                        beauty.rarity.badge().bold(),
                        addr_str.bright_white().bold(),
                        format!("(salt: {})", short_salt).bright_black(),
                        beauty.title.bright_green()
                    );
                }

                if args.count > 0 && total_found.load(Ordering::Relaxed) >= args.count {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
            }
        }
    }

    let _ = stats_thread.join();

    println!("\n--------------------------------------------------------------------------------");
    println!("session complete.");
    println!("total salts evaluated: {}", format_number(total_salts.load(Ordering::Relaxed)));
    println!("matches found:         {}", total_found.load(Ordering::Relaxed));
    println!("tier-0 [ex]:           {}", count_godlike.load(Ordering::Relaxed));
    println!("tier-1 [s]:            {}", count_mythic.load(Ordering::Relaxed));
    println!("tier-2 [a]:            {}", count_legendary.load(Ordering::Relaxed));
    println!("tier-3 [b]:            {}", count_epic.load(Ordering::Relaxed));
    println!("vault directory:       {}", args.output_dir);
    println!("--------------------------------------------------------------------------------");
}

#[cfg(target_os = "macos")]
fn run_eoa_engine(args: &Args) {
    let min_rarity = parse_min_rarity(&args.min_rarity);

    let raw_target = if let Some(ref t) = args.target {
        let t_trimmed = t.trim();
        if t_trimmed.starts_with("0x") || t_trimmed.starts_with("0X") {
            t_trimmed[2..].to_string()
        } else {
            t_trimmed.to_string()
        }
    } else {
        String::new()
    };

    let mut clean_target = String::new();
    let mut stripped_non_hex = false;
    for c in raw_target.chars() {
        if c.is_ascii_hexdigit() {
            clean_target.push(c.to_ascii_lowercase());
        } else {
            stripped_non_hex = true;
        }
    }

    if stripped_non_hex && !raw_target.is_empty() {
        eprintln!("[NOTE] Stripped non-hex chars for EVM target: '{}' -> '0x{}'", raw_target, clean_target);
    }

    let mut target_nibbles = Vec::new();
    if !clean_target.is_empty() {
        for c in clean_target.chars() {
            if let Some(d) = c.to_digit(16) {
                target_nibbles.push(d as u8);
            }
        }
    }

    let min_zeros = if !clean_target.is_empty() {
        3u32.min(target_nibbles.len() as u32)
    } else {
        match min_rarity {
            Rarity::Godlike => 8u32,
            Rarity::Mythic => 7u32,
            Rarity::Legendary => 6u32,
            Rarity::Epic => 6u32,
            Rarity::Rare => 6u32,
        }
    };

    let metal_engine = if metal::MetalEngine::is_supported() {
        metal::MetalEngine::init()
    } else {
        None
    };

    let storage = Arc::new(Storage::new(&args.output_dir));
    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();

    let _ = ctrlc::set_handler(move || {
        eprintln!("
interrupt signal received. finalizing writes...");
        r_clone.store(false, Ordering::SeqCst);
    });

    let total_salts = Arc::new(AtomicU64::new(0));
    let total_found = Arc::new(AtomicU32::new(0));
    let count_godlike = Arc::new(AtomicU32::new(0));
    let count_mythic = Arc::new(AtomicU32::new(0));
    let count_legendary = Arc::new(AtomicU32::new(0));
    let count_epic = Arc::new(AtomicU32::new(0));

    let has_target = !clean_target.is_empty();
    let target_len = clean_target.len();
    let initial_stage = if has_target {
        3u32.min(target_len as u32)
    } else {
        0
    };
    let target_stage_atomic = Arc::new(AtomicU32::new(initial_stage));

    println!("
vanity-gen v0.2.0 [darwin/aarch64]");
    if let Some(ref eng) = metal_engine {
        println!("engine:    Metal GPU Compute [{}]", eng.name());
    } else {
        println!("engine:    CPU Fallback (Metal unavailable)");
    }
    println!("mode:      EVM Personal Wallet Vanity Generator (secp256k1)");
    if has_target {
        println!("target:    0x{}", clean_target);
        println!("milestone: progressive stages starting from 3 chars");
    } else {
        let filter_name = match min_rarity {
            Rarity::Godlike => ">= 8 leading zeros [TIER-0 [EX]]",
            Rarity::Mythic => ">= 7 leading zeros [TIER-1 [S]]",
            _ => ">= 6 leading zeros [TIER-2 [A]]",
        };
        println!("filter:    {}", filter_name);
    }
    println!("limit:     {}", if args.count > 0 { format!("{} matches", args.count) } else { "continuous (Ctrl+C to stop)".to_string() });
    println!("vault:     {}", args.output_dir);
    println!("--------------------------------------------------------------------------------");

    let s_running = running.clone();
    let s_salts = total_salts.clone();
    let s_found = total_found.clone();
    let s_has_target = has_target;
    let s_clean_target = clean_target.clone();
    let s_target_stage = target_stage_atomic.clone();
    let start_time = Instant::now();

    let stats_thread = thread::spawn(move || {
        let mut last_salts = 0u64;
        let mut last_time = Instant::now();
        while s_running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(500));
            let now = Instant::now();
            let elapsed_total = now.duration_since(start_time).as_secs();
            let elapsed_interval = now.duration_since(last_time).as_secs_f64();
            let cur_salts = s_salts.load(Ordering::Relaxed);
            let diff = cur_salts.saturating_sub(last_salts);
            last_salts = cur_salts;
            last_time = now;
            let mhs = if elapsed_interval > 0.0 {
                (diff as f64 / elapsed_interval) / 1_000_000.0
            } else {
                0.0
            };
            let mins = elapsed_total / 60;
            let secs = elapsed_total % 60;
            let found = s_found.load(Ordering::Relaxed);
            let display_target = if s_has_target {
                format!("0x{} (stage >= {})", s_clean_target, s_target_stage.load(Ordering::Relaxed))
            } else {
                ">= 6 zeros [LEGENDARY+]".to_string()
            };
            eprint!(
                "\r[{:02}:{:02}] {:6.2} MH/s | keys: {:12} | target: {} | found: {}",
                mins,
                secs,
                mhs,
                format_number(cur_salts),
                display_target,
                found
            );
            let _ = std::io::stdout().flush();
        }
        eprint!("\r{}\r", " ".repeat(100));
    });

    let secp = Secp256k1::new();
    let mut rng = rand::thread_rng();

    let total_threads = 262144u32;
    let keys_per_thread = 8u64;
    let batch_size = (total_threads as u64) * keys_per_thread;

    let mut current_target_stage = initial_stage;

    if let Some(ref engine) = metal_engine {
        while running.load(Ordering::Relaxed) {
            let active_min_zeros = if has_target {
                current_target_stage
            } else {
                min_zeros
            };

            let mut base_priv = [0u8; 32];
            rng.fill_bytes(&mut base_priv);
            base_priv[0] &= 0x7f;
            if base_priv[0] == 0 {
                base_priv[0] = 1;
            }

            let base_sk = match SecretKey::from_slice(&base_priv) {
                Ok(k) => k,
                Err(_) => continue,
            };
            let base_pk = PublicKey::from_secret_key(&secp, &base_sk);
            let base_point = metal::point_from_pubkey(&base_pk);

            let matches = engine.run_eoa_batch(&base_point, total_threads, active_min_zeros, &target_nibbles);

            total_salts.fetch_add(batch_size, Ordering::Relaxed);

            for m in matches {
                let offset = m.salt_low;
                let mut tweak = [0u8; 32];
                tweak[24..32].copy_from_slice(&offset.to_be_bytes());
                let scalar = match Scalar::from_be_bytes(tweak) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let match_sk = match base_sk.add_tweak(&scalar) {
                    Ok(k) => k,
                    Err(_) => continue,
                };
                let priv_hex = chains::evm::hex::encode(&match_sk.secret_bytes());
                let priv_hex_0x = format!("0x{}", priv_hex);
                let short_priv = format!("0x{}...", &priv_hex[..8]);

                let addr_str = EvmGenerator::format_address_checksum(&m.address);
                let clean_addr = addr_str.trim_start_matches("0x");

                let eval_match = if has_target {
                    let match_len = calc_prefix_match(clean_addr, &clean_target, false);
                    if match_len >= current_target_stage as usize {
                        if match_len < target_len {
                            current_target_stage = (match_len as u32 + 1).min(target_len as u32);
                            target_stage_atomic.store(current_target_stage, Ordering::Relaxed);
                        }
                        let rarity = if match_len >= target_len {
                            Rarity::Godlike
                        } else if match_len >= 6 {
                            Rarity::Mythic
                        } else {
                            Rarity::Legendary
                        };
                        let matched_slice = &clean_addr[..match_len];
                        Some(analyzer::BeautyMatch {
                            rarity,
                            theme: Theme::CustomTarget,
                            score: (match_len as u32 * 110).min(1000),
                            title: format!("EVM Target milestone [{}/{}]: 0x{}", match_len, target_len, matched_slice),
                            pattern: format!("Target: 0x{} (matched: 0x{})", clean_target, matched_slice),
                        })
                    } else {
                        None
                    }
                } else {
                    Analyzer::evaluate(Network::Evm, &addr_str)
                };

                let beauty = match eval_match {
                    Some(b) => b,
                    None => continue,
                };

                if beauty.rarity < min_rarity || beauty.rarity < Rarity::Legendary {
                    continue;
                }

                total_found.fetch_add(1, Ordering::Relaxed);
                match beauty.rarity {
                    Rarity::Godlike => { count_godlike.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Mythic => { count_mythic.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Legendary => { count_legendary.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Epic => { count_epic.fetch_add(1, Ordering::Relaxed); }
                    Rarity::Rare => {}
                }

                let wallet = GeneratedWallet {
                    network: Network::Evm,
                    address: addr_str.clone(),
                    private_key: priv_hex_0x.clone(),
                    rarity: beauty.rarity,
                    theme: beauty.theme,
                    score: beauty.score,
                    title: beauty.title.clone(),
                    pattern: beauty.pattern.clone(),
                    timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                };

                storage.save(&wallet);

                let time_str = Local::now().format("%H:%M:%S").to_string();

                if has_target {
                    let match_len = calc_prefix_match(clean_addr, &clean_target, false);
                    let matched_slice = &clean_addr[..match_len];
                    if match_len >= target_len {
                        println!(
                            "\r[{}] [{}] [EVM/GPU] {}  {} | matched: 0x{} --> COMPLETE",
                            time_str.bright_black(),
                            "TARGET: FULL".bright_green().bold(),
                            addr_str.bright_green().bold(),
                            format!("(priv: {})", short_priv).bright_black(),
                            matched_slice.bright_yellow().bold()
                        );
                        if args.count > 0 && total_found.load(Ordering::Relaxed) >= args.count {
                            running.store(false, Ordering::Relaxed);
                            break;
                        }
                    } else {
                        println!(
                            "\r[{}] [STAGE: {}/{}] [EVM/GPU] {}  {} | matched: 0x{}",
                            time_str.bright_black(),
                            match_len,
                            target_len,
                            addr_str.bright_white().bold(),
                            format!("(priv: {})", short_priv).bright_black(),
                            matched_slice.bright_yellow().bold()
                        );
                    }
                } else {
                    let rarity_badge = match beauty.rarity {
                        Rarity::Godlike => "TIER-0 [EX]".black().on_bright_magenta().bold(),
                        Rarity::Mythic => "TIER-1 [S] ".black().on_bright_yellow().bold(),
                        Rarity::Legendary => "TIER-2 [A] ".black().on_bright_cyan().bold(),
                        _ => "TIER-3 [B] ".black().on_bright_blue().bold(),
                    };
                    println!(
                        "\r[{}] [{}] [EVM/GPU] {}  {} | {}",
                        time_str.bright_black(),
                        rarity_badge,
                        addr_str.bright_white().bold(),
                        format!("(priv: {})", short_priv).bright_black(),
                        beauty.title.bright_yellow()
                    );
                }

                if args.count > 0 && total_found.load(Ordering::Relaxed) >= args.count {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
            }
        }
    } else {
        eprintln!("Metal engine not available. Falling back to CPU.");
        run_wallet_engine(args);
        return;
    }

    let _ = stats_thread.join();

    println!("
--------------------------------------------------------------------------------");
    println!("session complete.");
    println!("total keys evaluated:  {}", format_number(total_salts.load(Ordering::Relaxed)));
    println!("matches found:         {}", total_found.load(Ordering::Relaxed));
    println!("tier-0 [ex]:           {}", count_godlike.load(Ordering::Relaxed));
    println!("tier-1 [s]:            {}", count_mythic.load(Ordering::Relaxed));
    println!("tier-2 [a]:            {}", count_legendary.load(Ordering::Relaxed));
    println!("tier-3 [b]:            {}", count_epic.load(Ordering::Relaxed));
    println!("vault directory:       {}", args.output_dir);
    println!("--------------------------------------------------------------------------------");
}

fn run_wallet_engine(args: &Args) {
    let num_threads = if args.threads == 0 {
        num_cpus()
    } else {
        args.threads
    };

    let min_rarity = parse_min_rarity(&args.min_rarity);

    let (clean_target, user_typed_0x, user_typed_bc1q) = if let Some(ref t) = args.target {
        let t_trimmed = t.trim();
        if t_trimmed.starts_with("0x") || t_trimmed.starts_with("0X") {
            (t_trimmed[2..].to_string(), true, false)
        } else if t_trimmed.to_ascii_lowercase().starts_with("bc1q") {
            (t_trimmed[4..].to_string(), false, true)
        } else {
            (t_trimmed.to_string(), false, false)
        }
    } else {
        (String::new(), false, false)
    };

    let active_networks: Vec<Network> = if args.target.is_some() {
        let explicit_net = match args.network.to_ascii_lowercase().as_str() {
            "evm" | "eth" => Some(vec![Network::Evm]),
            "sol" | "solana" => Some(vec![Network::Solana]),
            "btc" | "bitcoin" => Some(vec![Network::Bitcoin]),
            _ => {
                if user_typed_0x {
                    if is_evm_compatible(&clean_target) {
                        Some(vec![Network::Evm])
                    } else {
                        None
                    }
                } else if user_typed_bc1q {
                    if is_bitcoin_compatible(&clean_target) {
                        Some(vec![Network::Bitcoin])
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        if let Some(nets) = explicit_net {
            nets
        } else {
            let mut compatible = Vec::new();
            if is_evm_compatible(&clean_target) {
                compatible.push(Network::Evm);
            }
            if is_solana_compatible(&clean_target, args.case_sensitive) {
                compatible.push(Network::Solana);
            }
            if is_bitcoin_compatible(&clean_target) {
                compatible.push(Network::Bitcoin);
            }

            if compatible.is_empty() {
                vec![Network::Solana]
            } else {
                compatible
            }
        }
    } else {
        match args.network.to_ascii_lowercase().as_str() {
            "evm" | "eth" => vec![Network::Evm],
            "sol" | "solana" => vec![Network::Solana],
            "btc" | "bitcoin" => vec![Network::Bitcoin],
            _ => vec![Network::Evm, Network::Solana, Network::Bitcoin],
        }
    };

    println!("\nvanity-gen v0.2.0 [darwin/aarch64]");
    println!("workers:   {} [secp256k1, ed25519, bech32]", num_threads);
    if let Some(ref t) = args.target {
        println!("target:    {}", t);
        let net_names: Vec<String> = active_networks.iter().map(|n| n.to_string()).collect();
        println!("networks:  {}", net_names.join(", "));
        println!("milestone: >= 5 chars");
    } else {
        let net_names: Vec<String> = active_networks.iter().map(|n| n.to_string()).collect();
        println!("networks:  {}", net_names.join(", "));
        println!("filter:    {}", min_rarity.badge());
        if args.count > 0 {
            println!("limit:     {} records", args.count);
        } else {
            println!("limit:     continuous (Ctrl+C to stop)");
        }
        if let Some(ref p) = args.prefix {
            println!("prefix:    {}", p);
        }
        if let Some(ref s) = args.suffix {
            println!("suffix:    {}", s);
        }
    }
    println!("vault:     {}", args.output_dir);
    println!("--------------------------------------------------------------------------------");

    let running = Arc::new(AtomicBool::new(true));
    let r_clone = running.clone();

    let _ = ctrlc::set_handler(move || {
        eprintln!("\ninterrupt signal received. finalizing writes...");
        r_clone.store(false, Ordering::SeqCst);
    });

    let total_keys = Arc::new(AtomicU64::new(0));
    let total_found = Arc::new(AtomicU32::new(0));
    let count_godlike = Arc::new(AtomicU32::new(0));
    let count_mythic = Arc::new(AtomicU32::new(0));
    let count_legendary = Arc::new(AtomicU32::new(0));
    let count_epic = Arc::new(AtomicU32::new(0));
    let count_rare = Arc::new(AtomicU32::new(0));
    let best_match_len = Arc::new(AtomicUsize::new(2));

    let storage = Arc::new(Storage::new(&args.output_dir));

    let prefix_filter = args.prefix.as_ref().map(|p| p.to_ascii_lowercase());
    let suffix_filter = args.suffix.as_ref().map(|s| s.to_ascii_lowercase());
    let target_query = if clean_target.is_empty() { None } else { Some(clean_target.clone()) };
    let case_sensitive = args.case_sensitive;

    let mut handles = Vec::new();

    for _thread_id in 0..num_threads {
        let running = running.clone();
        let total_keys = total_keys.clone();
        let total_found = total_found.clone();
        let count_godlike = count_godlike.clone();
        let count_mythic = count_mythic.clone();
        let count_legendary = count_legendary.clone();
        let count_epic = count_epic.clone();
        let count_rare = count_rare.clone();
        let best_match_len = best_match_len.clone();
        let storage = storage.clone();
        let prefix_f = prefix_filter.clone();
        let suffix_f = suffix_filter.clone();
        let target_query = target_query.clone();
        let target_count = args.count;
        let thread_networks = active_networks.clone();

        let handle = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let evm_gen = EvmGenerator::new();
            let sol_gen = SolanaGenerator::new();
            let btc_gen = BitcoinGenerator::new();

            let mut local_counter: u64 = 0;
            let num_nets = thread_networks.len();

            while running.load(Ordering::Relaxed) {
                if target_count > 0 && total_found.load(Ordering::Relaxed) >= target_count {
                    running.store(false, Ordering::Relaxed);
                    break;
                }

                let net = thread_networks[(local_counter as usize) % num_nets];

                let (address, private_key) = match net {
                    Network::Evm => {
                        let (raw_addr, priv_bytes) = evm_gen.generate_raw(&mut rng);
                        let addr = EvmGenerator::format_address_checksum(&raw_addr);
                        let priv_hex = format!("0x{}", chains::evm::hex::encode(&priv_bytes));
                        (addr, priv_hex)
                    }
                    Network::Solana => sol_gen.generate(&mut rng),
                    Network::Bitcoin => btc_gen.generate(&mut rng),
                };

                local_counter += 1;
                if local_counter % 200 == 0 {
                    total_keys.fetch_add(200, Ordering::Relaxed);
                }

                if let Some(ref target) = target_query {
                    let clean_addr = match net {
                        Network::Evm => address.trim_start_matches("0x"),
                        Network::Bitcoin => address.trim_start_matches("bc1q"),
                        Network::Solana => &address,
                    };

                    let match_len = calc_prefix_match(clean_addr, target, case_sensitive);
                    let target_len = target.len();

                    let current_best = best_match_len.load(Ordering::Relaxed);
                    let is_new_record = match_len > current_best;

                    let min_save_threshold = if target_len <= 4 { target_len } else { 5 };
                    let should_save = (is_new_record && match_len >= min_save_threshold) || match_len >= target_len;

                    if should_save {
                        if is_new_record {
                            best_match_len.store(match_len, Ordering::Relaxed);
                        }

                        let rarity = if match_len >= target_len {
                            Rarity::Godlike
                        } else if match_len >= 6 {
                            Rarity::Mythic
                        } else {
                            Rarity::Legendary
                        };

                        total_found.fetch_add(1, Ordering::Relaxed);
                        match rarity {
                            Rarity::Godlike => { count_godlike.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Mythic => { count_mythic.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Legendary => { count_legendary.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Epic => { count_epic.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Rare => { count_rare.fetch_add(1, Ordering::Relaxed); }
                        }

                        let matched_slice = &clean_addr[..match_len];
                        let title = format!("Target milestone [{}/{}]: {}", match_len, target_len, matched_slice);
                        let score = (match_len as u32 * 110).min(1000);

                        let wallet = GeneratedWallet {
                            network: net,
                            address: address.clone(),
                            private_key: private_key.clone(),
                            rarity,
                            theme: Theme::CustomTarget,
                            score,
                            title: title.clone(),
                            pattern: format!("Target: {} (matched: {})", target, matched_slice),
                            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        };

                        storage.save(&wallet);

                        let time_str = Local::now().format("%H:%M:%S").to_string();
                        let net_str = match net {
                            Network::Evm => "EVM",
                            Network::Solana => "SOL",
                            Network::Bitcoin => "BTC",
                        };

                        println!(
                            "\r[{}] [TARGET: {}/{}] [{}] {}  (matched: {})",
                            time_str.bright_black(),
                            match_len,
                            target_len,
                            net_str.bright_cyan(),
                            address.bright_white().bold(),
                            matched_slice.bright_yellow()
                        );

                        if match_len >= target_len {
                            println!("\r[{}] [TARGET: FULL] [{}] {}  --> COMPLETE",
                                time_str.bright_black(),
                                net_str.bright_green(),
                                address.bright_green().bold()
                            );
                            running.store(false, Ordering::Relaxed);
                            break;
                        }
                    }

                    if target.len() >= 6 {
                        let half = 3;
                        let start_chunk = &target[..half];
                        let end_chunk = &target[target.len()-half..];

                        let starts_ok = if case_sensitive {
                            clean_addr.starts_with(start_chunk)
                        } else {
                            clean_addr.to_ascii_lowercase().starts_with(&start_chunk.to_ascii_lowercase())
                        };

                        let ends_ok = if case_sensitive {
                            clean_addr.ends_with(end_chunk)
                        } else {
                            clean_addr.to_ascii_lowercase().ends_with(&end_chunk.to_ascii_lowercase())
                        };

                        if starts_ok && ends_ok {
                            total_found.fetch_add(1, Ordering::Relaxed);
                            count_mythic.fetch_add(1, Ordering::Relaxed);

                            let wallet = GeneratedWallet {
                                network: net,
                                address: address.clone(),
                                private_key: private_key.clone(),
                                rarity: Rarity::Mythic,
                                theme: Theme::Mirrors,
                                score: 980,
                                title: format!("Symmetric combo: {}...{}", start_chunk, end_chunk),
                                pattern: format!("{}...{}", start_chunk, end_chunk),
                                timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            };

                            storage.save(&wallet);

                            let time_str = Local::now().format("%H:%M:%S").to_string();
                            let net_str = match net {
                                Network::Evm => "EVM",
                                Network::Solana => "SOL",
                                Network::Bitcoin => "BTC",
                            };

                            println!(
                                "\r[{}] [BOOKEND] [{}] {}  ({}...{})",
                                time_str.bright_black(),
                                net_str.bright_cyan(),
                                address.bright_white().bold(),
                                start_chunk.bright_yellow(),
                                end_chunk.bright_yellow()
                            );
                        }
                    }

                    continue;
                }

                let custom_matched = if prefix_f.is_some() || suffix_f.is_some() {
                    let addr_lower = address.to_ascii_lowercase();
                    let clean_addr = addr_lower
                        .trim_start_matches("0x")
                        .trim_start_matches("bc1q");
                    
                    let p_ok = match &prefix_f {
                        Some(p) => clean_addr.starts_with(p),
                        None => true,
                    };
                    let s_ok = match &suffix_f {
                        Some(s) => clean_addr.ends_with(s),
                        None => true,
                    };
                    p_ok && s_ok
                } else {
                    false
                };

                let eval_result = if prefix_f.is_some() || suffix_f.is_some() {
                    if custom_matched {
                        Some(analyzer::BeautyMatch {
                            rarity: Rarity::Mythic,
                            theme: Theme::CustomTarget,
                            score: 999,
                            title: format!("Custom filter: prefix={:?}, suffix={:?}", prefix_f, suffix_f),
                            pattern: format!("{:?} + {:?}", prefix_f, suffix_f),
                        })
                    } else {
                        None
                    }
                } else {
                    Analyzer::evaluate(net, &address)
                };

                if let Some(beauty) = eval_result {
                    if beauty.rarity >= min_rarity {
                        total_found.fetch_add(1, Ordering::Relaxed);
                        match beauty.rarity {
                            Rarity::Godlike => { count_godlike.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Mythic => { count_mythic.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Legendary => { count_legendary.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Epic => { count_epic.fetch_add(1, Ordering::Relaxed); }
                            Rarity::Rare => { count_rare.fetch_add(1, Ordering::Relaxed); }
                        }

                        let wallet = GeneratedWallet {
                            network: net,
                            address: address.clone(),
                            private_key: private_key.clone(),
                            rarity: beauty.rarity,
                            theme: beauty.theme,
                            score: beauty.score,
                            title: beauty.title.clone(),
                            pattern: beauty.pattern.clone(),
                            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        };

                        storage.save(&wallet);

                        let time_str = Local::now().format("%H:%M:%S").to_string();
                        let net_str = match net {
                            Network::Evm => "EVM",
                            Network::Solana => "SOL",
                            Network::Bitcoin => "BTC",
                        };

                        println!(
                            "\r[{}] [{:<11}] [{}] {}  {} | {}",
                            time_str.bright_black(),
                            beauty.rarity.badge().bold(),
                            net_str.bright_cyan(),
                            address.bright_white().bold(),
                            format!("(score: {})", beauty.score).bright_black(),
                            beauty.title.bright_green()
                        );
                    }
                }
            }

            total_keys.fetch_add(local_counter % 200, Ordering::Relaxed);
        });

        handles.push(handle);
    }

    let running_stats = running.clone();
    let total_k = total_keys.clone();
    let c_g = count_godlike.clone();
    let c_m = count_mythic.clone();
    let c_l = count_legendary.clone();
    let c_e = count_epic.clone();
    let best_len_stats = best_match_len.clone();
    let has_target = !clean_target.is_empty();
    let target_display = clean_target.clone();

    let stats_thread = thread::spawn(move || {
        let start_time = Instant::now();
        let mut last_check = 0u64;
        let mut last_time = Instant::now();

        while running_stats.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(500));
            let now = Instant::now();
            let current = total_k.load(Ordering::Relaxed);
            let dt = now.duration_since(last_time).as_secs_f64();
            let diff = current.saturating_sub(last_check);
            let speed = (diff as f64 / dt) as u64;

            last_check = current;
            last_time = now;

            let elapsed = start_time.elapsed().as_secs();
            let elapsed_str = format!("{:02}:{:02}", elapsed / 60, elapsed % 60);

            if has_target {
                let best = best_len_stats.load(Ordering::Relaxed);
                let stats_line = format!(
                    "[{}] {:>6.1} kH/s | keys: {:>10} | target: {} | best: {}/{}",
                    elapsed_str,
                    speed as f64 / 1000.0,
                    format_number(current),
                    target_display,
                    best,
                    target_display.len()
                );
                eprint!("\r{}", stats_line);
            } else {
                let stats_line = format!(
                    "[{}] {:>6.1} kH/s | keys: {:>10} | t0: {} | t1: {} | t2: {} | t3: {}",
                    elapsed_str,
                    speed as f64 / 1000.0,
                    format_number(current),
                    c_g.load(Ordering::Relaxed),
                    c_m.load(Ordering::Relaxed),
                    c_l.load(Ordering::Relaxed),
                    c_e.load(Ordering::Relaxed),
                );
                eprint!("\r{}", stats_line);
            }
        }
        eprintln!();
    });

    for h in handles {
        let _ = h.join();
    }
    let _ = stats_thread.join();

    println!("\n--------------------------------------------------------------------------------");
    println!("session complete.");
    println!("total keys evaluated: {}", format_number(total_keys.load(Ordering::Relaxed)));
    println!("matches found:        {}", total_found.load(Ordering::Relaxed));
    println!("tier-0 [ex]:          {}", count_godlike.load(Ordering::Relaxed));
    println!("tier-1 [s]:           {}", count_mythic.load(Ordering::Relaxed));
    println!("tier-2 [a]:           {}", count_legendary.load(Ordering::Relaxed));
    println!("tier-3 [b]:           {}", count_epic.load(Ordering::Relaxed));
    println!("vault directory:      {}", args.output_dir);
    println!("--------------------------------------------------------------------------------");
}

fn calc_prefix_match(addr: &str, target: &str, case_sensitive: bool) -> usize {
    let mut matched = 0;
    let mut addr_chars = addr.chars();
    let mut target_chars = target.chars();

    while let (Some(a), Some(t)) = (addr_chars.next(), target_chars.next()) {
        let eq = if case_sensitive {
            a == t
        } else {
            a.to_ascii_lowercase() == t.to_ascii_lowercase()
        };

        if eq {
            matched += 1;
        } else {
            break;
        }
    }

    matched
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8)
}
