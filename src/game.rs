use std::collections::HashMap;
use std::io;
use std::path::Path;

use crossbeam_channel::Receiver;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use rand;
use serde::{Deserialize, Serialize};

use crate::big_num::BigDollar;
use crate::upgrade::{get_upgrades, UpgradeId};

const WASTELAND: &str = include_str!("../assets/wasteland.txt");
const INTRO: &str = include_str!("../assets/intro.txt");
pub const NUM_LETTERS: usize = 26;
pub const LETTER_QUEUE_HEIGHT: usize = 10;
pub const MAX_TRUST_LEVEL: i32 = 100;
pub const TRUST_SCALE: f64 = 1.15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowPanes {
    TextPane,
    UpgradePane,
    AutoPane,
}

pub const UPGRADE_KEYS: [char; 10] = ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g'];

fn idx_to_letter(idx: usize) -> char {
    (b'a' + idx as u8) as char
}

fn idx_to_upper_letter(idx: usize) -> char {
    (b'A' + idx as u8) as char
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameState {
    // currently typed string
    pub typed: String,

    // Text variables
    #[serde(skip, default = "GameState::default_current_text")]
    pub current_text: Vec<&'static str>,
    pub current_line: usize,

    // auto info
    pub counts: HashMap<String, u32>,

    // displayed stats
    pub money: BigDollar,

    // secret stats
    pub high_water_money: BigDollar,
    pub total_money_earned: BigDollar,

    // progression variables
    pub upgrade_levels: HashMap<UpgradeId, usize>,
    pub trust_level: i32,
    pub base_letter_value: BigDollar,
    pub streaks_unlocked: bool,
    pub capital_letter_bonus_unlocked: bool,
    pub automation_unlocked: bool,
    pub seniority_level: u8,
    pub letter_compression_unlocked: bool,
    pub disable_penalty: bool,

    // top left is 0, 0
    // down is increasing y, right is increasing x
    pub window_x: u16,
    pub window_y: u16,
    pub current_pane: WindowPanes,
    previous_window_x: u16,
    previous_window_y: u16,


    pub letter_queue: [[char; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
}

impl GameState {
    fn default_current_text() -> Vec<&'static str> {
        // WASTELAND.split('\n').collect()
        INTRO.split('\n').collect()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            typed: String::new(),
            current_text: Self::default_current_text(),
            current_line: 0,
            counts: HashMap::new(),
            money: BigDollar::from(0),
            high_water_money: BigDollar::from(0),
            total_money_earned: BigDollar::from(0),
            upgrade_levels: HashMap::new(),
            trust_level: 0,
            base_letter_value: BigDollar::from(0.003),
            streaks_unlocked: false,
            capital_letter_bonus_unlocked: false,
            automation_unlocked: false,
            seniority_level: 0,
            letter_compression_unlocked: false,
            disable_penalty: false,
            window_x: 1,
            window_y: 1,
            current_pane: WindowPanes::TextPane,
            previous_window_x: 1,
            previous_window_y: 1,
            letter_queue: [[' '; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
        }
    }
}

pub struct Game {
    input_rx: Receiver<KeyEvent>,
    pub should_quit: bool,

    pub last_spawn_time: std::time::Instant,
    pub last_update_time: std::time::Instant,
    pub game_state: GameState,
}

impl Game {
    pub fn new(input_rx: Receiver<KeyEvent>) -> Self {
        Self::from_state(input_rx, Self::default_state())
    }

    pub fn from_state(input_rx: Receiver<KeyEvent>, game_state: GameState) -> Self {
        Self {
            input_rx,
            should_quit: false,
            last_spawn_time: std::time::Instant::now(),
            last_update_time: std::time::Instant::now(),
            game_state,
        }
    }

    pub fn default_state() -> GameState {
        GameState::default()
    }

    pub fn load_state(path: &Path) -> io::Result<GameState> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(io::Error::other)
    }

    pub fn save_state(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.game_state).map_err(io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn increment_money(&mut self, amount: BigDollar) {
        self.game_state.money += amount;
        self.game_state.high_water_money = self.game_state.high_water_money.max(self.game_state.money);
        self.game_state.total_money_earned += amount;
    }

    pub fn decrement_money(&mut self, amount: BigDollar) {
        self.game_state.money -= amount;
    }

    pub fn increment(&mut self, key: &str) {
        *self.game_state.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    pub fn calc_money_change(&self, typed: &str, reference: &str) -> BigDollar {
        let mut money_change: BigDollar = BigDollar::from(0);
        let typed_chars: Vec<char> = typed.chars().collect();
        let ref_chars: Vec<char> = reference.chars().collect();
        let len = typed_chars.len().max(ref_chars.len());

        for i in 0..len {
            match (typed_chars.get(i), ref_chars.get(i)) {
                (Some(&c), Some(&t)) if c == t => {
                    let mult = if c.is_ascii_uppercase() && self.game_state.capital_letter_bonus_unlocked {
                        5.0
                    } else {
                        1.0
                    };
                    money_change += self.game_state.base_letter_value * TRUST_SCALE.powi(self.game_state.trust_level) * mult;
                    logging::debug(&format!("money change was {money_change}"));
                }
                // Wrong char, missing char, or extra typed char
                _ => {
                    if self.game_state.disable_penalty {
                        continue;
                    }

                    money_change -= self.game_state.base_letter_value * 5.0 * TRUST_SCALE.powi(self.game_state.trust_level);
                }
            }
        }

        money_change
    }

    pub fn calc_trust(&self, ref_chars: &str, typed_chars: &str, trust_level: i32) -> i32 {
        let percentage_correct = if ref_chars.is_empty() {
            0.0
        } else {
            typed_chars.chars().zip(ref_chars.chars()).filter(|(t, r)| t == r).count() as f64 / ref_chars.len() as f64
        };
        if (percentage_correct * 5.0).round() >= (5.0 - self.game_state.seniority_level as f64 * 1.0).round() {
            (trust_level + 1).min(MAX_TRUST_LEVEL)
        } else {
            trust_level.min(0)
        }
    }


    fn recalculate_current_pane(&mut self) {
        match self.game_state.automation_unlocked {
            false => {
                self.game_state.window_x = self.game_state.window_x.min(0);
                self.game_state.current_pane = match (self.game_state.window_x, self.game_state.window_y) {
                    (0, 0) => WindowPanes::UpgradePane,
                    (0, 1) => WindowPanes::TextPane,
                    _ => WindowPanes::TextPane,
                }
            }
            true => {
                self.game_state.window_x = self.game_state.window_x.min(1);
                self.game_state.current_pane = match (self.game_state.window_x, self.game_state.window_y) {
                    (0, 0) => WindowPanes::UpgradePane,
                    (0, 1) => WindowPanes::TextPane,
                    (1, _) => WindowPanes::AutoPane,
                    _ => WindowPanes::TextPane,
                };
            }
        }
    }

    fn toggle_upgrade_pane(&mut self) {
        self.recalculate_current_pane();
        match self.game_state.current_pane {
            WindowPanes::UpgradePane => {
                self.game_state.window_x = self.game_state.previous_window_x;
                self.game_state.window_y = self.game_state.previous_window_y;
            }
            _ => {
                self.game_state.previous_window_x = self.game_state.window_x;
                self.game_state.previous_window_y = self.game_state.window_y;
                self.game_state.window_x = 0;
                self.game_state.window_y = 0;
            }
        }
        self.recalculate_current_pane();
    }

    fn text_pane_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.game_state.typed.push(c);
            }
            KeyCode::Enter => {
                let current_line: &'static str = self.game_state.current_text[self.game_state.current_line];
                let typed_chars: Vec<char> = self.game_state.typed.chars().collect();
                let ref_chars: Vec<char> = current_line.chars().collect();
                let money_change = self.calc_money_change(&self.game_state.typed, current_line);

                logging::debug(&format!(
                    "Scored line {}: money_change={money_change} (typed={} chars, ref={} chars)",
                    self.game_state.current_line,
                    typed_chars.len(),
                    ref_chars.len()
                ));
                self.increment_money(money_change);
                if self.game_state.streaks_unlocked {
                    self.game_state.trust_level = self.calc_trust(&String::from_iter(ref_chars.clone()), &String::from_iter(typed_chars.clone()), self.game_state.trust_level);
                }
                self.game_state.typed = String::new();
                self.game_state.current_line += 1;
            }
            KeyCode::Backspace => {
                self.game_state.typed.pop();
            }
            _ => {}
        }
    }

    pub fn buy_upgrade(&mut self, upgrade_id: UpgradeId) {
        let Some(upgrade) = get_upgrades().get(&upgrade_id) else {
            logging::info(&format!("Upgrade {:?} does not exist", upgrade_id));
            return;
        };

        if !(upgrade.upgrade_unlock_condition)(&self.game_state) {
            logging::info(&format!("Upgrade {:?} is locked", upgrade_id));
            return;
        }

        let level = self.game_state.upgrade_levels.get(&upgrade_id).copied().unwrap_or(0);
        if level >= upgrade.costs.len() {
            logging::info(&format!("Upgrade {:?} is maxed out", upgrade_id));
            return;
        }

        let upgrade_cost = &upgrade.costs[level];
        if self.game_state.money >= *upgrade_cost || *upgrade_cost == BigDollar::from(0) {
            self.decrement_money(*upgrade_cost);
            (upgrade.on_buy)(&mut self.game_state);
            if !upgrade.infinite {
                *self.game_state.upgrade_levels.entry(upgrade_id).or_insert(0) += 1;
            }
            logging::info(&format!(
                "Bought upgrade {:?}",
                upgrade_id,
            ));
        } else {
            logging::info(&format!(
                "Not enough money to buy upgrade {:?}: cost is {}, current money is {}",
                upgrade_id, upgrade_cost, self.game_state.money
            ));
        }
    }

    pub fn get_displayed_upgrades(&self) -> Vec<UpgradeId> {
        get_upgrades()
            .iter()
            .filter(|(_, upgrade)| (upgrade.upgrade_unlock_condition)(&self.game_state))
            .filter(|(upgrade_id, upgrade)| self.game_state.upgrade_levels.get(upgrade_id).map_or(true, |&level| level < upgrade.costs.len()))
            .map(|(upgrade_id, _)| *upgrade_id)
            .collect()
    }

    pub fn upgrade_pane_input(&mut self, key: KeyCode) {
        if let KeyCode::Char(c) = key {
            let Some(&upgrade_id) = self.get_displayed_upgrades()
                .iter().enumerate()
                .find(|(i, _)| UPGRADE_KEYS.get(*i) == Some(&c))
                .map(|(_, upgrade_id)| upgrade_id) else {
                    logging::info(&format!("No upgrade found for key '{}'", c));
                    return;
                };
            self.buy_upgrade(upgrade_id);
        }
    }

    fn update_handle_inputs(&mut self) {
        while let Ok(key) = self.input_rx.try_recv() {
            match key.code {
                KeyCode::Esc => self.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true;
                }
                KeyCode::Tab => self.toggle_upgrade_pane(),
                KeyCode::Up => self.game_state.window_y = self.game_state.window_y.saturating_sub(1),
                KeyCode::Down => self.game_state.window_y = self.game_state.window_y.saturating_add(1).min(1),
                KeyCode::Left => self.game_state.window_x = self.game_state.window_x.saturating_sub(1),
                KeyCode::Right => self.game_state.window_x = self.game_state.window_x.saturating_add(1).min(1),
                _ => {}
            }
            self.recalculate_current_pane();
            match self.game_state.current_pane {
                WindowPanes::TextPane => self.text_pane_input(key.code),
                WindowPanes::UpgradePane => self.upgrade_pane_input(key.code),
                WindowPanes::AutoPane => {
                    if let KeyCode::Char(c) = key.code {
                        self.increment(&c.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    fn letter_queue_update(&mut self, now: std::time::Instant) {
        // compress letters in the queue if letter_compression_unlocked is true
        if self.game_state.letter_compression_unlocked {
            for x in 0..NUM_LETTERS {
                let mut start_y = LETTER_QUEUE_HEIGHT;
                for y in (4..LETTER_QUEUE_HEIGHT).rev() {
                    if self.game_state.letter_queue[x][y] == idx_to_letter(x) {
                        start_y = y;
                        break;
                    }
                }
                if start_y == LETTER_QUEUE_HEIGHT {
                    continue;
                }
                let mut compression_possible = true;
                for y in start_y - 4..=start_y {
                    if self.game_state.letter_queue[x][y] == ' ' {
                        compression_possible = false;
                        break;
                    }
                }
                if compression_possible {
                    self.game_state.letter_queue[x][start_y] = self.game_state.letter_queue[x][start_y - 1].to_ascii_uppercase();
                    for y in start_y - 4..start_y {
                        self.game_state.letter_queue[x][y] = ' ';
                    }
                }
            }
        }

        // clear out letters that have reached the bottom and have counts
        let mut chars_processed = String::new();
        for x in 0..NUM_LETTERS {
            let letter = if self.game_state.letter_compression_unlocked {
                idx_to_upper_letter(x)
            } else {
                idx_to_letter(x)
            };
            if self.game_state.letter_queue[x][LETTER_QUEUE_HEIGHT - 1] == letter {
                let count = self.game_state.counts.entry(letter.to_string()).or_insert(0);
                if *count > 0 {
                    *count -= 1;
                    chars_processed.push(letter);
                    self.game_state.letter_queue[x][LETTER_QUEUE_HEIGHT - 1] = ' ';
                }
            }
        }
        let money_change = self.calc_money_change(&chars_processed, &chars_processed);
        self.increment_money(money_change);

        // move all letters down one row
        for y in (1..LETTER_QUEUE_HEIGHT).rev() {
            for x in 0..NUM_LETTERS {
                if self.game_state.letter_queue[x][y] == ' ' {
                    self.game_state.letter_queue[x][y] = self.game_state.letter_queue[x][y - 1];
                    self.game_state.letter_queue[x][y - 1] = ' ';
                }
            }
        }

        if now - self.last_spawn_time >= std::time::Duration::from_millis(50) {
            // spawn a new letter at the top
            let x = rand::random::<u32>() % NUM_LETTERS as u32;
            let idx = x as usize;

            if self.game_state.letter_queue[idx][0] != ' ' {
                // if the top row is already occupied, don't spawn a new letter
                return;
            }

            self.game_state.letter_queue[idx][0] = (b'a' + idx as u8) as char;
            self.last_spawn_time = now;
        }
    }

    pub fn update(&mut self, now: std::time::Instant) {
        self.update_handle_inputs();

        if now - self.last_update_time >= std::time::Duration::from_millis(200) {
            self.last_update_time = now;
            if self.game_state.automation_unlocked {
                self.letter_queue_update(now);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::upgrade::UpgradeId;

    #[test]
    fn game_state_json_roundtrip() {
        let mut state = Game::default_state();
        state.money = BigDollar::from(42);
        state.current_line = 3;
        state.upgrade_levels.insert(UpgradeId::UnlockStreak, 1);
        state.automation_unlocked = true;

        let json = serde_json::to_string_pretty(&state).unwrap();
        let loaded: GameState = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.money, state.money);
        assert_eq!(loaded.current_line, state.current_line);
        assert_eq!(loaded.upgrade_levels, state.upgrade_levels);
        assert_eq!(loaded.automation_unlocked, state.automation_unlocked);
        assert_eq!(loaded.current_text.len(), GameState::default_current_text().len());
    }

    #[test]
    fn game_state_partial_json_uses_defaults() {
        let json = r#"{"money": 0.042, "current_line": 3}"#;
        let loaded: GameState = serde_json::from_str(json).unwrap();

        assert_eq!(loaded.money, BigDollar::from(42));
        assert_eq!(loaded.current_line, 3);
        assert_eq!(loaded.automation_unlocked, false);
        assert_eq!(loaded.capital_letter_bonus_unlocked, false);
        assert_eq!(loaded.letter_compression_unlocked, false);
        assert_eq!(loaded.base_letter_value, BigDollar::from(0.003));
        assert_eq!(loaded.trust_level, 0);
        assert_eq!(loaded.current_pane, WindowPanes::TextPane);
        assert_eq!(loaded.current_text.len(), GameState::default_current_text().len());
    }
}
