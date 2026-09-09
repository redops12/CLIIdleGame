use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Color;
use ratatui::Frame;

use crate::game::Game;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum ModuleId {
    AutoQueue,
    Text,
    Upgrade,
    Graph,
    MoneyBar,
    AutoTyperSelector,
}

impl ModuleId {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto-queue" | "auto_queue" | "autoqueue" | "queue" | "auto" | "keys" => {
                Some(ModuleId::AutoQueue)
            }
            "text" | "typing" => Some(ModuleId::Text),
            "upgrade" | "upgrades" | "shop" => Some(ModuleId::Upgrade),
            "graph" | "graphs" => Some(ModuleId::Graph),
            "money" | "money-bar" | "money_bar" | "moneybar" => Some(ModuleId::MoneyBar),
            "auto-typer-selector" | "auto_typer_selector" | "auto-typer" | "auto_typer" => {
                Some(ModuleId::AutoTyperSelector)
            }
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ModuleId::AutoQueue => "auto-queue",
            ModuleId::Text => "text",
            ModuleId::Upgrade => "upgrade",
            ModuleId::Graph => "graph",
            ModuleId::MoneyBar => "money-bar",
            ModuleId::AutoTyperSelector => "auto-typer-selector",
        }
    }
}

/// Context for computing multi-pane layout constraints.
pub struct LayoutContext {
    pub text_active: bool,
    pub upgrade_active: bool,
    pub auto_active: bool,
    pub graph_active: bool,
}

impl LayoutContext {
    pub fn from_game(game: &Game) -> Self {
        Self {
            text_active: game.is_module_active(ModuleId::Text),
            upgrade_active: game.is_module_active(ModuleId::Upgrade),
            auto_active: game.is_module_active(ModuleId::AutoQueue),
            graph_active: game.is_module_active(ModuleId::Graph),
        }
    }

    pub fn active_pane_count(&self) -> usize {
        [
            self.text_active,
            self.upgrade_active,
            self.auto_active,
            self.graph_active,
        ]
        .iter()
        .filter(|&&active| active)
        .count()
    }

    pub fn has_main_column(&self) -> bool {
        self.text_active || self.upgrade_active
    }

    pub fn has_side_neighbors(&self, module: ModuleId) -> bool {
        match module {
            ModuleId::AutoQueue => {
                self.text_active || self.upgrade_active || self.graph_active
            }
            ModuleId::Graph => {
                self.text_active || self.upgrade_active || self.auto_active
            }
            _ => false,
        }
    }
}

pub fn focus_border_color(focused: bool) -> Color {
    if focused {
        Color::Cyan
    } else {
        Color::White
    }
}

/// Shared interface for game UI modules (panes and overlays).
pub trait PaneModule {
    fn title() -> &'static str;

    fn render(_frame: &mut Frame, _area: Rect, _game: &Game, _focused: bool);

    /// Horizontal column constraint when this module occupies its own column.
    fn column_constraint(ctx: &LayoutContext) -> Option<Constraint> {
        let _ = ctx;
        None
    }

    /// Vertical row constraint when stacked in the main (text/upgrade) column.
    fn main_column_row_constraint() -> Option<Constraint> {
        None
    }

    fn handle_input(_game: &mut Game, _key: KeyCode) {}

    fn update(_game: &mut Game) {}
}
