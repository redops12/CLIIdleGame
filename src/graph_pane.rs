use core::f64;

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, GraphType};
use ratatui::Frame;

use crate::big_num::BigDollar;
use crate::game::{Game, SECOND_POLL_WINDOW};
use crate::pane_module::{focus_border_color, LayoutContext, PaneModule, ModuleId};

const GRAY_MID: Color = Color::Rgb(140, 140, 140);

pub struct GraphPane;

impl GraphPane {
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
            Self::format_chart_value(y_bounds[0]),
            Self::format_chart_value(mid),
            Self::format_chart_value(y_bounds[1]),
        ]
    }

    pub fn ui(frame: &mut Frame, area: Rect, focused: bool, game: &Game) {
        let mut v: Vec<(f64, f64)> = Vec::new();
        for i in 0..SECOND_POLL_WINDOW {
            let idx =
                (game.game_state.second_profit_bucket_head + 1 + i) % (SECOND_POLL_WINDOW + 1);
            let value = game.game_state.second_profit_buckets[idx];
            v.push((i as f64, value.into()));
        }
        let data: &[(f64, f64)] = &v;
        let min_profit = data
            .iter()
            .copied()
            .map(|(_, v)| v)
            .fold(f64::INFINITY, f64::min);
        let max_profit = data
            .iter()
            .copied()
            .map(|(_, v)| v)
            .fold(f64::NEG_INFINITY, f64::max);
        let y_bounds = Self::profit_chart_y_bounds(min_profit, max_profit);
        let y_labels = Self::profit_chart_y_labels(y_bounds);
        let x_max = SECOND_POLL_WINDOW as f64;
        let zero_line = [(0.0, 0.0), (x_max, 0.0)];

        let chart = Chart::new(vec![
            Dataset::default()
                .graph_type(GraphType::Line)
                .marker(Marker::Braille)
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
                    .title(Self::title())
                    .border_style(Style::default().fg(focus_border_color(focused))),
            ),
            area,
        );
    }
}

impl PaneModule for GraphPane {
    fn title() -> &'static str {
        "Graphs"
    }

    fn column_constraint(ctx: &LayoutContext) -> Option<Constraint> {
        if ctx.has_side_neighbors(ModuleId::Graph) {
            Some(Constraint::Length(SECOND_POLL_WINDOW as u16 * 2 + 2))
        } else {
            Some(Constraint::Fill(1))
        }
    }

    fn render(frame: &mut Frame, area: Rect, game: &Game, focused: bool) {
        Self::ui(frame, area, focused, game);
    }
}
