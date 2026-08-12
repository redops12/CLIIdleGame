use std::collections::HashMap;

use crossterm::event::KeyCode;

use rand;

use crate::BigNum::BigDollar;
use crate::upgrade::{
    GameValue, Upgrade, button_to_upgrade, game_value_start, initial_game_values, initial_upgrade_costs, upgrade_buttons, upgrade_multiplier, upgrade_starting_cost, upgrade_value_change, upgrade_value_key,
};

const WASTELAND: &str = include_str!("../assets/wasteland.txt");
pub const NUM_LETTERS: usize = 26;
pub const LETTER_QUEUE_HEIGHT: usize = 20;

pub enum WindowPanes {
    HelpPane,
    TextPane,
    UpgradePane,
    AutoPane,
}

pub struct Game {
    pub counts: HashMap<String, u32>,

    // currently typed string
    pub typed: String,

    // Text variables
    pub current_text: Vec<&'static str>,
    pub current_line: usize,

    // displayed stats
    pub money: BigDollar,
    pub game_values: HashMap<GameValue, BigDollar>,
    pub upgrade_costs: HashMap<Upgrade, BigDollar>,

    // top left is 0, 0
    // down is increasing y, right is increasing x
    pub window_x: u16,
    pub window_y: u16,
    pub current_pane: WindowPanes,
    previous_window_x: u16,
    previous_window_y: u16,

    pub letter_queue: [[bool; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
    pub last_spawn_time: std::time::Instant,
}

impl Game {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
            typed: String::new(),
            current_text: WASTELAND.split('\n').collect(),
            current_line: 0,
            money: BigDollar::from(0),
            game_values: initial_game_values(),
            upgrade_costs: initial_upgrade_costs(),
            window_x: 1,
            window_y: 1,
            current_pane: WindowPanes::TextPane,
            previous_window_x: 1,
            previous_window_y: 1,
            letter_queue: [[false; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
            last_spawn_time: std::time::Instant::now(),
        }
    }

    pub fn increment(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    pub fn calc_money_change(&self, typed: &str, reference: &str) -> BigDollar {
        let mut money_change: BigDollar = BigDollar::from(0);
        let typed_chars: Vec<char> = typed.chars().collect();
        let ref_chars: Vec<char> = reference.chars().collect();
        let len = typed_chars.len().max(ref_chars.len());

        for i in 0..len {
            match (typed_chars.get(i), ref_chars.get(i)) {
                (Some(&c), Some(&t)) if c == t => {
                    money_change += *self.game_values.get(&GameValue::Increment).unwrap();
                }
                // Wrong char, missing char, or extra typed char
                _ => {
                    money_change -= *self.game_values.get(&GameValue::Penalty).unwrap();
                }
            }
        }

        money_change
    }

    pub fn buy_upgrade(&mut self, kind: Upgrade) {
        let upgrade_cost = self.upgrade_costs.get(&kind).unwrap();
        if self.money >= *upgrade_cost {
            self.money -= *upgrade_cost;
            let upgrade = upgrade_value_key(kind);
            *self
                .game_values
                .entry(upgrade)
                .or_insert(game_value_start(upgrade)) += upgrade_value_change(kind);
            *self
                .upgrade_costs
                .entry(kind)
                .or_insert(upgrade_starting_cost(kind)) *= upgrade_multiplier(kind);
            logging::info(&format!(
                "Bought upgrade {:?}: new value for {:?} is {}, next cost is {}",
                kind,
                upgrade,
                self.game_values.get(&upgrade).unwrap(),
                self.upgrade_costs.get(&kind).unwrap()
            ));
        } else {
            logging::info(&format!(
                "Not enough money to buy upgrade {:?}: cost is {}, current money is {}",
                kind, upgrade_cost, self.money
            ));
        }
    }

    fn recalculate_current_pane(&mut self) {
        self.current_pane = match (self.window_x, self.window_y) {
            (0, _) => WindowPanes::HelpPane,
            (1, 0) => WindowPanes::UpgradePane,
            (1, 1) => WindowPanes::TextPane,
            (2, _) => WindowPanes::AutoPane,
            _ => WindowPanes::TextPane,
        };
    }

    fn toggle_upgrade_pane(&mut self) {
        self.recalculate_current_pane();
        match self.current_pane {
            WindowPanes::UpgradePane => {
                self.window_x = self.previous_window_x;
                self.window_y = self.previous_window_y;
            }
            _ => {
                self.previous_window_x = self.window_x;
                self.previous_window_y = self.window_y;
                self.window_x = 1;
                self.window_y = 0;
            }
        }
        self.recalculate_current_pane();
    }

    fn text_pane_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.typed.push(c);
            }
            KeyCode::Enter => {
                let current_line: &'static str = self.current_text[self.current_line];
                let typed_chars: Vec<char> = self.typed.chars().collect();
                let ref_chars: Vec<char> = current_line.chars().collect();
                let money_change = self.calc_money_change(&self.typed, current_line);

                logging::debug(&format!(
                    "Scored line {}: money_change={money_change} (typed={} chars, ref={} chars)",
                    self.current_line,
                    typed_chars.len(),
                    ref_chars.len()
                ));
                self.money += money_change;
                self.typed = String::new();
                self.current_line += 1;
            }
            KeyCode::Backspace => {
                self.typed.pop();
            }
            _ => {}
        }
    }

