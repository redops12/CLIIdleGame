use std::collections::HashMap;

use crossbeam_channel::Receiver;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use rand;

use crate::BigNum::BigDollar;
use crate::upgrade::{get_upgrades, UpgradeId};

const WASTELAND: &str = include_str!("../assets/wasteland.txt");
pub const NUM_LETTERS: usize = 26;
pub const LETTER_QUEUE_HEIGHT: usize = 10;

pub enum WindowPanes {
    HelpPane,
    TextPane,
    UpgradePane,
    AutoPane,
}

pub const UPGRADE_KEYS: [char; 10] = ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g'];

pub struct GameState {
    // currently typed string
    pub typed: String,

    // Text variables
    pub current_text: Vec<&'static str>,
    pub current_line: usize,

    // auto info
    pub counts: HashMap<String, u32>,

    // displayed stats
    pub money: BigDollar,

    // secret stats
    pub total_money_earned: BigDollar,

    // progression variables
    pub upgrade_levels: HashMap<UpgradeId, usize>,
    pub trust_level: i32,
    pub base_letter_value: BigDollar,
    pub streaks_unlocked: bool,
    pub disable_penalty: bool,

    // top left is 0, 0
    // down is increasing y, right is increasing x
    pub window_x: u16,
    pub window_y: u16,
    pub current_pane: WindowPanes,
    previous_window_x: u16,
    previous_window_y: u16,


    pub letter_queue: [[bool; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
}

pub struct Game {
    input_rx: Receiver<KeyEvent>,
    pub should_quit: bool,

    pub last_spawn_time: std::time::Instant,
    pub game_state: GameState,
}

impl Game {
    pub fn new(input_rx: Receiver<KeyEvent>) -> Self {
        Self {
            input_rx,
            should_quit: false,
            last_spawn_time: std::time::Instant::now(),
            game_state: GameState {
                total_money_earned: BigDollar::from(0),
                upgrade_levels: HashMap::new(),
                base_letter_value: BigDollar::from(1),
                trust_level: 0,
                streaks_unlocked: false,
                disable_penalty: false,
                counts: HashMap::new(),
                typed: String::new(),
                current_text: WASTELAND.split('\n').collect(),
                current_line: 0,
                money: BigDollar::from(0),
                window_x: 1,
                window_y: 1,
                current_pane: WindowPanes::TextPane,
                previous_window_x: 1,
                previous_window_y: 1,
                letter_queue: [[false; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
            }
        }
    }

    pub fn increment_money(&mut self, amount: BigDollar) {
        self.game_state.money += amount;
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
                    money_change += self.game_state.base_letter_value * 2.0_f64.powi(self.game_state.trust_level);
                }
                // Wrong char, missing char, or extra typed char
                _ => {
                    if self.game_state.disable_penalty {
                        continue;
                    }

                    money_change -= BigDollar::from(500);
                }
            }
        }

        money_change
    }


    fn recalculate_current_pane(&mut self) {
        self.game_state.current_pane = match (self.game_state.window_x, self.game_state.window_y) {
            (0, _) => WindowPanes::HelpPane,
            (1, 0) => WindowPanes::UpgradePane,
            (1, 1) => WindowPanes::TextPane,
            (2, _) => WindowPanes::AutoPane,
            _ => WindowPanes::TextPane,
        };
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
                self.game_state.window_x = 1;
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
                KeyCode::Right => self.game_state.window_x = self.game_state.window_x.saturating_add(1).min(2),
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
        // clear out letters that have reached the bottom and have counts
        let mut chars_processed = String::new();
        for x in 0..NUM_LETTERS {
            if self.game_state.letter_queue[x][LETTER_QUEUE_HEIGHT - 1] {
                let letter = (b'a' + x as u8) as char;
                let count = self.game_state.counts.entry(letter.to_string()).or_insert(0);
                if *count > 0 {
                    *count -= 1;
                    chars_processed.push(letter);
                    self.game_state.letter_queue[x][LETTER_QUEUE_HEIGHT - 1] = false;
                }
            }
        }
        let money_change = self.calc_money_change(&chars_processed, &chars_processed);
        self.increment_money(money_change);

        // move all letters down one row
        for y in (1..LETTER_QUEUE_HEIGHT).rev() {
            for x in 0..NUM_LETTERS {
                if !self.game_state.letter_queue[x][y] {
                    self.game_state.letter_queue[x][y] = self.game_state.letter_queue[x][y - 1];
                    self.game_state.letter_queue[x][y - 1] = false;
                }
            }
        }

        if now - self.last_spawn_time >= std::time::Duration::from_millis(50) {
            // spawn a new letter at the top
            let x = rand::random::<u32>() % NUM_LETTERS as u32;
            let idx = x as usize;

            if self.game_state.letter_queue[idx][0] {
                // if the top row is already occupied, don't spawn a new letter
                return;
            }

            self.game_state.letter_queue[idx][0] = true;
            self.last_spawn_time = now;
        }
    }

    pub fn update(&mut self, now: std::time::Instant) {
        self.update_handle_inputs();
        self.letter_queue_update(now);
    }
}
