//! Utilities for validating string and char literals and turning them into
//! values they represent.

// **This file is reused from rustc_lexer at #897e37553bb relicensed by MIT**

use std::ops::Range;
// use std::str::Chars;

use super::Cursor;

/// Errors and warnings that can occur during string unescaping.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EscapeError {
    // /// Expected 1 char, but 0 were found.
    // ZeroChars,
    // /// Expected 1 char, but more than 1 were found.
    // MoreThanOneChar,
    /// Escaped '\' character without continuation.
    LoneSlash,
    /// Invalid escape character (e.g. '\z').
    InvalidEscape,
    /// Raw '\r' encountered.
    BareCarriageReturn,
    // /// Raw '\r' encountered in raw string.
    // BareCarriageReturnInRawString,
    /// Unescaped character that was expected to be escaped (e.g. raw '\t').
    EscapeOnlyChar,

    /// Numeric character escape is too short (e.g. '\x1').
    TooShortHexEscape,
    /// Invalid character in numeric escape (e.g. '\xz')
    InvalidCharInHexEscape,
    // /// Character code in numeric escape is non-ascii (e.g. '\xFF').
    // OutOfRangeHexEscape,
    /// '\u' not followed by '{'.
    NoBraceInUnicodeEscape,
    /// Non-hexadecimal value in '\u{..}'.
    InvalidCharInUnicodeEscape,
    /// '\u{}'
    EmptyUnicodeEscape,
    /// No closing brace in '\u{..}', e.g. '\u{12'.
    UnclosedUnicodeEscape,
    /// '\u{_12}'
    LeadingUnderscoreUnicodeEscape,
    /// More than 6 characters in '\u{..}', e.g. '\u{10FFFF_FF}'
    OverlongUnicodeEscape,
    /// Invalid in-bound unicode character code, e.g. '\u{DFFF}'.
    LoneSurrogateUnicodeEscape,
    /// Out of bounds unicode character code, e.g. '\u{FFFFFF}'.
    OutOfRangeUnicodeEscape,
    // /// Unicode escape code in byte literal.
    // UnicodeEscapeInByte,
    // /// Non-ascii character in byte literal.
    // NonAsciiCharInByte,
    // /// Non-ascii character in byte string literal.
    // NonAsciiCharInByteString,

    // /// After a line ending with '\', the next line contains whitespace
    // /// characters that are not skipped.
    // UnskippedWhitespaceWarning,

    // /// After a line ending with '\', multiple lines are skipped.
    // MultipleSkippedLinesWarning,
}

/// What kind of literal do we parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Single,
    Double,
}

fn scan_escape(cursor: &mut Cursor) -> Result<char, EscapeError> {
    // Previous character was '\\', unescape what follows.
    debug_assert_eq!(cursor.previous(), '\\');

    let second_char = cursor.consume().ok_or(EscapeError::LoneSlash)?;

    let res = match second_char {
        '"' => '"',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        '\'' => '\'',
        '0' => '\0',

        'x' => {
            // Parse hexadecimal character code.

            let hi = cursor.consume().ok_or(EscapeError::TooShortHexEscape)?;
            let hi = hi.to_digit(16).ok_or(EscapeError::InvalidCharInHexEscape)?;

            let lo = cursor.consume().ok_or(EscapeError::TooShortHexEscape)?;
            let lo = lo.to_digit(16).ok_or(EscapeError::InvalidCharInHexEscape)?;

            let value = hi * 16 + lo;

            // For a byte literal verify that it is within ASCII range.
            // if !mode.is_bytes() && !is_ascii(value) {
            //     return Err(EscapeError::OutOfRangeHexEscape);
            // }
            // let value = value as u8;

            value as u8 as char
        }

        'u' => {
            // We've parsed '\u', now we have to parse '{..}'.

            if cursor.consume() != Some('{') {
                return Err(EscapeError::NoBraceInUnicodeEscape);
            }

            // First character must be a hexadecimal digit.
            let mut n_digits = 1;
            let mut value: u32 = match cursor.consume().ok_or(EscapeError::UnclosedUnicodeEscape)? {
                '_' => return Err(EscapeError::LeadingUnderscoreUnicodeEscape),
                '}' => return Err(EscapeError::EmptyUnicodeEscape),
                c => c
                    .to_digit(16)
                    .ok_or(EscapeError::InvalidCharInUnicodeEscape)?,
            };

            // First character is valid, now parse the rest of the number
            // and closing brace.
            loop {
                match cursor.consume() {
                    None => return Err(EscapeError::UnclosedUnicodeEscape),
                    Some('_') => continue,
                    Some('}') => {
                        if n_digits > 6 {
                            return Err(EscapeError::OverlongUnicodeEscape);
                        }

                        // Incorrect syntax has higher priority for error reporting
                        // than unallowed value for a literal.
                        // if mode.is_bytes() {
                        //     return Err(EscapeError::UnicodeEscapeInByte);
                        // }

                        break std::char::from_u32(value).ok_or_else(|| {
                            if value > 0x10FFFF {
                                EscapeError::OutOfRangeUnicodeEscape
                            } else {
                                EscapeError::LoneSurrogateUnicodeEscape
                            }
                        })?;
                    }
                    Some(c) => {
                        let digit = c
                            .to_digit(16)
                            .ok_or(EscapeError::InvalidCharInUnicodeEscape)?;
                        n_digits += 1;
                        if n_digits > 6 {
                            // Stop updating value since we're sure that it's incorrect already.
                            continue;
                        }
                        // let digit = digit as u32;
                        value = value * 16 + digit;
                    }
                };
            }
        }
        _ => return Err(EscapeError::InvalidEscape),
    };
    Ok(res)
}

