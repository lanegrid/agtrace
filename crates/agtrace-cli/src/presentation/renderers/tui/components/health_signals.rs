use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::Component;
use crate::presentation::renderers::tui::app::AppState;
use chrono::Utc;

pub(crate) struct HealthSignalsComponent;

impl Component for HealthSignalsComponent {
    fn render(&self, f: &mut Frame, area: Rect, state: &mut AppState) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(Span::styled(
                " SESSION HEALTH ",
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            ));

        let mut lines = Vec::new();

        // 1. Rhythm indicator
        let (rhythm_bar, rhythm_label, rhythm_color) = calculate_rhythm_indicator(state);
        lines.push(Line::from(vec![
            Span::styled("Rhythm: ", Style::default().fg(Color::Gray)),
            Span::styled(rhythm_bar, Style::default().fg(rhythm_color)),
            Span::raw(" "),
            Span::styled(rhythm_label, Style::default().fg(rhythm_color)),
        ]));

        // 2. Token rate (tokens per minute)
        if let Some(start) = state.session_start_time {
            let elapsed_mins = Utc::now()
                .signed_duration_since(start)
                .num_minutes()
                .max(1) as f64;
            let token_rate = state.previous_token_total as f64 / elapsed_mins;
            lines.push(Line::from(vec![
                Span::styled("Token Rate: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:.1}k/min", token_rate / 1000.0),
                    Style::default().fg(Color::White),
                ),
            ]));
        }

        // 3. Compaction proximity
        if let Some(ctx) = &state.context_usage {
            let input_total = ctx.fresh_input + ctx.cache_creation + ctx.cache_read;
            let input_pct = (input_total as f64 / ctx.limit as f64) * 100.0;

            let (comp_bar, comp_label, comp_color) = if let Some(buffer_pct) = state.compaction_buffer_pct {
                let trigger_threshold = 100.0 - buffer_pct;
                if input_pct >= trigger_threshold {
                    ("▓▓▓▓█", "Near limit", Color::Rgb(255, 165, 0)) // Orange
                } else if input_pct >= trigger_threshold - 10.0 {
                    ("▓▓▓░░", "Approaching", Color::Yellow)
                } else {
                    ("▓▓░░░", "OK", Color::Green)
                }
            } else {
                if input_pct >= 90.0 {
                    ("▓▓▓▓█", "Near limit", Color::Red)
                } else if input_pct >= 70.0 {
                    ("▓▓▓░░", "Moderate", Color::Yellow)
                } else {
                    ("▓▓░░░", "OK", Color::Green)
                }
            };

            lines.push(Line::from(vec![
                Span::styled("Compaction: ", Style::default().fg(Color::Gray)),
                Span::styled(comp_bar, Style::default().fg(comp_color)),
                Span::raw(" "),
                Span::styled(comp_label, Style::default().fg(comp_color)),
            ]));
        }

        // 4. Last activity
        if let Some(last) = state.last_activity {
            let elapsed = Utc::now().signed_duration_since(last).num_seconds();
            let activity_text = if elapsed < 5 {
                format!("{}s ago", elapsed)
            } else if elapsed < 60 {
                format!("{}s ago", elapsed)
            } else {
                let mins = elapsed / 60;
                format!("{}m ago", mins)
            };
            lines.push(Line::from(vec![
                Span::styled("Last Activity: ", Style::default().fg(Color::Gray)),
                Span::styled(activity_text, Style::default().fg(Color::White)),
            ]));
        }

        // 5. Total steps
        lines.push(Line::from(vec![
            Span::styled("Total Steps: ", Style::default().fg(Color::Gray)),
            Span::styled(
                state.current_step_number.to_string(),
                Style::default().fg(Color::White),
            ),
        ]));

        // Add blank line
        lines.push(Line::from(""));

        // 6. Context window details (if available)
        if let Some(ctx) = &state.context_usage {
            let ratio = ctx.used as f64 / ctx.limit as f64;
            let usage_pct = ratio * 100.0;

            let (gauge_color, _) = if ratio > 0.9 {
                (Color::Red, Color::LightRed)
            } else if ratio > 0.7 {
                (Color::Rgb(255, 165, 0), Color::Rgb(255, 200, 100))
            } else {
                (Color::Green, Color::Cyan)
            };

            lines.push(Line::from(vec![Span::styled(
                "Context Window",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));

            // Progress bar (30 chars)
            let bar_width = 30;
            let filled = ((ratio * bar_width as f64) as usize).min(bar_width);
            let empty = bar_width.saturating_sub(filled);
            let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

            lines.push(Line::from(vec![Span::styled(
                bar,
                Style::default().fg(gauge_color),
            )]));

            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "{} / {} ({:.0}%)",
                        format_tokens(ctx.used as i32),
                        format_tokens(ctx.limit as i32),
                        usage_pct
                    ),
                    Style::default().fg(Color::White),
                ),
            ]));

            let input_total = ctx.fresh_input + ctx.cache_creation + ctx.cache_read;
            lines.push(Line::from(vec![
                Span::styled("In: ", Style::default().fg(Color::Gray)),
                Span::styled(format_tokens(input_total), Style::default().fg(Color::Cyan)),
                Span::styled("  Out: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_tokens(ctx.output),
                    Style::default().fg(Color::Green),
                ),
            ]));
        }

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }
}

fn calculate_rhythm_indicator(state: &AppState) -> (&'static str, &'static str, Color) {
    if state.activity_timestamps.len() < 2 {
        return ("░░░░░", "Unknown", Color::DarkGray);
    }

    // Calculate intervals between consecutive activities
    let intervals: Vec<i64> = state
        .activity_timestamps
        .iter()
        .zip(state.activity_timestamps.iter().skip(1))
        .map(|(t1, t2)| t2.signed_duration_since(*t1).num_seconds())
        .collect();

    if intervals.is_empty() {
        return ("░░░░░", "Unknown", Color::DarkGray);
    }

    // Calculate average and variance
    let avg: f64 = intervals.iter().sum::<i64>() as f64 / intervals.len() as f64;
    let variance: f64 = intervals
        .iter()
        .map(|&x| {
            let diff = x as f64 - avg;
            diff * diff
        })
        .sum::<f64>()
        / intervals.len() as f64;
    let std_dev = variance.sqrt();

    // Determine rhythm based on average interval and variance
    let coefficient_of_variation = if avg > 0.0 { std_dev / avg } else { 1.0 };

    if avg < 2.0 {
        // Very fast - potential runaway
        ("████▓", "Rapid", Color::Red)
    } else if coefficient_of_variation < 0.3 {
        // Low variance - steady rhythm
        ("██▓░░", "Steady", Color::Green)
    } else if coefficient_of_variation < 0.6 {
        // Moderate variance
        ("█▓█░░", "Variable", Color::Yellow)
    } else {
        // High variance - irregular
        ("█░█░█", "Irregular", Color::Rgb(255, 165, 0))
    }
}

fn format_tokens(count: i32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
