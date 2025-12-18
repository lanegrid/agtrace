use crate::display_model::{DisplayOptions, TokenSummaryDisplay};
use crate::reactor::{Reaction, Reactor, ReactorContext};
use crate::token_limits::TokenLimits;
use crate::views::session::print_event;
use agtrace_types::EventPayload;
use anyhow::Result;

// NOTE: TuiRenderer Design Rationale
//
// Why separate display into a reactor (not inline in watch loop)?
// - Keeps display logic isolated from event processing
// - Can be disabled/replaced without modifying core loop
// - Testable independently (mock reactor can verify display calls)
// - Future: Enable multiple renderers (TUI, JSON stream, HTML)

#[allow(dead_code)]
pub struct TuiRenderer {
    token_limits: TokenLimits,
}

impl TuiRenderer {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            token_limits: TokenLimits::new(),
        }
    }

    #[allow(dead_code)]
    fn print_token_summary(&self, ctx: &ReactorContext) {
        let total = ctx.state.total_context_window_tokens() as u64;

        if total == 0 {
            return;
        }

        let limit = ctx.state.context_window_limit.or_else(|| {
            ctx.state
                .model
                .as_ref()
                .and_then(|m| self.token_limits.get_limit(m).map(|l| l.total_limit))
        });

        let summary = TokenSummaryDisplay {
            input: ctx.state.total_input_side_tokens(),
            output: ctx.state.total_output_side_tokens(),
            cache_creation: ctx.state.current_usage.cache_creation.0,
            cache_read: ctx.state.current_usage.cache_read.0,
            total: ctx.state.total_context_window_tokens(),
            limit,
            model: ctx.state.model.clone(),
        };

        let opts = DisplayOptions {
            enable_color: true,
            relative_time: false,
            truncate_text: None,
        };

        println!();
        let lines = crate::views::session::format_token_summary(&summary, &opts);
        for line in lines {
            println!("{}", line);
        }
    }
}

impl Reactor for TuiRenderer {
    fn name(&self) -> &str {
        "TuiRenderer"
    }

    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction> {
        let event = ctx.event;
        let turn_context = ctx.state.turn_count;
        let project_root = ctx.state.project_root.as_deref();

        print_event(event, turn_context, project_root);

        // Print token summary after TokenUsage events (when tokens are updated)
        if matches!(event.payload, EventPayload::TokenUsage(_)) {
            self.print_token_summary(&ctx);
        }

        Ok(Reaction::Continue)
    }
}