/// Takes a contents of a string literal (without quotes) and produces a
/// sequence of escaped characters or errors.
pub fn unescape_str<F>(mut cursor: Cursor, mode: Mode, callback: &mut F)
where
    F: FnMut(Range<usize>, Result<char, EscapeError>),
{
    let initial_len = cursor.as_str().len();
    while let Some(first_char) = cursor.consume() {
        let start = initial_len - cursor.as_str().len() - first_char.len_utf8();

        let unescaped_char = match first_char {
            '\\' => scan_escape(&mut cursor),
            '\n' => Ok('\n'),
            '\t' => Ok('\t'),
            '"' if mode == Mode::Double => Err(EscapeError::EscapeOnlyChar),
            '\'' if mode == Mode::Single => Err(EscapeError::EscapeOnlyChar),
            '\r' => Err(EscapeError::BareCarriageReturn),
            _ => Ok(first_char),
        };
        let end = initial_len - cursor.as_str().len();
        callback(start..end, unescaped_char);
    }
}

#[cfg(tset)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_char_bad() {
        fn check(literal_text: &str, expected_error: EscapeError) {
            let mut actual_results = vec![];
            unescape_str(
                Cursor::from(literal_text),
                Mode::Double,
                &mut |_range, actual_result| {
                    actual_results.push(actual_result);
                },
            );
            assert_eq!(actual_results[0], Err(expected_error));
        }

        // check("", EscapeError::ZeroChars);
        check(r"\", EscapeError::LoneSlash);

        // check("\n", EscapeError::EscapeOnlyChar);
        // check("\t", EscapeError::EscapeOnlyChar);
        // check("'", EscapeError::EscapeOnlyChar);
        check("\r", EscapeError::BareCarriageReturn);

        // check("spam", EscapeError::MoreThanOneChar);
        // check(r"\x0ff", EscapeError::MoreThanOneChar);
        // check(r#"\"a"#, EscapeError::MoreThanOneChar);
        // check(r"\na", EscapeError::MoreThanOneChar);
        // check(r"\ra", EscapeError::MoreThanOneChar);
        // check(r"\ta", EscapeError::MoreThanOneChar);
        // check(r"\\a", EscapeError::MoreThanOneChar);
        // check(r"\'a", EscapeError::MoreThanOneChar);
        // check(r"\0a", EscapeError::MoreThanOneChar);
        // check(r"\u{0}x", EscapeError::MoreThanOneChar);
        // check(r"\u{1F63b}}", EscapeError::MoreThanOneChar);

        check(r"\v", EscapeError::InvalidEscape);
        check(r"\💩", EscapeError::InvalidEscape);
        check(r"\●", EscapeError::InvalidEscape);
        check("\\\r", EscapeError::InvalidEscape);

        check(r"\x", EscapeError::TooShortHexEscape);
        check(r"\x0", EscapeError::TooShortHexEscape);
        check(r"\xf", EscapeError::TooShortHexEscape);
        check(r"\xa", EscapeError::TooShortHexEscape);
        check(r"\xx", EscapeError::InvalidCharInHexEscape);
        check(r"\xы", EscapeError::InvalidCharInHexEscape);
        check(r"\x🦀", EscapeError::InvalidCharInHexEscape);
        check(r"\xtt", EscapeError::InvalidCharInHexEscape);
        // check(r"\xff", EscapeError::OutOfRangeHexEscape);
        // check(r"\xFF", EscapeError::OutOfRangeHexEscape);
        // check(r"\x80", EscapeError::OutOfRangeHexEscape);

        check(r"\u", EscapeError::NoBraceInUnicodeEscape);
        check(r"\u[0123]", EscapeError::NoBraceInUnicodeEscape);
        check(r"\u{0x}", EscapeError::InvalidCharInUnicodeEscape);
        check(r"\u{", EscapeError::UnclosedUnicodeEscape);
        check(r"\u{0000", EscapeError::UnclosedUnicodeEscape);
        check(r"\u{}", EscapeError::EmptyUnicodeEscape);
        check(r"\u{_0000}", EscapeError::LeadingUnderscoreUnicodeEscape);
        check(r"\u{0000000}", EscapeError::OverlongUnicodeEscape);
        check(r"\u{FFFFFF}", EscapeError::OutOfRangeUnicodeEscape);
        check(r"\u{ffffff}", EscapeError::OutOfRangeUnicodeEscape);
        check(r"\u{ffffff}", EscapeError::OutOfRangeUnicodeEscape);

        check(r"\u{DC00}", EscapeError::LoneSurrogateUnicodeEscape);
        check(r"\u{DDDD}", EscapeError::LoneSurrogateUnicodeEscape);
        check(r"\u{DFFF}", EscapeError::LoneSurrogateUnicodeEscape);

        check(r"\u{D800}", EscapeError::LoneSurrogateUnicodeEscape);
        check(r"\u{DAAA}", EscapeError::LoneSurrogateUnicodeEscape);
        check(r"\u{DBFF}", EscapeError::LoneSurrogateUnicodeEscape);
    }

    #[test]
    fn test_unescape_char_good() {
        fn check(literal_text: &str, expected_char: char) {
            unescape_str(
                Cursor::from(literal_text),
                Mode::Double,
                &mut |_range, actual_result| {
                    assert_eq!(actual_result, Ok(expected_char));
                },
            );
        }

        check("a", 'a');
        check("ы", 'ы');
        check("🦀", '🦀');

        check(r#"\""#, '"');
        check(r"\n", '\n');
        check(r"\r", '\r');
        check(r"\t", '\t');
        check(r"\\", '\\');
        check(r"\'", '\'');
        check(r"\0", '\0');

        check(r"\x00", '\0');
        check(r"\x5a", 'Z');
        check(r"\x5A", 'Z');
        check(r"\x7f", 127 as char);

        check(r"\u{0}", '\0');
        check(r"\u{000000}", '\0');
        check(r"\u{41}", 'A');
        check(r"\u{0041}", 'A');
        check(r"\u{00_41}", 'A');
        check(r"\u{4__1__}", 'A');
        check(r"\u{1F63b}", '😻');
    }

    #[test]
    fn test_unescape_str_warn() {
        fn check(literal: &str, expected: &[(Range<usize>, Result<char, EscapeError>)]) {
            let mut unescaped = Vec::with_capacity(literal.len());
            unescape_str(Cursor::from(literal), Mode::Double, &mut |range, res| {
                unescaped.push((range, res))
            });
            assert_eq!(unescaped, expected);
        }

        // Check we can handle escaped newlines at the end of a file.
        // check("\\\n", &[]);
        // check("\\\n ", &[]);

        // check(
        //     "\\\n \u{a0} x",
        //     &[
        //         (0..5, Err(EscapeError::UnskippedWhitespaceWarning)),
        //         (3..5, Ok('\u{a0}')),
        //         (5..6, Ok(' ')),
        //         (6..7, Ok('x')),
        //     ],
        // );
        // check(
        //     "\\\n  \n  x",
        //     &[
        //         (0..7, Err(EscapeError::MultipleSkippedLinesWarning)),
        //         (7..8, Ok('x')),
        //     ],
        // );
    }

    #[test]
    fn test_unescape_str_good() {
        fn check(literal_text: &str, expected: &str) {
            let mut buf = Ok(String::with_capacity(literal_text.len()));
            unescape_str(Cursor::from(literal_text), Mode::Double, &mut |range, c| {
                if let Ok(b) = &mut buf {
                    match c {
                        Ok(c) => b.push(c),
                        Err(e) => buf = Err((range, e)),
                    }
                }
            });
            let buf = buf.as_ref().map(|it| it.as_ref());
            assert_eq!(buf, Ok(expected))
        }

        check("foo", "foo");
        check("", "");
        check(" \t\n", " \t\n");

        // check("hello \\\n     world", "hello world");
        check("thread's", "thread's")
    }
}
