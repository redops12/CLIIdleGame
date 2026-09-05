use core::f64;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Axis, Chart, Dataset, GraphType};
use ratatui::symbols::Marker;
use ratatui::Frame;

use crate::auto_queue::AutoQueue;
use crate::game::{Game, PaneRects, SECOND_POLL_WINDOW, UPGRADE_KEYS, WindowPanes};
use crate::test_mode::AppModule;

use crate::upgrade::get_upgrades;

use crate::big_num::BigDollar;

const GRAY_DIM: Color = Color::Rgb(80, 80, 80);
const GRAY_MID: Color = Color::Rgb(140, 140, 140);
const MONEY_BAR_HEIGHT: u16 = 3;
const MONEY_GOLD: Color = Color::Rgb(255, 215, 0);

pub fn compute_pane_layout(area: Rect, game: &Game) -> PaneRects {
    let text_active = game.is_module_active(AppModule::Text);
    let upgrade_active = game.is_module_active(AppModule::Upgrade);
    let auto_active = game.is_module_active(AppModule::AutoQueue);
    let graph_active = game.is_module_active(AppModule::Graph);

    // If only one pane is active, allocate the entire area directly
    if auto_active && !text_active && !upgrade_active && !graph_active {
        return PaneRects {
            text: Rect::default(),
            upgrade: Rect::default(),
            auto_keys: Some(area),
            graph: None,
        };
    }
    if text_active && !upgrade_active && !auto_active && !graph_active {
        return PaneRects {
            text: area,
            upgrade: Rect::default(),
            auto_keys: None,
            graph: None,
        };
    }
    if upgrade_active && !text_active && !auto_active && !graph_active {
        return PaneRects {
            text: Rect::default(),
            upgrade: area,
            auto_keys: None,
            graph: None,
        };
    }
    if graph_active && !text_active && !upgrade_active && !auto_active {
        return PaneRects {
            text: Rect::default(),
            upgrade: Rect::default(),
            auto_keys: None,
            graph: Some(area),
        };
    }

    let mut column_constraints = Vec::new();
    let mut main_col_idx = None;
    let mut keys_col_idx = None;
    let mut graph_col_idx = None;
    let mut col_count = 0;

    if text_active || upgrade_active {
        column_constraints.push(Constraint::Fill(2));
        main_col_idx = Some(col_count);
        col_count += 1;
    }

    if auto_active {
        if text_active || upgrade_active || graph_active {
            column_constraints.push(Constraint::Length(AutoQueue::pane_width()));
        } else {
            column_constraints.push(Constraint::Fill(1));
        }
        keys_col_idx = Some(col_count);
        col_count += 1;
    }

    if graph_active {
        if text_active || upgrade_active || auto_active {
            column_constraints.push(Constraint::Length(SECOND_POLL_WINDOW as u16 * 2 + 2));
        } else {
            column_constraints.push(Constraint::Fill(1));
        }
        graph_col_idx = Some(col_count);
    }

    if column_constraints.is_empty() {
        return PaneRects::default();
    }

    let columns = Layout::horizontal(column_constraints).split(area);

    let (text_rect, upgrade_rect) = if let Some(idx) = main_col_idx {
        let col = columns[idx];
        if text_active && upgrade_active {
            let middle = Layout::vertical([
                Constraint::Length(7),
                Constraint::Min(5),
            ])
            .split(col);
            (middle[0], middle[1])
        } else if text_active {
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

fn format_chart_value(value: f64) -> String {
    BigDollar::from(value).to_string()
}

fn profit_chart_y_bounds(data_min: f64, data_max: f64) -> [f64; 2] {
    let mut min = data_min.min(0.0);
    let mut max = data_max.max(0.0);
    if (max - min).abs() < f64::EPSILON {
        min -= 1.0;
        max += 1.0;
    }
    let pad = (max - min) * 0.05;
    [min - pad, max + pad]
}

fn profit_chart_y_labels(y_bounds: [f64; 2]) -> [String; 3] {
    let mid = (y_bounds[0] + y_bounds[1]) / 2.0;
    [
        format_chart_value(y_bounds[0]),
        format_chart_value(mid),
        format_chart_value(y_bounds[1]),
    ]
}

fn render_money_bar(frame: &mut Frame, area: Rect, game: &Game) {
    let mut money_text = format!("Money: {}", game.game_state.money);
    if game.game_state.trust_level != 0 || game.game_state.streaks_unlocked{
        money_text.push_str(&format!(
            " (Trust: {}%)",
            game.game_state.trust_level
        ));
    }
    if game.game_state.seniority_level > 0 {
        let numerals: Vec<&str> = vec!["I", "II", "III", "IV", "V", "VI"];
        money_text.push_str(&format!(
            " Typer {}",
            numerals[game.game_state.seniority_level as usize]
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            money_text,
            Style::default()
                .fg(MONEY_GOLD)
                .add_modifier(Modifier::BOLD),
        )))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MONEY_GOLD)),
        ),
        area,
    );
}