    pub fn upgrade_pane_input(&mut self, key: KeyCode) {
        button_to_upgrade(key).map(|upgrade| self.buy_upgrade(upgrade));
    }

    pub fn input(&mut self, key: KeyCode) {
        logging::debug(&format!("Input received: {key}"));
        if key == KeyCode::Tab {
            self.toggle_upgrade_pane();
            return;
        }
        match key {
            KeyCode::Up => {
                self.window_y = self.window_y.saturating_sub(1);
            }
            KeyCode::Down => {
                self.window_y = self.window_y.saturating_add(1).min(1);
            }
            KeyCode::Left => {
                self.window_x = self.window_x.saturating_sub(1);
            }
            KeyCode::Right => {
                self.window_x = self.window_x.saturating_add(1).min(2);
            }
            _ => {}
        }
        self.recalculate_current_pane();
        match self.current_pane {
            WindowPanes::TextPane => self.text_pane_input(key),
            WindowPanes::UpgradePane => self.upgrade_pane_input(key),
            WindowPanes::AutoPane => {
                if let KeyCode::Char(c) = key {
                    self.increment(&c.to_string());
                }
            }
            _ => {}
        }
    }

    fn letter_queue_update(&mut self, now: std::time::Instant) {
        // clear out letters that have reached the bottom and have counts
        let mut money_change: BigDollar = BigDollar::from(0);
        for x in 0..NUM_LETTERS {
            if self.letter_queue[x][19] {
                let letter = (b'a' + x as u8) as char;
                let count = self.counts.entry(letter.to_string()).or_insert(0);
                if *count > 0 {
                    *count -= 1;
                    money_change += *self.game_values.get(&GameValue::Increment).unwrap();
                    self.letter_queue[x][19] = false;
                }
            }
        }
        self.money += money_change;

        // move all letters down one row
        for y in (1..LETTER_QUEUE_HEIGHT).rev() {
            for x in 0..NUM_LETTERS {
                if !self.letter_queue[x][y] {
                    self.letter_queue[x][y] = self.letter_queue[x][y - 1];
                    self.letter_queue[x][y - 1] = false;
                }
            }
        }

        if now - self.last_spawn_time >= std::time::Duration::from_millis(50) {
            // spawn a new letter at the top
            let x = rand::random::<u32>() % NUM_LETTERS as u32;
            let idx = x as usize;

            if self.letter_queue[idx][0] {
                // if the top row is already occupied, don't spawn a new letter
                return;
            }

            self.letter_queue[idx][0] = true;
            self.last_spawn_time = now;
        }
    }

    pub fn update(&mut self, now: std::time::Instant) {
        self.letter_queue_update(now);
    }
}
