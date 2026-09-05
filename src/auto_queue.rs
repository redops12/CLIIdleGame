use std::collections::{HashMap};

use crossterm::event::{KeyCode};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::text_sources::{TextSource, get_lines_from_source};

pub const NUM_LETTERS: usize = 26;
pub const MAX_LINE_LENGTH: usize = 40;
pub const LETTER_QUEUE_LEN: usize = 10;

pub fn idx_to_letter(idx: usize) -> char {
    (b'a' + idx as u8) as char
}

pub fn idx_to_upper_letter(idx: usize) -> char {
    (b'A' + idx as u8) as char
}

pub fn key_count(counts: &HashMap<String, u32>, key: &str) -> u32 {
    *counts.get(key).unwrap_or(&0)
}

pub fn count_color(count: u32) -> Color {
    if count == 0 {
        Color::Red
    } else if count < 10 {
        Color::Rgb(255, 140, 0)
    } else {
        Color::Green
    }
}

/// One char per column with a single space between: `a b c ...` → `cols * 2 - 1`.
pub fn auto_row_width(cols: usize) -> usize {
    cols * 2 - 1
}

/// Pane width: content plus left/right borders.
pub fn auto_pane_width(cols: usize) -> u16 {
    (auto_row_width(cols) + 2) as u16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum SorterPhase {
    Ready,
    Sorting,
    Queueing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoQueue {
    pub auto_current_text: TextSource,
    pub auto_current_line: usize,
    pub counts: HashMap<String, u32>,
    #[serde(with = "BigArray")]
    pub letter_queue: [[char; NUM_LETTERS]; MAX_LINE_LENGTH + LETTER_QUEUE_LEN],
    pub letter_compression_unlocked: bool,
    phase: SorterPhase,
    line_num: usize,
    sorter_delay: u64,
    #[serde(skip)]
    last_sorter_update: std::time::Instant,
}

impl Default for AutoQueue {
    fn default() -> Self {
        Self {
            auto_current_text: TextSource::Bartleby,
            auto_current_line: 0,
            counts: HashMap::new(),
            letter_queue: [[' '; NUM_LETTERS]; MAX_LINE_LENGTH + LETTER_QUEUE_LEN],
            letter_compression_unlocked: false,
            phase: SorterPhase::Ready,
            line_num: 0,
            sorter_delay: 10,
            last_sorter_update: std::time::Instant::now(),
        }
    }
}

impl AutoQueue {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pane_width() -> u16 {
        auto_pane_width(NUM_LETTERS)
    }

    fn increment_key_counts(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        if let KeyCode::Char(c) = key {
            self.increment_key_counts(&c.to_string());
        }
    }

    fn run_sorter(&mut self) {
        match self.phase {
            SorterPhase::Ready => {
                self.phase = SorterPhase::Sorting;
                self.line_num = 0;
                let next_line = get_lines_from_source(self.auto_current_text, self.auto_current_line);
                for (i, ch) in next_line.chars().enumerate() {
                    if i < MAX_LINE_LENGTH {
                        self.letter_queue[i][0] = ch;
                    } else {
                        panic!("Line length exceeds MAX_LINE_LENGTH: {}", next_line);
                    }
                }
                self.auto_current_line += 1;
            }
            SorterPhase::Sorting => {
                if self.line_num >= NUM_LETTERS - 1 {
                    self.phase = SorterPhase::Queueing;
                    return
                }

                // move letters down unless this is their line
                let letter = idx_to_letter(self.line_num);
                for i in 0..MAX_LINE_LENGTH {
                    let ch = self.letter_queue[i][self.line_num];
                    if ch != letter {
                        self.letter_queue[i][self.line_num + 1] = ch;
                        self.letter_queue[i][self.line_num] = ' ';
                    }
                }
                self.line_num += 1;
            }
            SorterPhase::Queueing => {
                // move letters to the right putting one letter in the letter queue
                for i in (1..MAX_LINE_LENGTH+1).rev() {
                    for j in 0..NUM_LETTERS {
                        if self.letter_queue[i][j] == ' ' {
                            self.letter_queue[i][j] = self.letter_queue[i - 1][j];
                            self.letter_queue[i-1][j] = ' ';
                        }
                    }
                }
                let all_spaces = (0..MAX_LINE_LENGTH).all(
                    |i| (0..NUM_LETTERS).all(
                        |j| self.letter_queue[i][j] == ' '));
                if all_spaces {
                    self.phase = SorterPhase::Ready;
                }
            }
        }
    }

    fn process_letter_queue(&mut self) -> String {
        let mut processed = String::new();
        for j in 0..NUM_LETTERS {
            let letter = idx_to_letter(j);
            if self.letter_queue[MAX_LINE_LENGTH + LETTER_QUEUE_LEN - 1][j] == letter {
                processed.push(letter);
                self.letter_queue[MAX_LINE_LENGTH + LETTER_QUEUE_LEN - 1][j] = ' ';
            }
        }

        processed
    }

    fn run_queue(&mut self) {
        // move letters to the right putting one letter in the letter queue
        for i in (MAX_LINE_LENGTH..MAX_LINE_LENGTH + LETTER_QUEUE_LEN).rev() {
            for j in 0..NUM_LETTERS {
                if self.letter_queue[i][j] == ' ' {
                    self.letter_queue[i][j] = self.letter_queue[i - 1][j];
                    self.letter_queue[i - 1][j] = ' ';
                }
            }
        }
    }

    pub fn update(
        &mut self,
    ) -> String {
        let processed = self.process_letter_queue();
        self.run_queue();

        if self.last_sorter_update.elapsed().as_millis() >= self.sorter_delay as u128 {
            self.last_sorter_update = std::time::Instant::now();
            self.run_sorter();
        }

        processed
    }

    pub fn render_lines(
        &self,
        height: usize,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let source_length = height - NUM_LETTERS;
        for i in (0..source_length).rev() {
            let line_idx = self.auto_current_line + i;
            let line = get_lines_from_source(self.auto_current_text, line_idx);
            lines.push(Line::from(line));
        }
        for j in 0..NUM_LETTERS {
            let letter = idx_to_letter(j);
            let color = count_color(key_count(&self.counts, &letter.to_string()));
            let mut spans = Vec::new();
            for i in 0..MAX_LINE_LENGTH + LETTER_QUEUE_LEN {
                spans.push(Span::styled(
                    self.letter_queue[i][j].to_string(),
                    Style::default().fg(Color::Cyan),
                ));
            }
            spans.push(Span::styled(
                format!(" {}", letter),
                Style::default().fg(color),
            ));
            spans.push(Span::raw(format!(" {}", key_count(&self.counts, &letter.to_string()))));
            lines.push(Line::from(spans));
        }

        lines
    }

    pub fn ui(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_focused: bool,
    ) {
        let keys_height = area.height.saturating_sub(2);
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::White
        };
        let lines = self.render_lines(keys_height as usize);
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Keys")
                    .border_style(Style::default().fg(border_color)),
            ),
            area,
        );
    }
}
