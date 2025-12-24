use std::collections::HashMap;
use std::fmt;

use crate::presentation::view_models::{
    CreateView, PackReportViewModel, ReportTemplate, SessionDigest, ViewMode,
};

impl CreateView for PackReportViewModel {
    fn create_view<'a>(&'a self, _mode: ViewMode) -> Box<dyn fmt::Display + 'a> {
        Box::new(PackReportView { data: self })
    }
}

struct PackReportView<'a> {
    data: &'a PackReportViewModel,
}

impl<'a> PackReportView<'a> {
    fn render_digest_summary(
        &self,
        f: &mut fmt::Formatter<'_>,
        digest: &SessionDigest,
    ) -> fmt::Result {
        let id_short = &digest.session_id[..8.min(digest.session_id.len())];
        let reason = digest.selection_reason.as_deref().unwrap_or("");

        writeln!(f, "Session {} ({}) -- {}", id_short, digest.source, reason)?;

        if let Some(opening) = &digest.opening {
            writeln!(f, "  Opening: \"{}\"", opening)?;
        }
        if let Some(activation) = &digest.activation {
            if digest.opening.as_ref() != Some(activation) {
                writeln!(f, "  Activation: \"{}\"", activation)?;
            }
        }
        Ok(())
    }

    fn render_compact(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for digest in &self.data.digests {
            self.render_digest_summary(f, digest)?;
            writeln!(f, "Work:")?;
            writeln!(f, "  (Session content rendering requires SessionViewModel)")?;
            writeln!(f)?;
        }
        Ok(())
    }

    fn render_diagnose(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "## Selected Sessions (Diagnose Mode)\n")?;

        let mut by_reason: HashMap<String, Vec<&SessionDigest>> = HashMap::new();
        for d in &self.data.digests {
            let key = d
                .selection_reason
                .as_deref()
                .unwrap_or("Other")
                .split(' ')
                .next()
                .unwrap_or("Other");
            by_reason.entry(key.to_string()).or_default().push(d);
        }

        for (category, list) in by_reason {
            writeln!(f, "### {}\n", category)?;
            for digest in list {
                self.render_digest_summary(f, digest)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }

    fn render_tools(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.render_compact(f)
    }
}

impl<'a> fmt::Display for PackReportView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "# Packing Report (pool: {} sessions from {} raw candidates)\n",
            self.data.pool_size, self.data.candidate_count
        )?;

        match self.data.template {
            ReportTemplate::Compact => self.render_compact(f),
            ReportTemplate::Diagnose => self.render_diagnose(f),
            ReportTemplate::Tools => self.render_tools(f),
        }
    }
}
