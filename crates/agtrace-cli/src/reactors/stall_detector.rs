use crate::reactor::{Reaction, Reactor, ReactorContext, Severity};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

/// StallDetector - detects when agent is idle/stalled (waiting for input)
pub struct StallDetector {
    /// Idle threshold - how long before we consider the agent stalled
    idle_threshold: Duration,

    /// Last time we sent a notification
    last_notify: Option<DateTime<Utc>>,

    /// Cooldown period between notifications
    notify_cooldown: Duration,
}

impl StallDetector {
    /// Create a new StallDetector with the given idle threshold
    pub fn new(idle_threshold_secs: i64) -> Self {
        Self {
            idle_threshold: Duration::seconds(idle_threshold_secs),
            last_notify: None,
            notify_cooldown: Duration::seconds(300), // 5 minutes cooldown
        }
    }
}

impl Reactor for StallDetector {
    fn name(&self) -> &str {
        "StallDetector"
    }

    fn handle(&mut self, ctx: ReactorContext) -> Result<Reaction> {
        let now = Utc::now();
        let idle_duration = now - ctx.state.last_activity;

        // Check if agent is stalled (idle beyond threshold)
        if idle_duration > self.idle_threshold {
            // Check if we should notify (cooldown period elapsed)
            let should_notify = match self.last_notify {
                None => true,
                Some(last) => (now - last) > self.notify_cooldown,
            };

            if should_notify {
                self.last_notify = Some(now);

                let idle_mins = idle_duration.num_minutes();
                return Ok(Reaction::Intervene {
                    reason: format!(
                        "Agent idle for {} minutes. Waiting for user input?",
                        idle_mins
                    ),
                    severity: Severity::Notification,
                });
            }
        } else {
            // Reset last_notify if activity resumed
            self.last_notify = None;
        }

        Ok(Reaction::Continue)
    }
}
