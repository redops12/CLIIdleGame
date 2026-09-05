use std::collections::HashMap;
use std::io;
use std::path::Path;

use crossbeam_channel::Receiver;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use serde::{Deserialize, Serialize};

use crate::auto_queue::{AutoQueue};
use crate::big_num::BigDollar;
use crate::test_mode::{AppModule, TestMode};
use crate::upgrade::{get_upgrades, UpgradeId};
use crate::text_sources::{TextSource, get_lines_from_source};

pub const MAX_TRUST_LEVEL: i32 = 100;
pub const TRUST_SCALE: f64 = 1.15;
pub const SECOND_POLL_WINDOW: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowPanes {
    TextPane,
    UpgradePane,
    AutoPane,
    GraphPane,
}

pub const UPGRADE_KEYS: [char; 10] = ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g'];

pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PaneRects {
    pub text: ratatui::layout::Rect,
    pub upgrade: ratatui::layout::Rect,
    pub auto_keys: Option<ratatui::layout::Rect>,
    pub graph: Option<ratatui::layout::Rect>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameState {
    // Text variables
    pub current_text: TextSource,
    pub current_line: usize,
    pub typed: String,

    #[serde(flatten)]
    pub auto_queue: AutoQueue,

    // displayed stats
    pub money: BigDollar,

    // secret stats
    pub high_water_money: BigDollar,
    pub total_money_earned: BigDollar,
    pub second_profit_buckets: Vec<BigDollar>, // tracks income from each second for the last SECOND_POLL_WINDOW seconds
    pub second_profit_bucket_head: usize,

    // progression variables
    pub upgrade_levels: HashMap<UpgradeId, usize>,
    pub trust_level: i32,
    pub base_letter_value: BigDollar,
    pub streaks_unlocked: bool,
    pub capital_letter_bonus_unlocked: bool,
    pub automation_unlocked: bool,
    pub seniority_level: u8,
    pub graphs_unlocked: bool,
    pub disable_penalty: bool,

    // top left is 0, 0
    // down is increasing y, right is increasing x
    pub window_x: u16,
    pub window_y: u16,
    pub current_pane: WindowPanes,
    previous_window_x: u16,
    previous_window_y: u16,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            typed: String::new(),
            current_text: TextSource::Intro,
            current_line: 0,
            auto_queue: AutoQueue::default(),
            money: BigDollar::from(0),
            high_water_money: BigDollar::from(0),
            total_money_earned: BigDollar::from(0),
            second_profit_buckets: vec![BigDollar::from(0); SECOND_POLL_WINDOW + 1],
            second_profit_bucket_head: 0,
            upgrade_levels: HashMap::new(),
            trust_level: 0,
            base_letter_value: BigDollar::from(0.003),
            streaks_unlocked: false,
            capital_letter_bonus_unlocked: false,
            automation_unlocked: false,
            seniority_level: 0,
            graphs_unlocked: false,
            disable_penalty: false,
            window_x: 1,
            window_y: 0,
            current_pane: WindowPanes::TextPane,
            previous_window_x: 1,
            previous_window_y: 0,
        }
    }
}

pub struct Game {
    input_rx: Receiver<InputEvent>,
    pub should_quit: bool,
    pub pane_rects: PaneRects,

    pub last_update_time: std::time::Instant,
    pub last_profit_time: std::time::Instant,
    pub game_state: GameState,
    pub test_mode: Option<TestMode>,
}

impl Game {
    #[allow(dead_code)]
    pub fn new(input_rx: Receiver<InputEvent>) -> Self {
        Self::from_state(input_rx, Self::default_state())
    }

    pub fn new_with_test_mode(input_rx: Receiver<InputEvent>, test_mode: Option<TestMode>) -> Self {
        Self::from_state_with_test_mode(input_rx, Self::default_state(), test_mode)
    }

    #[allow(dead_code)]
    pub fn from_state(input_rx: Receiver<InputEvent>, game_state: GameState) -> Self {
        Self::from_state_with_test_mode(input_rx, game_state, None)
    }

    pub fn from_state_with_test_mode(
        input_rx: Receiver<InputEvent>,
        game_state: GameState,
        test_mode: Option<TestMode>,
    ) -> Self {
        let mut game = Self {
            input_rx,
            should_quit: false,
            pane_rects: PaneRects::default(),
            last_update_time: std::time::Instant::now(),
            last_profit_time: std::time::Instant::now(),
            game_state,
            test_mode,
        };
        game.recalculate_current_pane();
        game
    }

