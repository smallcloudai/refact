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
    fn start(&self) -> usize {
        0
    }

    #[inline]
    fn is_match(&self, &state: &usize) -> bool {
        state == self.pattern.len()
    }

    #[inline]
    fn can_match(&self, &_state: &usize) -> bool {
        self.pattern.len() > 0
    }

    #[inline]
    fn will_always_match(&self, &state: &usize) -> bool {
        state == self.pattern.len()
    }

    #[inline]
    fn accept(&self, &state: &usize, byte: u8) -> usize {
        if state == self.pattern.len() {
            return state;
        }

        if self.byte_match(byte, self.pattern[state]) {
            state + 1
        } else {
            0
        }
    }
}
