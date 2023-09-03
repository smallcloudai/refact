pub trait Scratchpad {
    fn prompt(
        &self,
        context_size: usize,
    );

    fn re_stream_response();
}
