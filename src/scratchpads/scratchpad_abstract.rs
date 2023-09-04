pub trait CodeCompletionScratchpad {
    fn prompt(
        &self,
        context_size: i32,
    );

    fn re_stream_response(
        &self,
    );
}
