use std::collections::HashMap;

use crossterm::event::KeyCode;

use crate::BigNum::BigDollar;
use crate::upgrade::{
    GameValue, Upgrade, button_to_upgrade, game_value_start, initial_game_values, initial_upgrade_costs, upgrade_buttons, upgrade_multiplier, upgrade_starting_cost, upgrade_value_change, upgrade_value_key,
};

const WASTELAND: &str = include_str!("../assets/wasteland.txt");

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
        match key {
            KeyCode::Up => {self.window_y = self.window_y.saturating_sub(1);}
            KeyCode::Down => {self.window_y = self.window_y.saturating_add(1).min(1);}
            KeyCode::Left => {self.window_x = self.window_x.saturating_sub(1);}
            KeyCode::Right => {self.window_x = self.window_x.saturating_add(1).min(2);}
            _ => {}
        }
        self.recalculate_current_pane();
        match self.current_pane {
            WindowPanes::TextPane => self.text_pane_input(key),
            WindowPanes::UpgradePane => self.upgrade_pane_input(key),
            _ => {}
        }
    }
}
