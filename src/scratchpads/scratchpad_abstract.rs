pub trait CodeCompletionScratchpad {
    fn prompt(
        &self,
        context_size: usize,
    ) -> Result<String, String>;

    fn re_stream_response(
        &self,
    );
}
