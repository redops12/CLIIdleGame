use std::collections::HashMap;
use std::io;
use std::path::Path;

use crossbeam_channel::Receiver;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use serde::{Deserialize, Serialize};

use crate::auto_queue::AutoQueue;
use crate::auto_typer_selector::AutoTyperSelector;
use crate::big_num::BigDollar;
use crate::pane_module::{ModuleId, PaneModule};
use crate::test_mode::TestMode;
use crate::text_pane::TextPane;
use crate::upgrade::{get_upgrades, UpgradeId};
use crate::upgrade_pane::UpgradePane;

pub const MAX_TRUST_LEVEL: i32 = 100;
pub const TRUST_SCALE: f64 = 1.15;
pub const NUM_POLLS: usize = 30;

pub const UPGRADE_KEYS: [char; 10] = ['q', 'w', 'e', 'r', 't', 'a', 's', 'd', 'f', 'g'];

pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PaneRects {
    pub text: ratatui::layout::Rect,
    pub upgrade: ratatui::layout::Rect,
    pub graph: Option<ratatui::layout::Rect>,
    pub auto_keys: Option<ratatui::layout::Rect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GameState {
    #[serde(flatten)]
    pub text_pane: TextPane,

    #[serde(flatten)]
    pub auto_queue: AutoQueue,

    // displayed stats
    pub money: BigDollar,

    // secret stats
    pub high_water_money: BigDollar,
    pub total_money_earned: BigDollar,
    pub five_second_profit_buckets: Vec<BigDollar>,
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

    pub window_x: u16,
    pub window_y: u16,
    pub current_pane: ModuleId,
    previous_window_x: u16,
    previous_window_y: u16,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            text_pane: TextPane::default(),
            auto_queue: AutoQueue::default(),
            money: BigDollar::from(30.0),
            high_water_money: BigDollar::from(0),
            total_money_earned: BigDollar::from(30.0),
            five_second_profit_buckets: vec![BigDollar::from(0); NUM_POLLS + 1],
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
            current_pane: ModuleId::Text,
            previous_window_x: 1,
            previous_window_y: 0,
        }
    }
}

pub struct Game {
    pub game_state: GameState,

    input_rx: Receiver<InputEvent>,
    pub should_quit: bool,
    pub pane_rects: PaneRects,