struct UpgradeRow {
    key: char,
    level: String,
    cost: String,
    name: &'static str,
    description: &'static str,
    color: Color,
    can_buy: bool,
}

fn upgrade_zone(game: &Game) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let mut rows = Vec::new();
    for (i, kind) in game.get_displayed_upgrades().iter().enumerate() {
        let upgrade = get_upgrades().get(kind).unwrap();
        let level = game.game_state.upgrade_levels.get(kind).copied().unwrap_or(0);
        let max_level = upgrade.costs.len();
        let cost: BigDollar = upgrade.costs.get(level).copied().unwrap_or(BigDollar::from(0));
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

fn typing_zone<'a>(game: &'a Game) -> Vec<Line<'a>> {
    let reference = game.get_text_line(None, None);
    let next1 = game.get_text_line(None, Some(game.game_state.current_line + 1));
    let next2 = game.get_text_line(None, Some(game.game_state.current_line + 2));
    let typed_chars: Vec<char> = game.game_state.typed.chars().collect();
    let ref_chars: Vec<char> = reference.chars().collect();
    let mut line = vec![];

    let mut correct = 0usize;
    let mut checked = ref_chars.len().max(typed_chars.len());
    for (i, e) in ref_chars.iter().enumerate() {
        let span = match typed_chars.get(i) {
            Some(&c) if c == *e => {
                correct += 1;
                Span::styled(c.to_string(), Style::default().fg(Color::Green))
            }
            Some(&c) if c != *e && !c.is_whitespace() => {
                Span::styled(c.to_string(), Style::default().fg(Color::Red).add_modifier(ratatui::style::Modifier::CROSSED_OUT))
            }
            Some(&c) if c != *e && c.is_whitespace() => {
                Span::styled(c.to_string(), Style::default().bg(Color::Red).add_modifier(ratatui::style::Modifier::CROSSED_OUT))
            }
            Some(&_) => Span::styled(e.to_string(), Style::default().fg(Color::White)),
            None if i == typed_chars.len() => Span::styled(e.to_string(), Style::default().fg(Color::White).bg(GRAY_DIM)),
            None => Span::styled(e.to_string(), Style::default().fg(Color::White)),
        };
        line.push(span);
    }
    for c in typed_chars.iter().skip(ref_chars.len()) {
        checked += 1;
        if c.is_whitespace() {
            line.push(Span::styled(c.to_string(), Style::default().bg(Color::Red)));
        } else {
            line.push(Span::styled(c.to_string(), Style::default().fg(Color::Red)));
        }
    }

    let pct = if checked == 0 || game.game_state.seniority_level >= 5 {
        100
    } else {
        (correct * 100 / (checked as f64 * (1.0 - game.game_state.seniority_level as f64 * 0.2)).round() as usize).min(100)
    };
    let money_change = game.calc_money_change(&game.game_state.typed, reference);
    let pct_text = format!("{pct:>3}% {money_change:+}");

    let pct_color = if pct >= 90 {
        Color::Green
    } else if pct >= 70 {
        Color::Yellow
    } else {
        Color::Red
    };

    vec![
        Line::from(Span::styled(pct_text, Style::default().fg(pct_color))),
        Line::from(line),
        Line::from(Span::styled(next1, Style::default().fg(GRAY_MID))),
        Line::from(Span::styled(next2, Style::default().fg(GRAY_DIM))),
    ]
}

