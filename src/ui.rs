use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;

use crate::auto_queue::AutoQueue;
use crate::game::{Game, PaneRects, WindowPanes};
use crate::graph_pane::GraphPane;
use crate::money_bar::MoneyBar;
use crate::pane_module::{LayoutContext, PaneModule};
use crate::test_mode::AppModule;
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
    let mut graph_col_idx = None;
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
        col_count += 1;
    }

    if ctx.graph_active {
        if let Some(constraint) = GraphPane::column_constraint(&ctx) {
            column_constraints.push(constraint);
        }
        graph_col_idx = Some(col_count);
    }

    if column_constraints.is_empty() {
        return PaneRects::default();
    }

    let columns = Layout::horizontal(column_constraints).split(area);

    let (text_rect, upgrade_rect) = if let Some(idx) = main_col_idx {
        let col = columns[idx];
        if ctx.text_active && ctx.upgrade_active {
            let middle = Layout::vertical([
                TextPane::main_column_row_constraint().unwrap(),
                UpgradePane::main_column_row_constraint().unwrap(),
            ])
            .split(col);
            (middle[0], middle[1])
        } else if ctx.text_active {
            (col, Rect::default())
        } else {
            (Rect::default(), col)
        }
    } else {
        (Rect::default(), Rect::default())
    };

    PaneRects {
        text: text_rect,
        upgrade: upgrade_rect,
        auto_keys: keys_col_idx.map(|idx| columns[idx]),
        graph: graph_col_idx.map(|idx| columns[idx]),
    }
}

fn render_pane(
    frame: &mut Frame,
    game: &Game,
    module: AppModule,
    area: Rect,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let focused = match module {
        AppModule::Text => game.game_state.current_pane == WindowPanes::TextPane,
        AppModule::Upgrade => game.game_state.current_pane == WindowPanes::UpgradePane,
        AppModule::AutoQueue => game.game_state.current_pane == WindowPanes::AutoPane,
        AppModule::Graph => game.game_state.current_pane == WindowPanes::GraphPane,
        AppModule::MoneyBar => false,
    };

    match module {
        AppModule::Text => TextPane::render(frame, area, game, focused),
        AppModule::Upgrade => UpgradePane::render(frame, area, game, focused),
        AppModule::AutoQueue => AutoQueue::render(frame, area, game, focused),
        AppModule::Graph => GraphPane::render(frame, area, game, focused),
        AppModule::MoneyBar => MoneyBar::render(frame, area, game, focused),
    }
}

pub fn ui(frame: &mut Frame, game: &Game) -> PaneRects {
    let main_area = if game.is_module_active(AppModule::MoneyBar) {
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

    if game.is_module_active(AppModule::Upgrade) {
        render_pane(frame, game, AppModule::Upgrade, pane_rects.upgrade);
    }

    if game.is_module_active(AppModule::Text) {
        render_pane(frame, game, AppModule::Text, pane_rects.text);
    }

    if game.is_module_active(AppModule::AutoQueue) {
        if let Some(auto_rect) = pane_rects.auto_keys {
            render_pane(frame, game, AppModule::AutoQueue, auto_rect);
        }
    }

    if game.is_module_active(AppModule::Graph) {
        if let Some(graph_rect) = pane_rects.graph {
            render_pane(frame, game, AppModule::Graph, graph_rect);
        }
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
