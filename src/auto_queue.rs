use std::collections::HashMap;

use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::auto_typer_selector::AutoTyperSelector;
use crate::game::Game;
use crate::pane_module::{focus_border_color, LayoutContext, PaneModule, ModuleId};
use crate::text_sources::{get_lines_from_source, TextSource};

pub const NUM_LETTERS: usize            = 26;
pub const MAX_LINE_LENGTH: usize        = 40;
pub const LETTER_QUEUE_LEN: usize       = 10;
pub const QUEUE_MOVED_PER_UPDATE: usize = 2;

const READY_DELAY: u128   = 0;
const SORTING_DELAY: u128 = 100;
const QUEUING_DELAY: u128 = 300;
const QUEUE_DELAY: u128   = 50;
const LETTER_INCREMENT_DELAY: u128 = 2000;

pub fn idx_to_letter(idx: usize) -> char {
    (b'a' + idx as u8) as char
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
    pub letter_counts: HashMap<char, u32>,
    #[serde(with = "BigArray")]
    pub letter_queue: [[char; NUM_LETTERS]; MAX_LINE_LENGTH + LETTER_QUEUE_LEN],
    pub letter_compression_unlocked: bool,
    phase: SorterPhase,
    line_num: usize,
    #[serde(skip)]
    last_sorter_update: std::time::Instant,
    #[serde(skip)]
    last_queue_update: std::time::Instant,
    #[serde(skip)]
    last_letter_increment: std::time::Instant,

    #[serde(flatten)]
    pub auto_typer_selector: AutoTyperSelector,
}

impl Default for AutoQueue {
    fn default() -> Self {
        Self {
            auto_current_text: TextSource::Bartleby,
            auto_current_line: 0,
            letter_counts: HashMap::new(),
            letter_queue: [[' '; NUM_LETTERS]; MAX_LINE_LENGTH + LETTER_QUEUE_LEN],
            letter_compression_unlocked: false,
            phase: SorterPhase::Ready,
            line_num: 0,
            last_sorter_update: std::time::Instant::now(),
            last_queue_update: std::time::Instant::now(),
            last_letter_increment: std::time::Instant::now(),
            auto_typer_selector: AutoTyperSelector::default(),
        }
    }
}

impl AutoQueue {
    pub fn pane_width() -> u16 {
        MAX_LINE_LENGTH as u16 + LETTER_QUEUE_LEN as u16 + 2 + 1 + 1 + 4 // content + borders + letter +
                                                                     // count + spaces
    }

    pub fn get_letter_counts(&self, letter: char) -> u32 {
        *self.letter_counts.get(&letter).unwrap_or(&0)
    }

    pub fn increment_letter_count(&mut self, letter: char, count: u32) {
        let entry = self.letter_counts.entry(letter).or_insert(0);
        *entry = (*entry + count).min(999);
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        if let KeyCode::Char(c) = key {
            self.increment_letter_count(c.to_ascii_lowercase(), 1);
        }
    }

    fn increment_letter_counts(&mut self) {
        if self.last_letter_increment.elapsed().as_millis() < LETTER_INCREMENT_DELAY {
            return;
        }
        self.last_letter_increment = std::time::Instant::now();
        let to_add: Vec<(char, u32)> = self.auto_typer_selector.auto_typer_counts.iter().map(|(letter, count)| (*letter, *count)).collect();
        for (letter, count) in to_add {
            self.increment_letter_count(letter.clone(), count.clone());
        }
    }

    fn process_letter_queue(&mut self) -> String {
        let mut processed = String::new();
        for j in 0..NUM_LETTERS {
            let letter = idx_to_letter(j);
            if  self.letter_queue[MAX_LINE_LENGTH + LETTER_QUEUE_LEN - 1][j] == letter &&
                *self.letter_counts.get(&letter).unwrap_or(&0) > 0
            {
                processed.push(letter);
                self.letter_queue[MAX_LINE_LENGTH + LETTER_QUEUE_LEN - 1][j] = ' ';
                *self.letter_counts.entry(letter).or_insert(1) -= 1;
            }
        }

        processed
    }

    fn run_queue(&mut self) {
        if self.last_queue_update.elapsed().as_millis() < QUEUE_DELAY {
            return;
        }
        self.last_queue_update = std::time::Instant::now();

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

    fn run_sorter(&mut self) {
        match self.phase {
            SorterPhase::Ready => {
                if self.last_sorter_update.elapsed().as_millis() < READY_DELAY {
                    return;
                }
                self.last_sorter_update = std::time::Instant::now();

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
                if self.last_sorter_update.elapsed().as_millis() < SORTING_DELAY {
                    return;
                }
                self.last_sorter_update = std::time::Instant::now();

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
                if self.last_sorter_update.elapsed().as_millis() < QUEUING_DELAY {
                    return;
                }
                self.last_sorter_update = std::time::Instant::now();

                for _ in 0..QUEUE_MOVED_PER_UPDATE {
                    // move letters to the right putting one letter in the letter queue
                    for i in (1..MAX_LINE_LENGTH+1).rev() {
                        for j in 0..NUM_LETTERS {
                            if self.letter_queue[i][j] == ' ' {
                                self.letter_queue[i][j] = self.letter_queue[i - 1][j];
                                self.letter_queue[i-1][j] = ' ';
                            }
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

    pub fn update(
        &mut self,
    ) -> String {
        self.increment_letter_counts();
        let processed = self.process_letter_queue();

        self.run_queue();
        self.run_sorter();

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
            let color = count_color(self.get_letter_counts(letter));
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
            spans.push(Span::raw(format!(" {}", self.get_letter_counts(letter))));
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
        let lines = self.render_lines(keys_height as usize);
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Self::title())
                    .border_style(Style::default().fg(focus_border_color(is_focused))),
            ),
            area,
        );
    }
}

impl PaneModule for AutoQueue {
    fn title() -> &'static str {
        "Keys"
    }

    fn column_constraint(ctx: &LayoutContext) -> Option<Constraint> {
        if ctx.has_side_neighbors(ModuleId::AutoQueue) {
            Some(Constraint::Length(Self::pane_width()))
        } else {
            Some(Constraint::Fill(1))
        }
    }

    fn handle_input(game: &mut Game, key: KeyCode) {
        game.game_state.auto_queue.handle_input(key);
    }

    fn update(game: &mut Game) {
        let chars_processed = game.game_state.auto_queue.update();
        let money_change = game.calc_money_change(&chars_processed, &chars_processed);
        game.increment_money(money_change);
    }

    fn render(frame: &mut Frame, area: Rect, game: &Game, focused: bool) {
        game.game_state.auto_queue.ui(frame, area, focused);
    }
}
