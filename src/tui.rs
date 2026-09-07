use std::collections::HashMap;
use std::fs;
use std::io::{self, stdout, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState, Tabs},
    Terminal,
};

use crate::types::{GeneratedWallet, Network, Rarity, Theme};

pub fn run_tui(output_dir: &str) -> io::Result<()> {
    let wallets = load_all_wallets(output_dir);

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, wallets);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

pub fn load_all_wallets(output_dir: &str) -> Vec<GeneratedWallet> {
    let mut wallets_map: HashMap<String, GeneratedWallet> = HashMap::new();

    let all_json_path = format!("{}/wallets_all.json", output_dir);
    if let Ok(content) = fs::read_to_string(&all_json_path) {
        if let Ok(list) = serde_json::from_str::<Vec<GeneratedWallet>>(&content) {
            for w in list {
                wallets_map.insert(w.address.clone(), w);
            }
        }
    }

    let json_files = find_files_with_ext(output_dir, "json");
    for path in json_files {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(list) = serde_json::from_str::<Vec<GeneratedWallet>>(&content) {
                for w in list {
                    wallets_map.entry(w.address.clone()).or_insert(w);
                }
            }
        }
    }

    let txt_files = find_files_with_ext(output_dir, "txt");
    for path in txt_files {
        if let Ok(content) = fs::read_to_string(&path) {
            for w in parse_txt_cards(&content) {
                wallets_map.entry(w.address.clone()).or_insert(w);
            }
        }
    }

    let mut wallets: Vec<GeneratedWallet> = wallets_map.into_values().collect();

    wallets.sort_by(|a, b| {
        b.rarity
            .cmp(&a.rarity)
            .then_with(|| b.score.cmp(&a.score))
            .then_with(|| b.timestamp.cmp(&a.timestamp))
    });

    if !wallets.is_empty() {
        if let Ok(serialized) = serde_json::to_string_pretty(&wallets) {
            let _ = fs::write(&all_json_path, serialized);
        }
    }

    wallets
}

fn find_files_with_ext(dir: &str, ext: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let walk_dir = Path::new(dir);
    if !walk_dir.exists() {
        return files;
    }
    let mut stack = vec![walk_dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|s| s.to_str()) == Some(ext) {
                    files.push(path);
                }
            }
        }
    }
    files
}

