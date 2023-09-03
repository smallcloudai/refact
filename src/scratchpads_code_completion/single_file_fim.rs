use crate::scratchpad_abstract::Scratchpad;

pub struct SingleFileFIM;

impl SingleFileFIM {
    pub fn new() -> Self {
        SingleFileFIM
    }
}

impl Scratchpad for SingleFileFIM {
    fn prompt(
        &self,
        context_size: usize,
    ) {
        println!("This method is overridden in the derived class T={}", context_size);
    }

    fn re_stream_response()
    {
    }
}
