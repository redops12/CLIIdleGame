use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::big_num::BigDollar;
use crate::game::{Game, UPGRADE_KEYS};
use crate::pane_module::{focus_border_color, PaneModule};
use crate::upgrade::get_upgrades;

const GRAY_MID: Color = Color::Rgb(140, 140, 140);

pub struct UpgradePane;

struct UpgradeRow {
    key: char,
    level: String,
    cost: String,
    name: &'static str,
    description: &'static str,
    color: Color,
    can_buy: bool,
}

impl UpgradePane {
    fn render_lines(game: &Game) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut rows = Vec::new();

        for (i, kind) in game.get_displayed_upgrades().iter().enumerate() {
            let upgrade = get_upgrades().get(kind).unwrap();
            let level = game.game_state.upgrade_levels.get(kind).copied().unwrap_or(0);
            let max_level = upgrade.costs.len();
            let cost: BigDollar = upgrade
                .costs
                .get(level)
                .copied()
                .unwrap_or(BigDollar::from(0));
            let button = UPGRADE_KEYS.get(i).copied().unwrap_or('?');
            let can_buy = game.game_state.money >= cost || cost == BigDollar::from(0);
            let color = if can_buy {
                Color::Green
            } else {
                GRAY_MID
            };
            let level_str = if upgrade.infinite {
                String::new()
            } else {
                format!("({level}/{max_level})")
            };
            rows.push(UpgradeRow {
                key: button,
                level: level_str,
                cost: cost.to_string(),
                name: upgrade.name,
                description: upgrade.description,
                color,
                can_buy,
            });
        }

        let level_width = rows.iter().map(|row| row.level.len()).max().unwrap_or(0);
        let cost_width = rows.iter().map(|row| row.cost.len()).max().unwrap_or(0);
        let name_width = rows.iter().map(|row| row.name.len()).max().unwrap_or(0);

        for row in rows {
            let text = if level_width > 0 {
                format!(
                    "[{}] {:level_w$} {:>cost_w$} {:name_w$} --- {}",
                    row.key, row.level, row.cost, row.name, row.description,
                    level_w = level_width,
                    cost_w = cost_width,
                    name_w = name_width,
                )
            } else {
                format!(
                    "[{}] {:>cost_w$} {:name_w$} {}",
                    row.key, row.cost, row.name, row.description,
                    cost_w = cost_width,
                    name_w = name_width,
                )
            };
            let mut style = Style::default().fg(row.color);
            if row.can_buy {
                style = style.add_modifier(Modifier::BOLD);
            }
            lines.push(Line::from(Span::styled(text, style)));
        }
        lines
    }

    pub fn ui(frame: &mut Frame, area: Rect, focused: bool, game: &Game) {
        frame.render_widget(
            Paragraph::new(Self::render_lines(game)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Self::title())
                    .border_style(Style::default().fg(focus_border_color(focused))),
            ),
            area,
        );
    }

    fn handle_upgrade_key(game: &mut Game, key: KeyCode) {
        if let KeyCode::Char(c) = key {
            let Some(&upgrade_id) = game
                .get_displayed_upgrades()
                .iter()
                .enumerate()
                .find(|(i, _)| UPGRADE_KEYS.get(*i) == Some(&c))
                .map(|(_, upgrade_id)| upgrade_id)
            else {
                logging::info(&format!("No upgrade found for key '{}'", c));
                return;
            };
            game.buy_upgrade(upgrade_id);
        }
    }
}

impl PaneModule for UpgradePane {
    fn title() -> &'static str {
        "Upgrades"
    }

    fn main_column_row_constraint() -> Option<Constraint> {
        Some(Constraint::Min(5))
    }

    fn handle_input(game: &mut Game, key: KeyCode) {
        Self::handle_upgrade_key(game, key);
    }

    fn render(frame: &mut Frame, area: Rect, game: &Game, focused: bool) {
        Self::ui(frame, area, focused, game);
    }
}