fn parse_txt_cards(content: &str) -> Vec<GeneratedWallet> {
    let mut results = Vec::new();
    let delim = if content.contains("--------------------------------------------------------------------------------") {
        "--------------------------------------------------------------------------------"
    } else {
        "=================================================="
    };

    let blocks = content.split(delim);

    for block in blocks {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut network = None;
        let mut address = None;
        let mut private_key = None;
        let mut rarity = None;
        let mut theme = None;
        let mut score = 800u32;
        let mut title = None;
        let mut pattern = None;
        let mut timestamp = None;

        for line in trimmed.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("tier:").or_else(|| line.strip_prefix("Ранг:")) {
                let r_str = rest.trim();
                if r_str.contains("TIER-0") || r_str.contains("GODLIKE") {
                    rarity = Some(Rarity::Godlike);
                } else if r_str.contains("TIER-1") || r_str.contains("MYTHIC") {
                    rarity = Some(Rarity::Mythic);
                } else if r_str.contains("TIER-2") || r_str.contains("LEGENDARY") {
                    rarity = Some(Rarity::Legendary);
                } else if r_str.contains("TIER-3") || r_str.contains("EPIC") {
                    rarity = Some(Rarity::Epic);
                } else if r_str.contains("TIER-4") || r_str.contains("RARE") {
                    rarity = Some(Rarity::Rare);
                }

                if let Some(score_start) = r_str.find("score: ").or_else(|| r_str.find("Score: ")) {
                    if let Some(score_end) = r_str[score_start + 7..].find(')') {
                        if let Ok(sc) = r_str[score_start + 7..score_start + 7 + score_end].trim().parse::<u32>() {
                            score = sc;
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("category:").or_else(|| line.strip_prefix("Категория:")) {
                let c_str = rest.trim();
                if c_str.contains("REPDIGIT") || c_str.contains("ПОВТОР") {
                    theme = Some(Theme::Repdigits);
                } else if c_str.contains("LADDER") || c_str.contains("ЛЕСЕНК") {
                    theme = Some(Theme::Ladders);
                } else if c_str.contains("BINARY") || c_str.contains("ЗЕБР") {
                    theme = Some(Theme::Alternating);
                } else if c_str.contains("COMBINATION") || c_str.contains("ПОКЕР") {
                    theme = Some(Theme::Poker);
                } else if c_str.contains("BOOKEND") || c_str.contains("ЗЕРКАЛ") || c_str.contains("КАРКАС") {
                    theme = Some(Theme::Mirrors);
                } else if c_str.contains("DICTIONARY") || c_str.contains("БЛАТН") {
                    theme = Some(Theme::Street);
                } else if c_str.contains("STATUS") || c_str.contains("VIP") {
                    theme = Some(Theme::LuxuryStatus);
                } else if c_str.contains("PROTOCOL") || c_str.contains("КРИПТ") {
                    theme = Some(Theme::CryptoCult);
                } else if c_str.contains("HEX_CONST") || c_str.contains("HEX") {
                    theme = Some(Theme::HackerHex);
                } else if c_str.contains("TARGET") || c_str.contains("ЦЕЛЬ") {
                    theme = Some(Theme::CustomTarget);
                }
            } else if let Some(rest) = line.strip_prefix("title:").or_else(|| line.strip_prefix("Описание:")) {
                title = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("network:").or_else(|| line.strip_prefix("Сеть:")) {
                let n_str = rest.trim();
                if n_str.contains("EVM") {
                    network = Some(Network::Evm);
                } else if n_str.contains("SOL") || n_str.contains("Solana") {
                    network = Some(Network::Solana);
                } else if n_str.contains("BTC") || n_str.contains("Bitcoin") {
                    network = Some(Network::Bitcoin);
                }
            } else if let Some(rest) = line.strip_prefix("address:").or_else(|| line.strip_prefix("Адрес:")) {
                address = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("private_key:").or_else(|| line.strip_prefix("Приватный ключ:")) {
                private_key = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("timestamp:").or_else(|| line.strip_prefix("Время находки:")) {
                timestamp = Some(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("pattern:").or_else(|| line.strip_prefix("Паттерн:")) {
                pattern = Some(rest.trim().to_string());
            }
        }

        if let (Some(net), Some(addr), Some(pk)) = (network, address, private_key) {
            results.push(GeneratedWallet {
                network: net,
                address: addr,
                private_key: pk,
                rarity: rarity.unwrap_or(Rarity::Legendary),
                theme: theme.unwrap_or(Theme::CustomTarget),
                score,
                title: title.unwrap_or_else(|| "Match".to_string()),
                pattern: pattern.unwrap_or_default(),
                timestamp: timestamp.unwrap_or_else(|| "2026-09-07 00:00:00".to_string()),
            });
        }
    }

    results
}

struct App {
    wallets: Vec<GeneratedWallet>,
    table_state: TableState,
    selected_theme_tab: usize,
    show_private_key: bool,
    status_message: Option<(String, Instant)>,
}

const THEME_TABS: &[(&str, Option<Theme>)] = &[
    ("ALL", None),
    ("REPDIGITS", Some(Theme::Repdigits)),
    ("LADDERS", Some(Theme::Ladders)),
    ("MATRIX", Some(Theme::Alternating)),
    ("BOOKENDS", Some(Theme::Mirrors)),
    ("DICTIONARY", Some(Theme::Street)),
    ("STATUS", Some(Theme::LuxuryStatus)),
    ("PROTOCOL", Some(Theme::CryptoCult)),
    ("HEX", Some(Theme::HackerHex)),
    ("TARGETS", Some(Theme::CustomTarget)),
];

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    wallets: Vec<GeneratedWallet>,
) -> io::Result<()> {
    let mut app = App {
        wallets,
        table_state: TableState::default(),
        selected_theme_tab: 0,
        show_private_key: true,
        status_message: None,
    };

    if !app.wallets.is_empty() {
        app.table_state.select(Some(0));
    }

    loop {
        let filtered_indices = get_filtered_indices(&app);

        if let Some(selected) = app.table_state.selected() {
            if !filtered_indices.is_empty() && selected >= filtered_indices.len() {
                app.table_state.select(Some(filtered_indices.len() - 1));
            }
        } else if !filtered_indices.is_empty() {
            app.table_state.select(Some(0));
        }

        terminal.draw(|f| ui(f, &mut app, &filtered_indices))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Down | KeyCode::Char('j') => {
                            if !filtered_indices.is_empty() {
                                let next = match app.table_state.selected() {
                                    Some(i) => {
                                        if i + 1 < filtered_indices.len() {
                                            i + 1
                                        } else {
                                            0
                                        }
                                    }
                                    None => 0,
                                };
                                app.table_state.select(Some(next));
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if !filtered_indices.is_empty() {
                                let prev = match app.table_state.selected() {
                                    Some(i) => {
                                        if i > 0 {
                                            i - 1
                                        } else {
                                            filtered_indices.len() - 1
                                        }
                                    }
                                    None => 0,
                                };
                                app.table_state.select(Some(prev));
                            }
                        }
                        KeyCode::Tab => {
                            app.selected_theme_tab = (app.selected_theme_tab + 1) % THEME_TABS.len();
                            app.table_state.select(Some(0));
                        }
                        KeyCode::BackTab => {
                            if app.selected_theme_tab == 0 {
                                app.selected_theme_tab = THEME_TABS.len() - 1;
                            } else {
                                app.selected_theme_tab -= 1;
                            }
                            app.table_state.select(Some(0));
                        }
                        KeyCode::Char(' ') => {
                            app.show_private_key = !app.show_private_key;
                            let state = if app.show_private_key { "VISIBLE" } else { "MASKED" };
                            app.status_message = Some((
                                format!("Private key view: {}", state),
                                Instant::now(),
                            ));
                        }
                        KeyCode::Char('c') | KeyCode::Enter => {
                            if let Some(selected_idx) = app.table_state.selected() {
                                if let Some(&actual_idx) = filtered_indices.get(selected_idx) {
                                    let addr = &app.wallets[actual_idx].address;
                                    copy_to_clipboard(addr);
                                    app.status_message = Some((
                                        format!("Copied address: {}", addr),
                                        Instant::now(),
                                    ));
                                }
                            }
                        }
                        KeyCode::Char('p') => {
                            if let Some(selected_idx) = app.table_state.selected() {
                                if let Some(&actual_idx) = filtered_indices.get(selected_idx) {
                                    let priv_key = &app.wallets[actual_idx].private_key;
                                    copy_to_clipboard(priv_key);
                                    app.status_message = Some((
                                        format!("Copied private key: {}", priv_key),
                                        Instant::now(),
                                    ));
                                }
                            }
                        }
                        KeyCode::Char(digit) if digit.is_ascii_digit() => {
                            let idx = digit.to_digit(10).unwrap() as usize;
                            if idx < THEME_TABS.len() {
                                app.selected_theme_tab = idx;
                                app.table_state.select(Some(0));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn get_filtered_indices(app: &App) -> Vec<usize> {
    let (_, current_theme_filter) = THEME_TABS[app.selected_theme_tab];
    app.wallets
        .iter()
        .enumerate()
        .filter_map(|(idx, w)| {
            if let Some(target_theme) = current_theme_filter {
                if w.theme == target_theme {
                    Some(idx)
                } else {
                    None
                }
            } else {
                Some(idx)
            }
        })
        .collect()
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut child) = Command::new("pbcopy").stdin(std::process::Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App, filtered_indices: &[usize]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(7),
            Constraint::Length(2),
        ])
        .split(f.area());

    let tab_titles: Vec<Line> = THEME_TABS
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let style = if i == app.selected_theme_tab {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(vec![Span::raw(format!("[{}:{}]", i, name))]).style(style)
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(
                    " VANITY-VAULT :: KEY EXPLORER (indexed: {} addresses) ",
                    app.wallets.len()
                ))
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .select(app.selected_theme_tab);
    f.render_widget(tabs, chunks[0]);

    let header_cells = ["TIER", "NET", "ADDRESS", "TYPE", "PATTERN", "TIMESTAMP"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = filtered_indices
        .iter()
        .map(|&idx| {
            let w = &app.wallets[idx];
            let (rarity_label, color) = match w.rarity {
                Rarity::Godlike => ("TIER-0 [EX]", Color::White),
                Rarity::Mythic => ("TIER-1 [S]", Color::Cyan),
                Rarity::Legendary => ("TIER-2 [A]", Color::Green),
                Rarity::Epic => ("TIER-3 [B]", Color::Yellow),
                Rarity::Rare => ("TIER-4 [C]", Color::DarkGray),
            };

            let net_str = match w.network {
                Network::Evm => "EVM",
                Network::Solana => "SOL",
                Network::Bitcoin => "BTC",
            };

            let theme_str = w.theme.badge();

            Row::new(vec![
                Cell::from(rarity_label).style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Cell::from(net_str).style(Style::default().fg(Color::LightBlue)),
                Cell::from(w.address.as_str()).style(Style::default().fg(Color::White)),
                Cell::from(theme_str).style(Style::default().fg(Color::DarkGray)),
                Cell::from(w.pattern.as_str()).style(Style::default().fg(Color::LightCyan)),
                Cell::from(w.timestamp.as_str()).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(13),
            Constraint::Length(6),
            Constraint::Min(44),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(20),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(format!(
                " RECORDS ({}/{}) ",
                filtered_indices.len(),
                app.wallets.len()
            )),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(30, 40, 60))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, chunks[1], &mut app.table_state);

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" KEY METADATA ");

    if let Some(selected) = app.table_state.selected() {
        if let Some(&actual_idx) = filtered_indices.get(selected) {
            let w = &app.wallets[actual_idx];
            let priv_display = if app.show_private_key {
                w.private_key.clone()
            } else {
                "*".repeat(w.private_key.len().min(40)) + " [masked, press Space to reveal]"
            };

            let priv_color = if app.show_private_key { Color::LightGreen } else { Color::DarkGray };

            let details = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("Address:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&w.address, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  [{}]", w.network), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Private Key:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(priv_display, Style::default().fg(priv_color).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Metadata:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled(w.rarity.badge(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw(" | "),
                    Span::styled(w.theme.badge(), Style::default().fg(Color::White)),
                    Span::raw(format!(" | score: {}", w.score)),
                    Span::raw(format!(" | time: {}", w.timestamp)),
                ]),
                Line::from(vec![
                    Span::styled("Details:      ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&w.title, Style::default().fg(Color::White)),
                    Span::styled(" | pattern: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&w.pattern, Style::default().fg(Color::Cyan)),
                ]),
            ])
            .block(detail_block);

            f.render_widget(details, chunks[2]);
        }
    } else {
        let empty = Paragraph::new("No records found in this category")
            .style(Style::default().fg(Color::DarkGray))
            .block(detail_block);
        f.render_widget(empty, chunks[2]);
    }

    let status_text = if let Some((msg, time)) = &app.status_message {
        if time.elapsed() < Duration::from_secs(3) {
            Span::styled(msg, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(
                "[↑/↓/j/k] Navigate  │  [Tab] Category  │  [c/Enter] Copy Address  │  [p] Copy Private Key  │  [Space] Toggle Key  │  [q] Quit",
                Style::default().fg(Color::DarkGray),
            )
        }
    } else {
        Span::styled(
            "[↑/↓/j/k] Navigate  │  [Tab] Category  │  [c/Enter] Copy Address  │  [p] Copy Private Key  │  [Space] Toggle Key  │  [q] Quit",
            Style::default().fg(Color::DarkGray),
        )
    };

    let footer = Paragraph::new(Line::from(vec![status_text]));
    f.render_widget(footer, chunks[3]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_all_wallets() {
        let wallets = load_all_wallets("output");
        assert!(!wallets.is_empty());
        for w in &wallets {
            assert!(!w.address.is_empty());
            assert!(!w.private_key.is_empty());
        }
    }

    #[test]
    fn test_parse_txt_card() {
        let sample = r#"
--------------------------------------------------------------------------------
timestamp:    2026-09-07 23:28:17
network:      EVM
address:      0x000008275ae760b6d8E3877469BF7942776b10F4
private_key:  0x8dbc9764b977dc7641205b5177f2d0e0774b8d63b55e3838943b1070f9711b2c
tier:         TIER-2 [A] (score: 850)
category:     HEX_CONST
title:        Zero prefix: 5 consecutive zeros
pattern:      00000
--------------------------------------------------------------------------------
"#;
        let parsed = parse_txt_cards(sample);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].address, "0x000008275ae760b6d8E3877469BF7942776b10F4");
        assert_eq!(parsed[0].private_key, "0x8dbc9764b977dc7641205b5177f2d0e0774b8d63b55e3838943b1070f9711b2c");
        assert_eq!(parsed[0].rarity, Rarity::Legendary);
        assert_eq!(parsed[0].network, Network::Evm);
    }
}
