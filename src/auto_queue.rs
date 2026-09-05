use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use serde::{Deserialize, Serialize};

use crate::game::TextSource;

pub const NUM_LETTERS: usize = 26;
pub const LETTER_QUEUE_HEIGHT: usize = 10;
pub const AUTO_TEXT_LINES: usize = 5;
pub const UNSET_AUTO_LINE_SOURCE: usize = usize::MAX;

pub fn default_auto_line_sources() -> [usize; AUTO_TEXT_LINES] {
    [UNSET_AUTO_LINE_SOURCE; AUTO_TEXT_LINES]
}

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

pub fn auto_row(cells: impl IntoIterator<Item = (char, Color)>) -> Line<'static> {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoQueue {
    pub auto_current_text: TextSource,
    pub auto_current_line: usize,
    pub remaining_auto_lines: [String; AUTO_TEXT_LINES],
    #[serde(default = "default_auto_line_sources")]
    pub auto_line_sources: [usize; AUTO_TEXT_LINES],
    pub counts: HashMap<String, u32>,
    pub letter_queue: [[char; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
}

impl Default for AutoQueue {
    fn default() -> Self {
        Self {
            auto_current_text: TextSource::Bartleby,
            auto_current_line: 0,
            remaining_auto_lines: std::array::from_fn(|_| String::new()),
            auto_line_sources: default_auto_line_sources(),
            counts: HashMap::new(),
            letter_queue: [[' '; LETTER_QUEUE_HEIGHT]; NUM_LETTERS],
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

    pub fn row_width() -> usize {
        auto_row_width(NUM_LETTERS)
    }

    pub fn increment(&mut self, key: &str) {
        *self.counts.entry(key.to_string()).or_insert(0) += 1;
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        if let KeyCode::Char(c) = key {
            self.increment(&c.to_string());
        }
    }

    #[allow(dead_code)]
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        self.handle_input(key.code);
    }

    pub fn compress_letters(&mut self) {
        for x in 0..NUM_LETTERS {
            let mut start_y = LETTER_QUEUE_HEIGHT;
            for y in (4..LETTER_QUEUE_HEIGHT).rev() {
                if self.letter_queue[x][y] == idx_to_letter(x) {
                    start_y = y;
                    break;
                }
            }
            if start_y == LETTER_QUEUE_HEIGHT {
                continue;
            }
            let mut compression_possible = true;
            for y in start_y - 4..=start_y {
                if self.letter_queue[x][y] == ' ' {
                    compression_possible = false;
                    break;
                }
            }
            if compression_possible {
                self.letter_queue[x][start_y] =
                    self.letter_queue[x][start_y - 1].to_ascii_uppercase();
                for y in start_y - 4..start_y {
                    self.letter_queue[x][y] = ' ';
                }
            }
        }
    }

    pub fn clear_letters(&mut self, letter_compression_unlocked: bool) -> String {
        let mut chars_processed = String::new();
        for x in 0..NUM_LETTERS {
            let letter = if letter_compression_unlocked {
                idx_to_upper_letter(x)
            } else {
                idx_to_letter(x)
            };
            if self.letter_queue[x][LETTER_QUEUE_HEIGHT - 1] == letter {
                let count = self.counts.entry(letter.to_string()).or_insert(0);
                if *count > 0 {
                    *count -= 1;
                    chars_processed.push(letter);
                    self.letter_queue[x][LETTER_QUEUE_HEIGHT - 1] = ' ';
                }
            }
        }
        chars_processed
    }

    pub fn advance_letters(&mut self) {
        for y in (1..LETTER_QUEUE_HEIGHT).rev() {
            for x in 0..NUM_LETTERS {
                if self.letter_queue[x][y] == ' ' {
                    self.letter_queue[x][y] = self.letter_queue[x][y - 1];
                    self.letter_queue[x][y - 1] = ' ';
                }
            }
        }
    }

    pub fn spawn_letter(&mut self, c: char) -> bool {
        if !c.is_ascii_alphabetic() {
            return false;
        }

        let idx = c.to_ascii_lowercase() as usize - 'a' as usize;
        if idx >= NUM_LETTERS {
            return false;
        }

        if self.letter_queue[idx][0] != ' ' {
            return false;
        }

        self.letter_queue[idx][0] = (b'a' + idx as u8) as char;
        true
    }

    pub fn sort_and_spawn(&mut self) {
        let mut chars_to_spawn = HashSet::new();
        for i in (0..AUTO_TEXT_LINES).rev() {
            let old_auto_text = std::mem::take(&mut self.remaining_auto_lines[i]);
            let mut remaining_auto_text = String::new();
            for c in old_auto_text.chars() {
                if c.is_whitespace() {
                    remaining_auto_text.push(' ');
                    continue;
                }

                if !chars_to_spawn.insert(c) {
                    remaining_auto_text.push(c);
                } else if !self.spawn_letter(c) {
                    remaining_auto_text.push(c);
                } else {
                    remaining_auto_text.push(' ');
                }
            }
            self.remaining_auto_lines[i] = remaining_auto_text;
        }
    }

    fn auto_line_complete(line: &str) -> bool {
        !line.is_empty() && line.chars().all(|c| c.is_whitespace())
    }

    fn collect_auto_line_state(&self) -> (HashMap<usize, String>, HashSet<usize>) {
        let mut saved = HashMap::new();
        let mut completed = HashSet::new();
        let base = self.auto_current_line;

        for i in 0..AUTO_TEXT_LINES {
            let src = if self.auto_line_sources[i] == UNSET_AUTO_LINE_SOURCE {
                base + i
            } else {
                self.auto_line_sources[i]
            };
            let line = &self.remaining_auto_lines[i];
            if Self::auto_line_complete(line) {
                completed.insert(src);
            } else if !line.is_empty() {
                saved.insert(src, line.clone());
            }
        }

        (saved, completed)
    }

    pub fn ensure_auto_line_sources_initialized(&mut self) {
        let base = self.auto_current_line;
        for i in 0..AUTO_TEXT_LINES {
            if self.auto_line_sources[i] == UNSET_AUTO_LINE_SOURCE {
                self.auto_line_sources[i] = base + i;
            }
        }
    }

    fn next_unused_source(
        start: usize,
        source_len: usize,
        completed: &HashSet<usize>,
        reserved: &HashSet<usize>,
    ) -> Option<usize> {
        let mut src = start;
        while src < source_len {
            if !completed.contains(&src) && !reserved.contains(&src) {
                return Some(src);
            }
            src += 1;
        }
        None
    }

    pub fn refill_auto_lines(&mut self, source_lines: &[String]) {
        self.ensure_auto_line_sources_initialized();
        let (saved, completed) = self.collect_auto_line_state();
        let mut reserved = HashSet::new();
        let mut search = self.auto_current_line;

        for slot in 0..AUTO_TEXT_LINES {
            let preferred = self.auto_line_sources[slot];
            let src = if preferred != UNSET_AUTO_LINE_SOURCE
                && saved.contains_key(&preferred)
                && !reserved.contains(&preferred)
            {
                preferred
            } else if let Some(found) =
                Self::next_unused_source(search, source_lines.len(), &completed, &reserved)
            {
                search = found + 1;
                found
            } else {
                self.remaining_auto_lines[slot] = String::new();
                self.auto_line_sources[slot] = UNSET_AUTO_LINE_SOURCE;
                continue;
            };

            reserved.insert(src);
            self.auto_line_sources[slot] = src;
            self.remaining_auto_lines[slot] = saved
                .get(&src)
                .cloned()
                .unwrap_or_else(|| source_lines.get(src).cloned().unwrap_or_default());
        }

        if let Some(min_src) = self
            .auto_line_sources
            .iter()
            .filter(|&&s| s != UNSET_AUTO_LINE_SOURCE)
            .min()
        {
            self.auto_current_line = *min_src;
        }
    }

    pub fn auto_preview_start(&self) -> usize {
        self.auto_line_sources
            .iter()
            .filter(|&&s| s != UNSET_AUTO_LINE_SOURCE)
            .max()
            .map(|max| max + 1)
            .unwrap_or(self.auto_current_line)
    }

    pub fn update(
        &mut self,
        letter_compression_unlocked: bool,
        source_lines: &[String],
    ) -> String {
        if letter_compression_unlocked {
            self.compress_letters();
        }

        let chars_processed = self.clear_letters(letter_compression_unlocked);
        self.advance_letters();
        self.refill_auto_lines(source_lines);
        self.sort_and_spawn();
        for _ in 0..AUTO_TEXT_LINES {
            let before = self.remaining_auto_lines.clone();
            self.refill_auto_lines(source_lines);
            if before == self.remaining_auto_lines {
                break;
            }
        }

        chars_processed
    }

    #[allow(dead_code)]
    pub fn update_from_sources(
        &mut self,
        letter_compression_unlocked: bool,
        text_sources: &HashMap<TextSource, Vec<String>>,
    ) -> String {
        let empty = Vec::new();
        let lines = text_sources.get(&self.auto_current_text).unwrap_or(&empty);
        self.update(letter_compression_unlocked, lines)
    }

    pub fn render_lines(
        &self,
        height: u16,
        letter_compression_unlocked: bool,
        source_lines: &[String],
    ) -> Vec<Line<'static>> {
        let queue = &self.letter_queue;
        let counts = &self.counts;
        let width = Self::row_width();
        let keys: Vec<char> = if letter_compression_unlocked {
            ('A'..='Z').collect()
        } else {
            ('a'..='z').collect()
        };
        let mut content = Vec::new();

        content.push(Line::from(Span::styled(
            "_".repeat(width),
            Style::default().fg(Color::Cyan),
        )));
        for line in &self.remaining_auto_lines {
            content.push(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::Cyan),
            )));
        }
        content.push(Line::from(Span::styled(
            "^|".repeat(NUM_LETTERS - 1) + "^",
            Style::default().fg(Color::Cyan),
        )));

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

        content.push(auto_row(
            keys.iter()
                .zip(key_counts.iter())
                .map(|(&c, &count)| (c, count_color(count))),
        ));

        for digit_row in 0..max_digits {
            content.push(auto_row(key_counts.iter().map(|&count| {
                let digits: Vec<char> = count.to_string().chars().collect();
                let ch = digits.get(digit_row).copied().unwrap_or(' ');
                (ch, count_color(count))
            })));
        }

        let pad = (height as usize).saturating_sub(content.len());
        let mut lines = vec![];
        for i in (0..pad).rev() {
            let source_idx = self.auto_preview_start() + i;
            let next_line = source_lines.get(source_idx).map(|s| s.as_str()).unwrap_or("");
            lines.push(Line::from(Span::styled(
                next_line.to_string(),
                Style::default().fg(Color::Cyan),
            )));
        }
        lines.extend(content);
        lines
    }

    pub fn ui(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_focused: bool,
        letter_compression_unlocked: bool,
        source_lines: &[String],
    ) {
        let keys_height = area.height.saturating_sub(2);
        let border_color = if is_focused {
            Color::Cyan
        } else {
            Color::White
        };
        let lines = self.render_lines(keys_height, letter_compression_unlocked, source_lines);
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

    #[allow(dead_code)]
    pub fn ui_from_sources(
        &self,
        frame: &mut Frame,
        area: Rect,
        is_focused: bool,
        letter_compression_unlocked: bool,
        text_sources: &HashMap<TextSource, Vec<String>>,
    ) {
        let empty = Vec::new();
        let lines = text_sources.get(&self.auto_current_text).unwrap_or(&empty);
        self.ui(frame, area, is_focused, letter_compression_unlocked, lines);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_default_auto_queue() {
        let aq = AutoQueue::default();
        assert_eq!(aq.auto_current_text, TextSource::Bartleby);
        assert_eq!(aq.auto_current_line, 0);
        assert_eq!(aq.counts.len(), 0);
        for row in aq.letter_queue {
            for ch in row {
                assert_eq!(ch, ' ');
            }
        }
    }

    #[test]
    fn test_handle_input_and_key_event() {
        let mut aq = AutoQueue::new();
        aq.handle_input(KeyCode::Char('a'));
        assert_eq!(*aq.counts.get("a").unwrap(), 1);

        aq.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(*aq.counts.get("a").unwrap(), 2);

        aq.handle_input(KeyCode::Char('b'));
        assert_eq!(*aq.counts.get("b").unwrap(), 1);

        // Non-character keys do nothing
        aq.handle_input(KeyCode::Enter);
        assert_eq!(aq.counts.len(), 2);
    }

    #[test]
    fn test_spawn_and_advance() {
        let mut aq = AutoQueue::new();
        assert!(aq.spawn_letter('a'));
        assert_eq!(aq.letter_queue[0][0], 'a');

        // Cannot spawn into occupied slot
        assert!(!aq.spawn_letter('a'));

        // Advance moves down
        aq.advance_letters();
        assert_eq!(aq.letter_queue[0][0], ' ');
        assert_eq!(aq.letter_queue[0][1], 'a');
    }

    #[test]
    fn test_clear_letters() {
        let mut aq = AutoQueue::new();
        aq.letter_queue[0][LETTER_QUEUE_HEIGHT - 1] = 'a';

        // Without count, nothing cleared
        let cleared = aq.clear_letters(false);
        assert_eq!(cleared, "");
        assert_eq!(aq.letter_queue[0][LETTER_QUEUE_HEIGHT - 1], 'a');

        // With count, letter is cleared and returned
        aq.increment("a");
        let cleared = aq.clear_letters(false);
        assert_eq!(cleared, "a");
        assert_eq!(aq.letter_queue[0][LETTER_QUEUE_HEIGHT - 1], ' ');
        assert_eq!(*aq.counts.get("a").unwrap(), 0);
    }

    #[test]
    fn test_compress_letters() {
        let mut aq = AutoQueue::new();
        for y in 0..5 {
            aq.letter_queue[0][y] = 'a';
        }
        aq.compress_letters();
        // Lowercase 'a' stacked 5 high compresses to uppercase 'A' at index 4
        assert_eq!(aq.letter_queue[0][4], 'A');
        for y in 0..4 {
            assert_eq!(aq.letter_queue[0][y], ' ');
        }
    }

    #[test]
    fn test_update() {
        let mut aq = AutoQueue::new();
        let source_lines = vec![
            "hello world".to_string(),
            "second line".to_string(),
        ];

        aq.increment("h");
        let cleared = aq.update(false, &source_lines);
        // Initially nothing at bottom, so cleared is empty
        assert_eq!(cleared, "");

        // Letters from "hello world" should have been sorted and spawned into row 0
        let spawned_chars: Vec<char> = (0..NUM_LETTERS)
            .map(|col| aq.letter_queue[col][0])
            .filter(|&c| c != ' ')
            .collect();
        assert!(!spawned_chars.is_empty());
    }

    #[test]
    fn test_render_lines() {
        let aq = AutoQueue::new();
        let source_lines = vec!["preview text".to_string()];
        let lines = aq.render_lines(20, false, &source_lines);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_json_roundtrip() {
        let mut aq = AutoQueue::new();
        aq.increment("x");
        aq.letter_queue[0][0] = 'a';

        let json = serde_json::to_string(&aq).unwrap();
        let loaded: AutoQueue = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.counts.get("x"), Some(&1));
        assert_eq!(loaded.letter_queue[0][0], 'a');
    }

    #[test]
    fn test_save2_json_compatibility() {
        let data = std::fs::read_to_string("save2.json").unwrap();
        let state: crate::game::GameState = serde_json::from_str(&data).unwrap();
        assert_eq!(state.auto_queue.auto_current_text, TextSource::Bartleby);
        assert_eq!(state.auto_queue.auto_current_line, 1);
        assert_eq!(state.auto_queue.counts.get("a"), Some(&28));
        assert_eq!(state.auto_queue.counts.get("b"), Some(&5));
    }
}
