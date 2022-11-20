use std::str::Chars;

#[derive(Debug, Clone)]
pub struct Cursor<'i> {
    input: &'i str,
    chars: Chars<'i>,
}

impl<'i> From<&'i str> for Cursor<'i> {
    fn from(input: &'i str) -> Self {
        Self {
            input,
            chars: input.chars(),
        }
    }
}

impl<'i> Cursor<'i> {
    pub fn new(input: &'i str, chars: Chars<'i>) -> Self {
        #[cfg(debug_assertions)]
        {
            let start = chars.as_str().as_ptr() as usize;
            let end = start + chars.as_str().len();
            let input_start = input.as_ptr() as usize;
            let input_end = input_start + input.len();
            assert!(start >= input_start);
            assert!(end <= input_end);
        }

        Self { input, chars }
    }

    pub fn focus(&self, chars: Chars<'i>) -> Self {
        Self::new(self.input, chars)
    }

    pub fn input(&self) -> &'i str {
        self.input
    }

    pub fn chars(&self) -> Chars<'i> {
        self.chars.clone()
    }

    pub fn as_str(&self) -> &'i str {
        self.chars.as_str()
    }

    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

impl<'i> Cursor<'i> {
    pub fn position(&self) -> usize {
        self.as_str().as_ptr() as usize - self.input.as_ptr() as usize
    }

    pub fn previous(&self) -> char {
        self.input[0..self.position()]
            .chars()
            .next_back()
            .unwrap_or('\n')
    }

    pub fn first(&self) -> Option<char> {
        self.chars().next()
    }

    pub fn second(&self) -> Option<char> {
        let mut chars = self.chars();
        chars.next();
        chars.next()
    }
}
