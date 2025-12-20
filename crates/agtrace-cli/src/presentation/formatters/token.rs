use super::{FormatOptions, TokenSummaryDisplay};
use agtrace_runtime::reactor::SessionState;
use agtrace_runtime::TokenLimits;
use owo_colors::OwoColorize;
use std::fmt;

/// View for displaying token usage information
pub struct TokenUsageView {
    pub summary: TokenSummaryDisplay,
    pub options: FormatOptions,
}

impl TokenUsageView {
    pub fn from_state(state: &SessionState, options: FormatOptions) -> Self {
        let token_limits = TokenLimits::new();
        let token_spec = state.model.as_ref().and_then(|m| token_limits.get_limit(m));

        let limit = state
            .context_window_limit
            .or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));

        let compaction_buffer_pct = token_spec.map(|spec| spec.compaction_buffer_pct);

        let summary = TokenSummaryDisplay {
            input: state.total_input_side_tokens(),
            output: state.total_output_side_tokens(),
            cache_creation: state.current_usage.cache_creation.0,
            cache_read: state.current_usage.cache_read.0,
            total: state.total_context_window_tokens(),
            limit,
            model: state.model.clone(),
            compaction_buffer_pct,
        };

        Self { summary, options }
    }

    pub fn from_usage_data(
        fresh_input: i32,
        cache_creation: i32,
        cache_read: i32,
        output: i32,
        reasoning_tokens: i32,
        model: Option<String>,
        context_window_limit: Option<u64>,
        options: FormatOptions,
    ) -> Self {
        let token_limits = TokenLimits::new();
        let token_spec = model.as_ref().and_then(|m| token_limits.get_limit(m));

        let limit =
            context_window_limit.or_else(|| token_spec.as_ref().map(|spec| spec.effective_limit()));

        let compaction_buffer_pct = token_spec.map(|spec| spec.compaction_buffer_pct);

        let input = fresh_input + cache_creation + cache_read;
        let total = input + output + reasoning_tokens;

        let summary = TokenSummaryDisplay {
            input,
            output,
            cache_creation,
            cache_read,
            total,
            limit,
            model,
            compaction_buffer_pct,
        };

        Self { summary, options }
    }
}