    pub fn is_module_active(&self, module: AppModule) -> bool {
        match &self.test_mode {
            Some(tm) => tm.is_active(module),
            None => match module {
                AppModule::Text => true,
                AppModule::Upgrade => true,
                AppModule::AutoQueue => self.game_state.automation_unlocked,
                AppModule::Graph => self.game_state.graphs_unlocked,
                AppModule::MoneyBar => true,
            },
        }
    }

    pub fn default_state() -> GameState {
        GameState::default()
    }

    pub fn load_state(path: &Path) -> io::Result<GameState> {
        let data = std::fs::read_to_string(path)?;
        let value: serde_json::Value =
            serde_json::from_str(&data).map_err(io::Error::other)?;
        serde_json::from_value(value).map_err(io::Error::other)
    }

    pub fn save_state(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.game_state).map_err(io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn increment_money(&mut self, amount: BigDollar) {
        self.game_state.money += amount;
        self.game_state.high_water_money = self.game_state.high_water_money.max(self.game_state.money);
        self.game_state.total_money_earned += amount;
        self.game_state.second_profit_buckets[self.game_state.second_profit_bucket_head] += amount;
    }

    pub fn decrement_money(&mut self, amount: BigDollar) {
        self.game_state.money -= amount;
        self.game_state.second_profit_buckets[self.game_state.second_profit_bucket_head] -= amount;
    }

    pub fn get_text_line(&self, offset: Option<usize>) -> &str {
        get_lines_from_source(
            self.game_state.current_text,
            self.game_state.current_line + offset.unwrap_or(0),
        )
    }

    pub fn calc_money_change(&self, typed: &str, reference: &str) -> BigDollar {
        let mut money_change: BigDollar = BigDollar::from(0);
        let typed_chars: Vec<char> = typed.chars().collect();
        let ref_chars: Vec<char> = if self.game_state.capital_letter_bonus_unlocked {
            reference.chars().collect()
        } else {
            reference.to_lowercase().chars().collect()
        };
        let len = typed_chars.len().max(ref_chars.len());

        for i in 0..len {
            match (typed_chars.get(i), ref_chars.get(i)) {
                (Some(&c), Some(&t)) if c == t => {
                    let mult = if c.is_ascii_uppercase() && self.game_state.capital_letter_bonus_unlocked {
                        10.0
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

                    money_change -= self.game_state.base_letter_value * 2.0 * TRUST_SCALE.powi(self.game_state.trust_level);
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
        if let Some(ref tm) = self.test_mode {
            if tm.is_single_active(AppModule::AutoQueue) {
                self.game_state.window_x = 0;
                self.game_state.window_y = 0;
                self.game_state.current_pane = WindowPanes::AutoPane;
                return;
            }
            if tm.is_single_active(AppModule::Text) {
                self.game_state.window_x = 0;
                self.game_state.window_y = 0;
                self.game_state.current_pane = WindowPanes::TextPane;
                return;
            }
            if tm.is_single_active(AppModule::Upgrade) {
                self.game_state.window_x = 0;
                self.game_state.window_y = 0;
                self.game_state.current_pane = WindowPanes::UpgradePane;
                return;
            }
            if tm.is_single_active(AppModule::Graph) {
                self.game_state.window_x = 0;
                self.game_state.window_y = 0;
                self.game_state.current_pane = WindowPanes::GraphPane;
                return;
            }
        }

        let auto_active = self.is_module_active(AppModule::AutoQueue);
        match auto_active {
            false => {
                self.game_state.window_x = self.game_state.window_x.min(0);
                self.game_state.current_pane = match (self.game_state.window_x, self.game_state.window_y) {
                    (0, 0) => WindowPanes::TextPane,
                    (0, 1) => WindowPanes::UpgradePane,
                    _ => WindowPanes::TextPane,
                };
            }
            true => {
                self.game_state.window_x = self.game_state.window_x.min(1);
                self.game_state.current_pane = match (self.game_state.window_x, self.game_state.window_y) {
                    (0, 0) => WindowPanes::TextPane,
                    (0, 1) => WindowPanes::UpgradePane,
                    (1, _) => WindowPanes::AutoPane,
                    _ => WindowPanes::TextPane,
                };
            }
        }
    }

    fn toggle_upgrade_pane(&mut self) {
        if !self.is_module_active(AppModule::Upgrade) {
            return;
        }
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
                self.game_state.window_y = 1;
            }
        }
        self.recalculate_current_pane();
    }

    pub fn handle_mouse_click(&mut self, column: u16, row: u16) {
        use ratatui::layout::Position;

        let pos = Position { x: column, y: row };
        if self.is_module_active(AppModule::Text) && self.pane_rects.text.contains(pos) {
            self.game_state.window_x = 0;
            self.game_state.window_y = 0;
            self.game_state.current_pane = WindowPanes::TextPane;
        } else if self.is_module_active(AppModule::Upgrade) && self.pane_rects.upgrade.contains(pos) {
            self.game_state.window_x = 0;
            self.game_state.window_y = 1;
            self.game_state.current_pane = WindowPanes::UpgradePane;
        } else if self.is_module_active(AppModule::AutoQueue) && self.pane_rects.auto_keys.is_some_and(|rect| rect.contains(pos)) {
            self.game_state.window_x = 1;
            self.game_state.window_y = 0;
            self.game_state.current_pane = WindowPanes::AutoPane;
        } else if self.is_module_active(AppModule::Graph) && self.pane_rects.graph.is_some_and(|rect| rect.contains(pos)) {
            self.game_state.current_pane = WindowPanes::GraphPane;
        }
    }

    fn text_pane_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.game_state.typed.push(c);
            }
            KeyCode::Enter => {
                let current_line: &str = self.get_text_line(None);
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
        while let Ok(input) = self.input_rx.try_recv() {
            match input {
                InputEvent::Mouse(mouse) => {
                    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                        self.handle_mouse_click(mouse.column, mouse.row);
                    }
                }
                InputEvent::Key(key) => {
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
                        WindowPanes::AutoPane => self.game_state.auto_queue.handle_input(key.code),
                        _ => {}
                    }
                }
            }
        }
    }

