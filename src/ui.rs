use core::f64;
use std::collections::HashMap;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Axis, Chart, Dataset, GraphType};
use ratatui::symbols::Marker;
use ratatui::Frame;

use crate::game::{Game, LETTER_QUEUE_HEIGHT, NUM_LETTERS, SECOND_POLL_WINDOW, UPGRADE_KEYS, WindowPanes, PaneRects};

use crate::upgrade::get_upgrades;

use crate::big_num::BigDollar;

const GRAY_DIM: Color = Color::Rgb(80, 80, 80);
const GRAY_MID: Color = Color::Rgb(140, 140, 140);
const MONEY_BAR_HEIGHT: u16 = 3;
const MONEY_GOLD: Color = Color::Rgb(255, 215, 0);

pub fn compute_pane_layout(area: Rect, game: &Game) -> PaneRects {
    let mut column_constraints = vec![Constraint::Fill(2)];
    let mut keys_column = None;
    let mut graph_column = None;
    let mut next_column = 1;

    if game.game_state.automation_unlocked {
        column_constraints.push(Constraint::Length(auto_pane_width(NUM_LETTERS)));
        keys_column = Some(next_column);
        next_column += 1;
    }
    if game.game_state.graphs_unlocked {
        column_constraints.push(Constraint::Length(SECOND_POLL_WINDOW as u16 * 2 + 2));
        graph_column = Some(next_column);
    }

    let columns = Layout::horizontal(column_constraints).split(area);
    let middle = Layout::vertical([
        Constraint::Length(7),
        Constraint::Min(5),
    ])
    .split(columns[0]);

    PaneRects {
        text: middle[0],
        upgrade: middle[1],
        auto_keys: keys_column.map(|col| columns[col]),
        graph: graph_column.map(|col| columns[col]),
    }
}

fn key_count(counts: &HashMap<String, u32>, key: &str) -> u32 {
    *counts.get(key).unwrap_or(&0)
}

fn count_color(count: u32) -> Color {
    if count == 0 {
        Color::Red
    } else if count < 10 {
        Color::Rgb(255, 140, 0)
    } else {
        Color::Green
    }
}

/// One char per column with a single space between: `a b c ...` → `cols * 2 - 1`.
fn auto_row_width(cols: usize) -> usize {
    cols * 2 - 1
}

/// Pane width: content plus left/right borders.
fn auto_pane_width(cols: usize) -> u16 {
    (auto_row_width(cols) + 2) as u16
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

fn auto_row(cells: impl IntoIterator<Item = (char, Color)>) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (ch, color)) in cells.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color),
        ));
    }
    Line::from(spans)
}

fn auto_zone(game: &Game, height: u16) -> Vec<Line<'static>> {
    let queue = &game.game_state.letter_queue;
    let counts = &game.game_state.counts;
    let keys: Vec<char> = if game.game_state.letter_compression_unlocked {
        ('A'..='Z').collect()
    } else {
        ('a'..='z').collect()
    };
    let mut content = Vec::new();

    // Spawner sits one row above where letters are spawned (queue row 0).
    content.push(auto_row((0..NUM_LETTERS).map(|_| ('█', Color::Cyan))));

    for row in 0..LETTER_QUEUE_HEIGHT {
        let cells = (0..NUM_LETTERS).map(|idx| {
            let ch = queue[idx][row];
            (ch, Color::Green)
        });
        content.push(auto_row(cells));
    }

    let key_counts: Vec<u32> = keys
        .iter()
        .map(|key| key_count(counts, &key.to_string()))
        .collect();
    let max_digits = key_counts
        .iter()
        .map(|n| n.to_string().len())
        .max()
        .unwrap_or(3)
        .max(3);

    content.push(auto_row(keys.iter().zip(key_counts.iter()).map(|(&c, &count)| {
        (c, count_color(count))
    })));

    for digit_row in 0..max_digits {
        content.push(auto_row(key_counts.iter().map(|&count| {
            let digits: Vec<char> = count.to_string().chars().collect();
            let ch = digits.get(digit_row).copied().unwrap_or(' ');
            (ch, count_color(count))
        })));
    }

    let pad = (height as usize).saturating_sub(content.len());
    let mut lines = vec![Line::from(""); pad];
    lines.extend(content);
    lines
}

fn render_money_bar(frame: &mut Frame, area: Rect, game: &Game) {
    let money_text = format!("Money: {}", game.game_state.money);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            money_text,
            Style::default()
                .fg(MONEY_GOLD)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MONEY_GOLD)),
        ),
        area,
    );
}

