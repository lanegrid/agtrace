pub use agtrace_engine::token_usage::{
    CacheCreationTokens, CacheReadTokens, ContextWindowUsage, FreshInputTokens, OutputTokens,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_window_usage_calculation() {
        let usage = ContextWindowUsage::from_raw(100, 200, 300, 50);

        assert_eq!(usage.input_tokens(), 600); // 100 + 200 + 300
        assert_eq!(usage.output_tokens(), 50);
        assert_eq!(usage.context_window_tokens(), 650); // 600 + 50
    }

    #[test]
    fn test_cache_read_always_included() {
        // This test documents that cache_read MUST be included
        // The type system makes it impossible to exclude
        let usage = ContextWindowUsage::from_raw(10, 20, 5000, 30);

        // cache_read (5000) is always part of the context window
        assert_eq!(usage.context_window_tokens(), 5060); // Not 60!
    }

    #[test]
    fn test_add_usage() {
        let usage1 = ContextWindowUsage::from_raw(100, 200, 300, 50);
        let usage2 = ContextWindowUsage::from_raw(10, 20, 30, 5);

        let total = usage1 + usage2;

        assert_eq!(total.fresh_input.0, 110);
        assert_eq!(total.cache_creation.0, 220);
        assert_eq!(total.cache_read.0, 330);
        assert_eq!(total.output.0, 55);
        assert_eq!(total.context_window_tokens(), 715);
    }

    #[test]
    fn test_default_is_empty() {
        let usage = ContextWindowUsage::default();
        assert!(usage.is_empty());
        assert_eq!(usage.context_window_tokens(), 0);
    }
}
