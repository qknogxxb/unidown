use std::ops::{Deref, DerefMut};
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

impl<'i> Cursor<'i> {
    pub fn consume(&mut self) -> Option<char> {
        self.chars.next()
    }

    pub fn consume_with(&mut self, mut func: impl FnMut(&mut Cursor<'i>)) -> &mut Self {
        func(self);
        self
    }

    pub fn consume_while(&mut self, mut predicate: impl FnMut(char) -> bool) -> &mut Self {
        self.consume_with(|cursor| {
            for ch in cursor.chars() {
                if predicate(ch) {
                    cursor.consume();
                } else {
                    break;
                }
            }
        })
    }

    pub fn consume_until(&mut self, mut predicate: impl FnMut(char) -> bool) -> &mut Self {
        self.consume_with(|cursor| {
            for ch in cursor.chars() {
                cursor.consume();

                if predicate(ch) {
                    break;
                }
            }
        })
    }
}

impl<'i> Cursor<'i> {
    pub fn consume_line(&mut self) -> &mut Self {
        self.consume_until(|ch| ch == '\n')
    }

    pub fn consume_lines_while(&mut self, mut predicate: impl FnMut(&'i str) -> bool) -> &mut Self {
        self.consume_with(|cursor| {
            for next_line in cursor.chars().as_str().lines() {
                if predicate(next_line) {
                    cursor.consume_line();
                } else {
                    break;
                }
            }
        })
    }

    pub fn consume_lines_until(&mut self, mut predicate: impl FnMut(&'i str) -> bool) -> &mut Self {
        self.consume_with(|cursor| {
            for next_line in cursor.chars().as_str().lines() {
                cursor.consume_line();

                if predicate(next_line) {
                    break;
                }
            }
        })
    }
}

impl<'i> Cursor<'i> {
    pub fn focus_with(&mut self, mut func: impl FnMut(&mut Cursor<'i>)) -> Self {
        let start = self.position();
        func(self);
        let end = self.position();
        Self::new(self.input, self.input[start..end].chars())
    }

    pub fn focus_char(&mut self) -> Self {
        self.focus_with(|cursor| {
            cursor.consume();
        })
    }

    pub fn focus_line(&mut self) -> Self {
        self.focus_with(|cursor| {
            cursor.consume_line();
        })
    }

    pub fn focus_while(&mut self, mut predicate: impl FnMut(char) -> bool) -> Self {
        self.focus_with(|cursor| {
            cursor.consume_while(&mut predicate);
        })
    }

    pub fn focus_until(&mut self, mut predicate: impl FnMut(char) -> bool) -> Self {
        self.focus_with(|cursor| {
            cursor.consume_until(&mut predicate);
        })
    }

    pub fn focus_lines_while(&mut self, mut predicate: impl FnMut(&'i str) -> bool) -> Self {
        self.focus_with(|cursor| {
            cursor.consume_lines_while(&mut predicate);
        })
    }

    pub fn focus_lines_until(&mut self, mut predicate: impl FnMut(&'i str) -> bool) -> Self {
        self.focus_with(|cursor| {
            cursor.consume_lines_until(&mut predicate);
        })
    }
}

#[derive(Debug, Clone)]
pub struct Span<'i, Kind: 'i> {
    pub kind: Kind,
    pub cursor: Cursor<'i>,
}

impl<'i, Kind: 'i> Span<'i, Kind> {
    pub fn new(kind: Kind, cursor: Cursor<'i>) -> Self {
        Self { kind, cursor }
    }

    pub fn to_kind<OtherKind: 'i>(&self, other_kind: OtherKind) -> Span<'i, OtherKind> {
        Span::new(other_kind, self.cursor.clone())
    }
}

impl<'i, Kind: 'i> Deref for Span<'i, Kind> {
    type Target = Cursor<'i>;

    fn deref(&self) -> &Self::Target {
        &self.cursor
    }
}

impl<'i, Kind: 'i> DerefMut for Span<'i, Kind> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cursor
    }
}
