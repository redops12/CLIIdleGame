use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Clear, Block};
use ratatui::style::Style;
use ratatui::Frame;

use crate::auto_queue::AutoQueue;
use crate::auto_typer_selector::AutoTyperSelector;
use crate::game::{Game, PaneRects};
use crate::graph_pane::GraphPane;
use crate::money_bar::MoneyBar;
use crate::pane_module::{focus_background_color, LayoutContext, PaneModule, ModuleId};
use crate::text_pane::TextPane;
use crate::upgrade_pane::UpgradePane;

pub fn compute_pane_layout(area: Rect, game: &Game) -> PaneRects {
    let ctx = LayoutContext::from_game(game);

    if ctx.active_pane_count() == 1 {
        if ctx.auto_active {
            return PaneRects {
                text: Rect::default(),
                upgrade: Rect::default(),
                auto_keys: Some(area),
                graph: None,
            };
        }
        if ctx.text_active {
            return PaneRects {
                text: area,
                upgrade: Rect::default(),
                auto_keys: None,
                graph: None,
            };
        }
        if ctx.upgrade_active {
            return PaneRects {
                text: Rect::default(),
                upgrade: area,
                auto_keys: None,
                graph: None,
            };
        }
        if ctx.graph_active {
            return PaneRects {
                text: Rect::default(),
                upgrade: Rect::default(),
                auto_keys: None,
                graph: Some(area),
            };
        }
    }

    let mut column_constraints = Vec::new();
    let mut main_col_idx = None;
    let mut keys_col_idx = None;
    let mut col_count = 0;

    if ctx.has_main_column() {
        column_constraints.push(Constraint::Fill(2));
        main_col_idx = Some(col_count);
        col_count += 1;
    }

    if ctx.auto_active {
        if let Some(constraint) = AutoQueue::column_constraint(&ctx) {
            column_constraints.push(constraint);
        }
        keys_col_idx = Some(col_count);
    }

    if column_constraints.is_empty() {
        return PaneRects::default();
    }

    let columns = Layout::horizontal(column_constraints).split(area);

    let (text_rect, upgrade_rect, graph_rect) = if let Some(idx) = main_col_idx {
        let col = columns[idx];
        if ctx.text_active && ctx.upgrade_active && ctx.graph_active {
            let middle = Layout::vertical([
                TextPane::main_column_row_constraint().unwrap(),
                UpgradePane::main_column_row_constraint().unwrap(),
                GraphPane::main_column_row_constraint().unwrap(),
            ])
            .split(col);
            (middle[0], middle[1], Some(middle[2]))
        } else if ctx.text_active && ctx.upgrade_active {
            let middle = Layout::vertical([
                TextPane::main_column_row_constraint().unwrap(),
                UpgradePane::main_column_row_constraint().unwrap(),
            ])
            .split(col);
            (middle[0], middle[1], None)
        } else if ctx.text_active {
            (col, Rect::default(), None)
        } else {
            (Rect::default(), col, None)
        }
    } else {
        (Rect::default(), Rect::default(), None)
    };

    PaneRects {
        text: text_rect,
        upgrade: upgrade_rect,
        graph: graph_rect,
        auto_keys: keys_col_idx.map(|idx| columns[idx]),
    }
}

fn compute_floating_area(whole: Rect) -> Rect {
    let [_, rect, _] = Layout::vertical([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)]).areas(whole);
    let [_, rect2, _] = Layout::horizontal([Constraint::Percentage(20), Constraint::Percentage(60), Constraint::Percentage(20)]).areas(rect);
    return rect2;
}

fn render_pane(
    frame: &mut Frame,
    game: &Game,
    module: ModuleId,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let focused = game.game_state.current_pane == module;

    let bg = Block::default().style(Style::default().bg(focus_background_color(focused)));
    frame.render_widget(bg, area);

    match module {
        ModuleId::Text => TextPane::render(frame, area, game, focused),
        ModuleId::Upgrade => UpgradePane::render(frame, area, game, focused),
        ModuleId::AutoQueue => AutoQueue::render(frame, area, game, focused),
        ModuleId::Graph => GraphPane::render(frame, area, game, focused),
        ModuleId::MoneyBar => MoneyBar::render(frame, area, game, focused),
        ModuleId::AutoTyperSelector => AutoTyperSelector::render(frame, area, game, focused),

    }
}

pub fn ui(frame: &mut Frame, game: &Game) -> PaneRects {
    let main_area = if game.is_module_active(ModuleId::MoneyBar) {
        let [money_rect, rest] = Layout::vertical([
            MoneyBar::height_constraint(),
            Constraint::Min(0),
        ])
        .areas(frame.area());
        MoneyBar::render(frame, money_rect, game, false);
        rest
    } else {
        frame.area()
    };

    let pane_rects = compute_pane_layout(main_area, game);

    if game.is_module_active(ModuleId::Upgrade) {
        render_pane(frame, game, ModuleId::Upgrade, pane_rects.upgrade);
    }

    if game.is_module_active(ModuleId::Text) {
        render_pane(frame, game, ModuleId::Text, pane_rects.text);
    }

    if game.is_module_active(ModuleId::AutoQueue) {
        if let Some(auto_rect) = pane_rects.auto_keys {
            render_pane(frame, game, ModuleId::AutoQueue, auto_rect);
        }
    }

    if game.is_module_active(ModuleId::Graph) {
        if let Some(graph_rect) = pane_rects.graph {
            render_pane(frame, game, ModuleId::Graph, graph_rect);
        }
    }

    if let Some(floating_pane_id) = game.get_floating_pane() {
        let floating_area = compute_floating_area(frame.area());
        frame.render_widget(Clear, floating_area);
        render_pane(frame, game, floating_pane_id, floating_area);
    }

    pane_rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_mode::TestMode;

    #[test]
    fn test_compute_pane_layout_auto_queue_only() {
        let (_tx, rx) = crossbeam_channel::unbounded();
        let game = Game::new_with_test_mode(rx, Some(TestMode::auto_queue()));
        let area = Rect::new(0, 0, 100, 30);
        let panes = compute_pane_layout(area, &game);

        assert_eq!(panes.auto_keys, Some(area));
        assert_eq!(panes.text, Rect::default());
        assert_eq!(panes.upgrade, Rect::default());
        assert_eq!(panes.graph, None);
    }

    #[test]
    fn test_compute_pane_layout_default_game() {
        let (_tx, rx) = crossbeam_channel::unbounded();
        let game = Game::new(rx);
        let area = Rect::new(0, 0, 100, 30);
        let panes = compute_pane_layout(area, &game);

        assert_ne!(panes.text, Rect::default());
        assert_ne!(panes.upgrade, Rect::default());
        assert_eq!(panes.auto_keys, None);
        assert_eq!(panes.graph, None);
    }
}