fn upgrade_zone(game: &Game) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if game.game_state.streaks_unlocked || game.game_state.trust_level != 0 {
        lines.push(Line::from(Span::raw(format!("Trust: {}", game.game_state.trust_level))));
    }
    if game.game_state.seniority_level > 0 {
        lines.push(Line::from(Span::raw(format!("Seniority: {}", game.game_state.seniority_level))));
    }
    for (i, kind) in game.get_displayed_upgrades().iter().enumerate() {
        let upgrade = get_upgrades().get(kind).unwrap();
        let level = game.game_state.upgrade_levels.get(kind).copied().unwrap_or(0);
        let max_level = upgrade.costs.len();
        let cost: BigDollar = upgrade.costs.get(level).copied().unwrap_or(BigDollar::from(0));
        let name = upgrade.name;
        let button = UPGRADE_KEYS.get(i).copied().unwrap_or('?');
        let color = if game.game_state.money >= cost || cost == BigDollar::from(0) {
            Color::Green
        } else {
            GRAY_MID
        };
        if upgrade.infinite {
            lines.push(Line::from(Span::styled(
                format!("[{button}] {cost}|{name}"),
                Style::default().fg(color),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("[{button}]({level}/{max_level}) {cost}|{name}"),
                Style::default().fg(color),
            )));
        }
    }
    lines
}

fn typing_zone(game: &Game, width: u16) -> Vec<Line<'static>> {
    let reference = game
        .game_state.current_text
        .get(game.game_state.current_line)
        .copied()
        .unwrap_or("");
    let next1 = game
        .game_state.current_text
        .get(game.game_state.current_line + 1)
        .copied()
        .unwrap_or("");
    let next2 = game
        .game_state.current_text
        .get(game.game_state.current_line + 2)
        .copied()
        .unwrap_or("");
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
    let text_width: usize = line.iter().map(Span::width).sum();
    let pad = (width as usize).saturating_sub(text_width + pct_text.chars().count());
    if pad > 0 {
        line.push(Span::raw(" ".repeat(pad)));
    }
    let pct_color = if pct >= 90 {
        Color::Green
    } else if pct >= 70 {
        Color::Yellow
    } else {
        Color::Red
    };
    line.push(Span::styled(pct_text, Style::default().fg(pct_color)));

    vec![
        Line::from(line),
        Line::from(Span::styled(next1, Style::default().fg(GRAY_MID))),
        Line::from(Span::styled(next2, Style::default().fg(GRAY_DIM))),
    ]
}

pub fn ui(frame: &mut Frame, game: &Game) -> PaneRects {
    let [money_rect, main_area] = Layout::vertical([
        Constraint::Length(MONEY_BAR_HEIGHT),
        Constraint::Min(0),
    ])
    .areas(frame.area());

    render_money_bar(frame, money_rect, game);

    let pane_rects = compute_pane_layout(main_area, game);

    frame.render_widget(
        Paragraph::new(upgrade_zone(game)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Upgrades")
                .border_style(
                    Style::default().fg(
                        match &game.game_state.current_pane {
                           WindowPanes::UpgradePane => Color::Cyan,
                           _ => Color::White,
                        }
                    )
                )),
        pane_rects.upgrade,
    );

    // Inner width accounts for left/right borders.
    let typing_text_width = pane_rects.upgrade.width.saturating_sub(2);
    frame.render_widget(
        Paragraph::new(typing_zone(game, typing_text_width))
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Text").border_style(
                    Style::default().fg(
                        match &game.game_state.current_pane {
                           WindowPanes::TextPane => Color::Cyan,
                           _ => Color::White,
                        }
                    )
                )),
        pane_rects.text,
    );

    if let Some(auto_rect) = pane_rects.auto_keys {
        let keys_height = auto_rect.height.saturating_sub(2);
        frame.render_widget(
            Paragraph::new(auto_zone(game, keys_height)).block(
                Block::default()
                .borders(Borders::ALL)
                .title("Keys")
                .border_style(Style::default().fg(
                        match &game.game_state.current_pane {
                            WindowPanes::AutoPane => Color::Cyan,
                            _ => Color::White,
                        },
                )),
            ),
            auto_rect,
        );
    }

    if let Some(graph_rect) = pane_rects.graph {
        // go around ring buffer starting from head + 1 and going to head - 1
        // wrapping around if necessary
        let mut v: Vec<(f64, f64)> = Vec::new();
        for i in 0..SECOND_POLL_WINDOW {
            let idx = (game.game_state.second_profit_bucket_head + 1 + i) % (SECOND_POLL_WINDOW + 1);
            let value = game.game_state.second_profit_buckets[idx];
            v.push((i as f64, value.into()));
        }
        let data: &[(f64, f64)] = &v;
        let min_profit = data.iter().copied().map(|(_, v)| v).fold(f64::INFINITY, f64::min);
        let max_profit = data.iter().copied().map(|(_, v)| v).fold(f64::NEG_INFINITY, f64::max);
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
                .marker(Marker::Braille)
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

    pane_rects
}
