use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Color;
use ratatui::Frame;

use crate::game::{Game, GameState, WindowPanes};
use crate::test_mode::{AppModule, TestMode};

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
            text_active: game.is_module_active(AppModule::Text),
            upgrade_active: game.is_module_active(AppModule::Upgrade),
            auto_active: game.is_module_active(AppModule::AutoQueue),
            graph_active: game.is_module_active(AppModule::Graph),
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

    pub fn has_side_neighbors(&self, module: AppModule) -> bool {
        match module {
            AppModule::AutoQueue => {
                self.text_active || self.upgrade_active || self.graph_active
            }
            AppModule::Graph => {
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
    fn module_id() -> AppModule;

    /// Focusable pane identity, if this module participates in pane navigation.
    fn pane_id() -> Option<WindowPanes> {
        None
    }

    fn title() -> &'static str;

    fn is_unlocked(state: &GameState, test_mode: Option<&TestMode>) -> bool;

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

    fn render(_frame: &mut Frame, _area: Rect, _game: &Game, _focused: bool) {}
}
