use std::collections::HashMap;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::game::{Game, WindowPanes, LETTER_QUEUE_HEIGHT, NUM_LETTERS, UPGRADE_KEYS};

use crate::upgrade::get_upgrades;

use crate::BigNum::BigDollar;

const GRAY_DIM: Color = Color::Rgb(80, 80, 80);
const GRAY_MID: Color = Color::Rgb(140, 140, 140);

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
    let keys: Vec<char> = ('a'..='z').collect();
    let mut content = Vec::new();

    // Spawner sits one row above where letters are spawned (queue row 0).
    content.push(auto_row((0..NUM_LETTERS).map(|_| ('█', Color::Cyan))));

    for row in 0..LETTER_QUEUE_HEIGHT {
        let cells = (0..NUM_LETTERS).map(|idx| {
            let ch = if queue[idx][row] {
                (b'a' + idx as u8) as char
            } else {
                ' '
            };
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

fn upgrade_zone(game: &Game, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.push(Line::from(Span::raw(format!("Money: {}", game.game_state.money))));
    lines.push(Line::from(Span::raw("=".repeat(width as usize))));
    for (i, kind) in game.get_displayed_upgrades().iter().enumerate() {
        let upgrade = get_upgrades().get(kind).unwrap();
        let level = game.game_state.upgrade_levels.get(kind).copied().unwrap_or(0);
        let max_level = upgrade.costs.len();
        let cost: BigDollar = upgrade.costs.get(level).copied().unwrap_or(BigDollar::from(0));
        let name = upgrade.name;
        let description = upgrade.description;
        let button = UPGRADE_KEYS.get(i).copied().unwrap_or('?');
        let color = if game.game_state.money >= cost {
            Color::Green
        } else {
            GRAY_MID
        };
        lines.push(Line::from(Span::styled(
            format!("[{button}]({level}/{max_level}) {cost}|{name} --- {description}"),
            Style::default().fg(color),
        )));
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
                Span::styled(c.to_string(), Style::default().fg(Color::Red))
            }
            Some(&c) if c != *e && c.is_whitespace() => {
                Span::styled(c.to_string(), Style::default().bg(Color::Red))
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

    let pct = if checked == 0 {
        100
    } else {
        correct * 100 / checked
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
        Line::from(Span::styled(next2, Style::default().fg(GRAY_DIM))),
        Line::from(Span::styled(next1, Style::default().fg(GRAY_MID))),
        Line::from(line),
    ]
}

pub fn ui(frame: &mut Frame, game: &Game) {
    let columns = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Fill(2),
        Constraint::Length(auto_pane_width(NUM_LETTERS)),
    ])
    .split(frame.area());

    let middle = Layout::vertical([
        Constraint::Min(5),
        Constraint::Length(5),
    ])
    .split(columns[1]);

    frame.render_widget(
        Paragraph::new("Esc / Ctrl-C to quit")
        .block(
            Block::default()
            .borders(Borders::ALL)
            .title("Left")
            .border_style(
                Style::default().fg(
                    match &game.game_state.current_pane {
                        WindowPanes::HelpPane => Color::Green,
                        _ => Color::White,
                    }
                )
            )),
        columns[0],
    );

    let upgrade_text_width = middle[1].width.saturating_sub(2);
    frame.render_widget(
        Paragraph::new(upgrade_zone(game, upgrade_text_width)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stats")
                .border_style(
                    Style::default().fg(
                        match &game.game_state.current_pane {
                           WindowPanes::UpgradePane => Color::Green,
                           _ => Color::White,
                        }
                    )
                )),
        middle[0],
    );

    // Inner width accounts for left/right borders.
    let typing_text_width = middle[1].width.saturating_sub(2);
    frame.render_widget(
        Paragraph::new(typing_zone(game, typing_text_width))
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Text").border_style(
                    Style::default().fg(
                        match &game.game_state.current_pane {
                           WindowPanes::TextPane => Color::Green,
                           _ => Color::White,
                        }
                    )
                )),
        middle[1],
    );

    let keys_height = columns[2].height.saturating_sub(2);
    frame.render_widget(
        Paragraph::new(auto_zone(game, keys_height)).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Keys")
                .border_style(Style::default().fg(
                    match &game.game_state.current_pane {
                        WindowPanes::AutoPane => Color::Green,
                        _ => Color::White,
                    },
                )),
        ),
        columns[2],
    );
}