    pub last_profit_time: std::time::Instant,
    pub test_mode: Option<TestMode>,
    pub floating_pane_queue: Vec<ModuleId>,
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
            game_state,
            input_rx,
            should_quit: false,
            pane_rects: PaneRects::default(),
            last_profit_time: std::time::Instant::now(),
            test_mode,
            floating_pane_queue: Vec::new(),
        };
        game.recalculate_current_pane();
        game
    }

    pub fn is_module_active(&self, module: ModuleId) -> bool {
        let test_mode = self.test_mode.as_ref();
        if let Some(tm) = test_mode {
            tm.is_active(module)
        } else {
            match module {
                ModuleId::Text => true,
                ModuleId::Upgrade => true,
                ModuleId::AutoQueue => self.game_state.automation_unlocked,
                ModuleId::Graph => self.game_state.graphs_unlocked,
                ModuleId::MoneyBar => true,
                ModuleId::AutoTyperSelector => true,
            }
        }
    }

    pub fn get_floating_pane(&self) -> Option<ModuleId> {
        let test_mode = self.test_mode.as_ref();
        if let Some(tm) = test_mode {
            return tm.single_active_module()
        }
        self.floating_pane_queue.first().copied()
    }

    pub fn add_floating_pane(&mut self, module: ModuleId) {
        if !self.floating_pane_queue.contains(&module) {
            self.floating_pane_queue.push(module);
        }
    }

    pub fn remove_floating_pane(&mut self, module: ModuleId) {
        self.floating_pane_queue.retain(|&m| m != module);
        self.recalculate_current_pane();
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
        self.game_state.five_second_profit_buckets[self.game_state.second_profit_bucket_head] += amount;
    }

    pub fn decrement_money(&mut self, amount: BigDollar) {
        self.game_state.money -= amount;
    }

    pub fn get_text_line(&self, offset: Option<usize>) -> &str {
        self.game_state.text_pane.get_line(offset)
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
            match tm.single_active_module() {
                None => {}
                Some(module) =>
                {
                    self.game_state.current_pane = module;
                    return;
                }
            }
        }

        if let Some(module) = self.get_floating_pane() {
            self.game_state.window_x = self.game_state.previous_window_x;
            self.game_state.window_y = self.game_state.previous_window_y;
            self.game_state.current_pane = module;
            return;
        }

        let auto_active = self.is_module_active(ModuleId::AutoQueue);
        match auto_active {
            false => {
                self.game_state.window_x = self.game_state.window_x.min(0);
                self.game_state.current_pane = match (self.game_state.window_x, self.game_state.window_y) {
                    (0, 0) => ModuleId::Text,
                    (0, 1) => ModuleId::Upgrade,
                    _ => ModuleId::Text,
                };
            }
            true => {
                self.game_state.window_x = self.game_state.window_x.min(1);
                self.game_state.current_pane = match (self.game_state.window_x, self.game_state.window_y) {
                    (0, 0) => ModuleId::Text,
                    (0, 1) => ModuleId::Upgrade,
                    (1, _) => ModuleId::AutoQueue,
                    _ => ModuleId::Text,
                };
            }
        }
    }

    fn toggle_upgrade_pane(&mut self) {
        if !self.is_module_active(ModuleId::Upgrade) {
            return;
        }
        self.recalculate_current_pane();
        match self.game_state.current_pane {
            ModuleId::Upgrade => {
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
        if let Some(module) = self.get_floating_pane() {
            self.game_state.current_pane = module;
            return;
        }

        let pos = Position { x: column, y: row };
        if self.is_module_active(ModuleId::Text) && self.pane_rects.text.contains(pos) {
            self.game_state.window_x = 0;
            self.game_state.window_y = 0;
            self.game_state.current_pane = ModuleId::Text;
        } else if self.is_module_active(ModuleId::Upgrade) && self.pane_rects.upgrade.contains(pos) {
            self.game_state.window_x = 0;
            self.game_state.window_y = 1;
            self.game_state.current_pane = ModuleId::Upgrade;
        } else if self.is_module_active(ModuleId::AutoQueue) && self.pane_rects.auto_keys.is_some_and(|rect| rect.contains(pos)) {
            self.game_state.window_x = 1;
            self.game_state.window_y = 0;
            self.game_state.current_pane = ModuleId::AutoQueue;
        } else if self.is_module_active(ModuleId::Graph) && self.pane_rects.graph.is_some_and(|rect| rect.contains(pos)) {
            self.game_state.current_pane = ModuleId::Graph;
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
            (upgrade.on_buy)(self);
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

    fn dispatch_pane_input(&mut self, key: KeyCode) {
        match self.game_state.current_pane {
            ModuleId::Text if self.is_module_active(ModuleId::Text) => {
                <TextPane as PaneModule>::handle_input(self, key);
            }
            ModuleId::Upgrade if self.is_module_active(ModuleId::Upgrade) => {
                <UpgradePane as PaneModule>::handle_input(self, key);
            }
            ModuleId::AutoQueue if self.is_module_active(ModuleId::AutoQueue) => {
                <AutoQueue as PaneModule>::handle_input(self, key);
            }
            ModuleId::AutoTyperSelector if self.is_module_active(ModuleId::AutoTyperSelector) => {
                <AutoTyperSelector as PaneModule>::handle_input(self, key);
            }
            _ => {}
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
                        KeyCode::Esc => {
                            if let Some(module) = self.get_floating_pane() {
                                self.remove_floating_pane(module);
                            } else {
                                self.should_quit = true;
                            }
                        }
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
                    self.dispatch_pane_input(key.code);
                }
            }
        }
    }

    fn handle_tracking(&mut self, now: std::time::Instant) {
        if now - self.last_profit_time >= std::time::Duration::from_secs(5) {
            self.last_profit_time = now;
            self.game_state.second_profit_bucket_head = (self.game_state.second_profit_bucket_head + 1) % (NUM_POLLS + 1);
            self.game_state.five_second_profit_buckets[self.game_state.second_profit_bucket_head] = BigDollar::from(0);
        }
    }

    pub fn update(&mut self, now: std::time::Instant) {
        self.update_handle_inputs();
        self.handle_tracking(now);

        if self.is_module_active(ModuleId::AutoQueue) {
            <AutoQueue as PaneModule>::update(self);
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
        state.text_pane.current_line = 3;
        state.upgrade_levels.insert(UpgradeId::UnlockStreak, 1);
        state.automation_unlocked = true;

        let json = serde_json::to_string_pretty(&state).unwrap();
        let loaded: GameState = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.money, state.money);
        assert_eq!(loaded.text_pane.current_line, state.text_pane.current_line);
        assert_eq!(loaded.upgrade_levels, state.upgrade_levels);
        assert_eq!(loaded.automation_unlocked, state.automation_unlocked);
    }

    #[test]
    fn game_state_partial_json_uses_defaults() {
        let json = r#"{"money": 0.042, "current_line": 3}"#;
        let loaded: GameState = serde_json::from_str(json).unwrap();

        assert_eq!(loaded.money, BigDollar::from(42));
        assert_eq!(loaded.text_pane.current_line, 3);
        assert_eq!(loaded.automation_unlocked, false);
        assert_eq!(loaded.capital_letter_bonus_unlocked, false);
        assert_eq!(loaded.auto_queue.letter_compression_unlocked, false);
        assert_eq!(loaded.base_letter_value, BigDollar::from(0.003));
        assert_eq!(loaded.trust_level, 0);
        assert_eq!(loaded.current_pane, ModuleId::Text);
    }

    #[test]
    fn test_mode_auto_queue_boot() {
        let (_tx, rx) = crossbeam_channel::unbounded();
        let game = Game::new_with_test_mode(rx, Some(TestMode::auto_queue()));

        assert!(game.is_module_active(ModuleId::AutoQueue));
        assert!(!game.is_module_active(ModuleId::Text));
        assert!(!game.is_module_active(ModuleId::Upgrade));
        assert!(!game.is_module_active(ModuleId::MoneyBar));
        assert_eq!(game.game_state.current_pane, ModuleId::AutoQueue);
    }
}