pub fn ui(frame: &mut Frame, game: &Game) -> PaneRects {
    let main_area = if game.is_module_active(AppModule::MoneyBar) {
        let [money_rect, rest] = Layout::vertical([
            Constraint::Length(MONEY_BAR_HEIGHT),
            Constraint::Min(0),
        ])
        .areas(frame.area());
        render_money_bar(frame, money_rect, game);
        rest
    } else {
        frame.area()
    };

    let pane_rects = compute_pane_layout(main_area, game);

    if game.is_module_active(AppModule::Upgrade)
        && pane_rects.upgrade.width > 0
        && pane_rects.upgrade.height > 0
    {
        frame.render_widget(
            Paragraph::new(upgrade_zone(game)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Upgrades")
                    .border_style(Style::default().fg(match &game.game_state.current_pane {
                        WindowPanes::UpgradePane => Color::Cyan,
                        _ => Color::White,
                    })),
            ),
            pane_rects.upgrade,
        );
    }

    if game.is_module_active(AppModule::Text)
        && pane_rects.text.width > 0
        && pane_rects.text.height > 0
    {
        // Inner width accounts for left/right borders.
        frame.render_widget(
            Paragraph::new(typing_zone(game)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Text")
                    .border_style(Style::default().fg(match &game.game_state.current_pane {
                        WindowPanes::TextPane => Color::Cyan,
                        _ => Color::White,
                    })),
            ),
            pane_rects.text,
        );
    }

    if game.is_module_active(AppModule::AutoQueue) {
        if let Some(auto_rect) = pane_rects.auto_keys {
            if auto_rect.width > 0 && auto_rect.height > 0 {
                let is_focused = game.game_state.current_pane == WindowPanes::AutoPane;
                let empty = Vec::new();
                let lines = game
                    .text_sources
                    .get(&game.game_state.auto_queue.auto_current_text)
                    .unwrap_or(&empty);
                game.game_state.auto_queue.ui(
                    frame,
                    auto_rect,
                    is_focused,
                    game.game_state.letter_compression_unlocked,
                    lines,
                );
            }
        }
    }

    if game.is_module_active(AppModule::Graph) {
        if let Some(graph_rect) = pane_rects.graph {
            if graph_rect.width > 0 && graph_rect.height > 0 {
                // go around ring buffer starting from head + 1 and going to head - 1
                // wrapping around if necessary
                let mut v: Vec<(f64, f64)> = Vec::new();
                for i in 0..SECOND_POLL_WINDOW {
                    let idx = (game.game_state.second_profit_bucket_head + 1 + i)
                        % (SECOND_POLL_WINDOW + 1);
                    let value = game.game_state.second_profit_buckets[idx];
                    v.push((i as f64, value.into()));
                }
                let data: &[(f64, f64)] = &v;
                let min_profit =
                    data.iter().copied().map(|(_, v)| v).fold(f64::INFINITY, f64::min);
                let max_profit =
                    data.iter().copied().map(|(_, v)| v).fold(f64::NEG_INFINITY, f64::max);
                let y_bounds = profit_chart_y_bounds(min_profit, max_profit);
                let y_labels = profit_chart_y_labels(y_bounds);
                let x_max = SECOND_POLL_WINDOW as f64;
                let zero_line = [(0.0, 0.0), (x_max, 0.0)];

                let chart = Chart::new(vec![
                    Dataset::default()
                        .graph_type(GraphType::Line)
                        .marker(Marker::Dot)
                        .style(Style::default().fg(GRAY_MID))
                        .data(&zero_line),
                    Dataset::default()
                        .name("PPS")
                        .marker(Marker::HalfBlock)
                        .graph_type(GraphType::Line)
                        .style(Color::Cyan)
                        .data(data),
                ])
                .x_axis(
                    Axis::default()
                        .bounds([0.0, x_max])
                        .title("Seconds")
                        .labels([
                            "0".to_string(),
                            ((x_max / 2.0) as u32).to_string(),
                            (x_max as u32).to_string(),
                        ])
                        .style(Style::default().fg(GRAY_MID)),
                )
                .y_axis(
                    Axis::default()
                        .bounds(y_bounds)
                        .title("PPS")
                        .labels(y_labels)
                        .style(Style::default().fg(GRAY_MID)),
                );
                frame.render_widget(
                    chart.block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Graphs")
                            .border_style(Style::default().fg(
                                match &game.game_state.current_pane {
                                    WindowPanes::GraphPane => Color::Cyan,
                                    _ => Color::White,
                                },
                            )),
                    ),
                    graph_rect,
                );
            }
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
