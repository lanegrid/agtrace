pub fn build_progress_bar(current: u32, max: u32, width: usize) -> String {
    if max == 0 {
        return format!("[{}] 0.0%", "░".repeat(width));
    }

    let percent = (current as f64 / max as f64) * 100.0;
    let filled = ((percent / 100.0) * width as f64) as usize;
    let filled = filled.min(width);
    let empty = width - filled;

    format!(
        "[{}{}] {:.1}%",
        "█".repeat(filled),
        "░".repeat(empty),
        percent
    )
}
