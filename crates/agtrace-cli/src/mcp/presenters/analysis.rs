use crate::mcp::models::response::AnalysisViewModel;

pub fn present_analysis(report: agtrace_sdk::AnalysisReport) -> AnalysisViewModel {
    AnalysisViewModel(report)
}