    fn letter_queue_update(&mut self) {
        let chars_processed = self.game_state.auto_queue.update();
        let money_change = self.calc_money_change(&chars_processed, &chars_processed);
        self.increment_money(money_change);
    }

    fn handle_tracking(&mut self, now: std::time::Instant) {
        if now - self.last_profit_time >= std::time::Duration::from_secs(1) {
            self.last_profit_time = now;
            self.game_state.second_profit_bucket_head = (self.game_state.second_profit_bucket_head + 1) % (SECOND_POLL_WINDOW + 1);
            self.game_state.second_profit_buckets[self.game_state.second_profit_bucket_head] = BigDollar::from(0);
        }
    }

    pub fn update(&mut self, now: std::time::Instant) {
        self.update_handle_inputs();
        self.handle_tracking(now);

        if self.is_module_active(AppModule::AutoQueue) {
            self.letter_queue_update();
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
    }

    #[test]
    fn game_state_partial_json_uses_defaults() {
        let json = r#"{"money": 0.042, "current_line": 3}"#;
        let loaded: GameState = serde_json::from_str(json).unwrap();

        assert_eq!(loaded.money, BigDollar::from(42));
        assert_eq!(loaded.current_line, 3);
        assert_eq!(loaded.automation_unlocked, false);
        assert_eq!(loaded.capital_letter_bonus_unlocked, false);
        assert_eq!(loaded.auto_queue.letter_compression_unlocked, false);
        assert_eq!(loaded.base_letter_value, BigDollar::from(0.003));
        assert_eq!(loaded.trust_level, 0);
        assert_eq!(loaded.current_pane, WindowPanes::TextPane);
    }

    #[test]
    fn test_mode_auto_queue_boot() {
        let (_tx, rx) = crossbeam_channel::unbounded();
        let game = Game::new_with_test_mode(rx, Some(TestMode::auto_queue()));

        assert!(game.is_module_active(AppModule::AutoQueue));
        assert!(!game.is_module_active(AppModule::Text));
        assert!(!game.is_module_active(AppModule::Upgrade));
        assert!(!game.is_module_active(AppModule::MoneyBar));
        assert_eq!(game.game_state.current_pane, WindowPanes::AutoPane);
    }
}
