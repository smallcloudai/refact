use fst::Automaton;

#[derive(Clone, Debug)]
pub struct Substring<'a> {
    pattern: &'a [u8],
    case_insensitive: bool,
}

impl<'a> Substring<'a> {
    #[inline]
    pub fn new(substring: &'a str, case_insensitive: bool) -> Substring<'a> {
        Substring {
            pattern: substring.as_bytes(),
            case_insensitive,
        }
    }

    fn byte_match(&self, b1: u8, b2: u8) -> bool {
        if self.case_insensitive {
            b1.to_ascii_lowercase() == b2.to_ascii_lowercase()
        } else {
            b1 == b2
        }
    }
}

impl<'a> Automaton for Substring<'a> {
    type State = usize;

    #[inline]
    fn start(&self) -> Self::State {
        0
    }

    #[inline]
    fn is_match(&self, state: &Self::State) -> bool {
        *state >= self.pattern.len()
    }

    #[inline]
    fn can_match(&self, state: &Self::State) -> bool {
        true
    }

    #[inline]
    fn will_always_match(&self, state: &Self::State) -> bool {
        self.is_match(state)
    }

    #[inline]
    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        if state < &self.pattern.len() && self.byte_match(byte, self.pattern[*state]) {
            state + 1
        } else {
            *state
        }
    }
}