impl fmt::Display for TokenUsageView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts = &self.summary;
        let enable_color = self.options.enable_color;

        if enable_color {
            writeln!(f, "{}", "Context Window".bright_white().bold())?;
        } else {
            writeln!(f, "Context Window")?;
        }

        // Display progress bar if limit is available
        if let Some(limit) = ts.limit {
            let bar = create_progress_bar(ts.total, limit as i32, 30, enable_color);
            writeln!(f, "  {}", bar)?;

            let total_pct = (ts.total as f64 / limit as f64) * 100.0;
            let input_pct = (ts.input as f64 / limit as f64) * 100.0;
            let output_pct = (ts.output as f64 / limit as f64) * 100.0;

            if enable_color {
                writeln!(
                    f,
                    "  {}: {} / {} ({:.1}%)",
                    "Total".cyan(),
                    format_token_count(ts.total).bright_white(),
                    format_token_count(limit as i32).bright_white(),
                    total_pct
                )?;
            } else {
                writeln!(
                    f,
                    "  Total: {} / {} ({:.1}%)",
                    format_token_count(ts.total),
                    format_token_count(limit as i32),
                    total_pct
                )?;
            }

            // Input/Output breakdown
            if enable_color {
                writeln!(
                    f,
                    "    {}: {} ({:.1}%)",
                    "Input".green(),
                    format_token_count(ts.input).bright_white(),
                    input_pct
                )?;
                writeln!(
                    f,
                    "    {}: {} ({:.1}%)",
                    "Output".blue(),
                    format_token_count(ts.output).bright_white(),
                    output_pct
                )?;
            } else {
                writeln!(
                    f,
                    "    Input: {} ({:.1}%)",
                    format_token_count(ts.input),
                    input_pct
                )?;
                writeln!(
                    f,
                    "    Output: {} ({:.1}%)",
                    format_token_count(ts.output),
                    output_pct
                )?;
            }

            // Cache information
            if ts.cache_creation > 0 || ts.cache_read > 0 {
                if enable_color {
                    writeln!(f, "    {}", "Cache:".cyan())?;
                    if ts.cache_creation > 0 {
                        writeln!(
                            f,
                            "      Created: {}",
                            format_token_count(ts.cache_creation).bright_white()
                        )?;
                    }
                    if ts.cache_read > 0 {
                        writeln!(
                            f,
                            "      Read: {}",
                            format_token_count(ts.cache_read).bright_white()
                        )?;
                    }
                } else {
                    writeln!(f, "    Cache:")?;
                    if ts.cache_creation > 0 {
                        writeln!(
                            f,
                            "      Created: {}",
                            format_token_count(ts.cache_creation)
                        )?;
                    }
                    if ts.cache_read > 0 {
                        writeln!(f, "      Read: {}", format_token_count(ts.cache_read))?;
                    }
                }
            }

            // Compaction buffer status
            if let Some(buffer_pct) = ts.compaction_buffer_pct {
                if buffer_pct > 0.0 {
                    let trigger_pct = 100.0 - buffer_pct;
                    let status = if input_pct >= trigger_pct {
                        if enable_color {
                            "TRIGGERED".red().to_string()
                        } else {
                            "TRIGGERED".to_string()
                        }
                    } else if enable_color {
                        format!("at {:.0}%", trigger_pct).dimmed().to_string()
                    } else {
                        format!("at {:.0}%", trigger_pct)
                    };
                    writeln!(f, "  Compaction buffer: {}", status)?;
                }
            }
        } else {
            // No limit available, just show totals
            if enable_color {
                writeln!(
                    f,
                    "  {}: {}",
                    "Total".cyan(),
                    format_token_count(ts.total).bright_white()
                )?;
                writeln!(
                    f,
                    "    {}: {}",
                    "Input".green(),
                    format_token_count(ts.input).bright_white()
                )?;
                writeln!(
                    f,
                    "    {}: {}",
                    "Output".blue(),
                    format_token_count(ts.output).bright_white()
                )?;
            } else {
                writeln!(f, "  Total: {}", format_token_count(ts.total))?;
                writeln!(f, "    Input: {}", format_token_count(ts.input))?;
                writeln!(f, "    Output: {}", format_token_count(ts.output))?;
            }

            if ts.cache_creation > 0 || ts.cache_read > 0 {
                if enable_color {
                    writeln!(f, "    {}", "Cache:".cyan())?;
                    if ts.cache_creation > 0 {
                        writeln!(
                            f,
                            "      Created: {}",
                            format_token_count(ts.cache_creation).bright_white()
                        )?;
                    }
                    if ts.cache_read > 0 {
                        writeln!(
                            f,
                            "      Read: {}",
                            format_token_count(ts.cache_read).bright_white()
                        )?;
                    }
                } else {
                    writeln!(f, "    Cache:")?;
                    if ts.cache_creation > 0 {
                        writeln!(
                            f,
                            "      Created: {}",
                            format_token_count(ts.cache_creation)
                        )?;
                    }
                    if ts.cache_read > 0 {
                        writeln!(f, "      Read: {}", format_token_count(ts.cache_read))?;
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn create_progress_bar(current: i32, total: i32, width: usize, enable_color: bool) -> String {
    let filled_width = if total > 0 {
        ((current as f64 / total as f64) * width as f64) as usize
    } else {
        0
    };
    let empty_width = width.saturating_sub(filled_width);

    let filled = "=".repeat(filled_width);
    let empty = ".".repeat(empty_width);
    let bar = format!("[{}{}]", filled, empty);

    if enable_color {
        if current as f64 / total as f64 > 0.9 {
            bar.red().to_string()
        } else if current as f64 / total as f64 > 0.7 {
            bar.yellow().to_string()
        } else {
            bar.green().to_string()
        }
    } else {
        bar
    }
}

fn format_token_count(count: i32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}k", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_progress_bar() {
        let bar = create_progress_bar(50, 100, 10, false);
        assert_eq!(bar, "[=====.....]");

        let bar = create_progress_bar(100, 100, 10, false);
        assert_eq!(bar, "[==========]");

        let bar = create_progress_bar(0, 100, 10, false);
        assert_eq!(bar, "[..........]");
    }

    #[test]
    fn test_format_token_count() {
        assert_eq!(format_token_count(500), "500");
        assert_eq!(format_token_count(1500), "1.5k");
        assert_eq!(format_token_count(1500000), "1.5M");
    }
}
