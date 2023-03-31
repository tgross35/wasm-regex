//! All the messy-ish error handling code

use std::ops::Range;
use std::str;

use regex_syntax::ast::Span as ReSpan;
use rustc_lexer::unescape::EscapeError;
use serde::Serialize;

use crate::strops::{utf16_index_bytes, utf16_index_chars, StrType};

/// Wrapper so we can serialize regex errors
#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
#[serde(tag = "errorClass", content = "error")]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    /// An error from regex
    RegexSyntax(Box<ReSyntax>),
    /// Regex compiled larger than the limit (unlikely, unless we set a limit)
    RegexCompiledTooBig(String),
    /// Unspecified error (very unlikely)
    RegexUnspecified(String),
    /// Error with an input string. The second argument indicates which
    Unescape(Box<Unescape>),
}

/// Add automatic conversion from regex error to our error type
impl From<regex::Error> for Error {
    fn from(value: regex::Error) -> Self {
        let err_string = value.to_string();
        match value {
            // This should be unreachable because our builder checked the syntax
            // already
            regex::Error::Syntax(_) => unreachable!(),
            regex::Error::CompiledTooBig(_) => Self::RegexCompiledTooBig(err_string),
            _ => Self::RegexUnspecified(err_string),
        }
    }
}

/// Automatic conversion from
impl From<regex_syntax::Error> for Error {
    fn from(value: regex_syntax::Error) -> Self {
        Self::RegexSyntax(Box::new(value.into()))
    }
}

impl From<(Unescape, &'static str)> for Error {
    fn from(value: (Unescape, &'static str)) -> Self {
        let mut err = value.0;
        err.source = Some(value.1);
        Self::Unescape(Box::new(err))
    }
}

impl From<(Box<Unescape>, &'static str)> for Error {
    fn from(value: (Box<Unescape>, &'static str)) -> Self {
        let mut err = value.0;
        err.source = Some(value.1);
        Self::Unescape(err)
    }
}

/// Serializable wrapper for a regex syntax error
///
/// Should represent both these types:
/// - <https://docs.rs/regex-syntax/latest/regex_syntax/ast/struct.Error.html>
/// - <https://docs.rs/regex-syntax/latest/regex_syntax/hir/struct.Error.html>
#[derive(Default, Debug, Serialize)]
pub struct ReSyntax {
    /// Debug representation of the syntax error type
    kind: String,
    /// Display
    message: String,
    /// Pattern that caused the error
    pattern: String,
    /// Location of the error
    span: Span,
    /// Location of the error with js offsets
    span_utf16: Span,
    /// If applicable, second location of the error (e.g. for duplicates)
    auxiliary_span: Option<Span>,
    /// Auxiliary span with js offsets
    auxiliary_span_utf16: Option<Span>,
}

/// Convert regex syntax errors into our common error type
impl From<regex_syntax::Error> for ReSyntax {
    fn from(value: regex_syntax::Error) -> Self {
        if let regex_syntax::Error::Parse(e) = value {
            let (span_u8, span_u16) = convert_re_spans(e.pattern(), e.span());
            let (aux_span_u8, aux_span_u16) = e
                .auxiliary_span()
                .map(|sp| convert_re_spans(e.pattern(), sp))
                .unzip();
            // AST error
            Self {
                kind: format!("{:?}", e.kind()),
                message: e.kind().to_string(),
                pattern: e.pattern().to_owned(),
                span: span_u8,
                span_utf16: span_u16,
                auxiliary_span: aux_span_u8,
                auxiliary_span_utf16: aux_span_u16,
            }
        } else if let regex_syntax::Error::Translate(e) = value {
            let (span_u8, span_u16) = convert_re_spans(e.pattern(), e.span());
            // HIR error
            Self {
                kind: format!("{:?}", e.kind()),
                message: e.kind().to_string(),
                pattern: e.pattern().to_owned(),
                span: span_u8,
                span_utf16: span_u16,
                auxiliary_span: None,
                auxiliary_span_utf16: None,
            }
        } else {
            Self {
                kind: "unspecified error".to_owned(),
                ..Self::default()
            }
        }
    }
}

/// Direct serializable map of `regex_syntax::ast::Span`
#[derive(Default, Debug, PartialEq, Serialize)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    #[allow(unused)]
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Returns a utf8 and utf16 span
    pub fn from_offsets(s: &str, range: Range<usize>) -> (Self, Self) {
        assert!(range.start < range.end);
        let (start_u8, start_u16) = Position::from_offset(s, range.start);
        let (mut end_u8, mut end_u16) = Position::from_offset(s, range.end);
        end_u8.increment_line();
        end_u16.increment_line();
        (
            Self {
                start: start_u8,
                end: end_u8,
            },
            Self {
                start: start_u16,
                end: end_u16,
            },
        )
    }
}

/// Direct serializable map of `regex_syntax::ast::Position`
///
/// See: <https://docs.rs/regex-syntax/latest/regex_syntax/ast/struct.Position.html>
#[derive(Default, Debug, PartialEq, Serialize)]
pub struct Position {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

impl Position {
    fn new(offset: usize, line: usize, column: usize) -> Self {
        Self {
            offset,
            line,
            column,
        }
    }

    /// Return utf8 and utf16 positions from a single utf8 byte index. Somewhat
    /// inefficient algorithm, but simple
    fn from_offset(s: &str, offset: usize) -> (Self, Self) {
        let mut line = 1;
        let newline_idx = s[..offset]
            .bytes()
            .enumerate()
            .filter_map(|(i, b)| {
                if b == b'\n' {
                    line += 1;
                    Some(i)
                } else {
                    None
                }
            })
            .last()
            .map_or(0, |v| v + 1);

        let col_u8 = offset - newline_idx;
        let col_u16 = utf16_index_bytes(&s[newline_idx..], offset - newline_idx);
        let offset_u16 = utf16_index_bytes(s, offset);

        (
            Self::new(offset, line, col_u8),
            Self::new(offset_u16, line, col_u16),
        )
    }

    /// We kind commonly need to bump this to make it a proper range
    fn increment_line(&mut self) {
        self.line += 1;
    }
}

/// Creates a utf8 span and a utf16 span
fn convert_re_spans(s: &str, span: &ReSpan) -> (Span, Span) {
    let off16_start = utf16_index_bytes(s, span.start.offset);
    let off16_end = utf16_index_bytes(s, span.end.offset);

    // Need to recalculate char offset within the line
    let start_line = s.lines().nth(span.start.line - 1).unwrap();
    let end_line = s.lines().nth(span.end.line - 1).unwrap();

    let col16_start = utf16_index_chars(start_line, span.start.column - 1) + 1;
    let col16_end = utf16_index_chars(end_line, span.end.column - 1) + 1;

    let span_u8 = Span {
        start: Position {
            offset: span.start.offset,
            line: span.start.line,
            column: span.start.column,
        },
        end: Position {
            offset: span.end.offset,
            line: span.end.line,
            column: span.end.column,
        },
    };
    let span_u16 = Span {
        start: Position {
            offset: off16_start,
            line: span.start.line,
            column: col16_start,
        },
        end: Position {
            offset: off16_end,
            line: span.end.line,
            column: col16_end,
        },
    };

    (span_u8, span_u16)
}

/// Error while unescaping a string
#[derive(Default, Debug, Serialize)]
pub struct Unescape {
    /// Debug representation of the syntax error type,
    pub message: String,
    /// Location of the error
    pub span: Span,
    /// Location of the error with js offsets
    pub span_utf16: Span,
    /// Where this error came from
    pub source: Option<&'static str>,
}

impl Unescape {
    /// Create a pattern error
    pub fn from_pat(s: &str, pat: &str, idx: usize, type_: StrType) -> Self {
        let (span, span_utf16) = Span::from_offsets(s, idx..(idx + pat.len()));
        Self {
            message: format!("pattern '{pat}' may not be contained in {type_} strings"),
            span,
            span_utf16,
            source: None,
        }
    }
}

fn escape_error_message(err: &EscapeError) -> String {
    match err {
        EscapeError::LoneSlash => r"escaped '\' character without continuation",
        EscapeError::InvalidEscape => "invalid escape character",
        EscapeError::BareCarriageReturn | EscapeError::BareCarriageReturnInRawString => {
            "raw '\r' encountered"
        }
        EscapeError::EscapeOnlyChar => "unescaped character that was expected to be escaped",
        EscapeError::TooShortHexEscape => "numeric character escape is too short",
        EscapeError::InvalidCharInHexEscape => "invalid character in numeric escape",
        EscapeError::OutOfRangeHexEscape => "character code in numeric escape is non-ascii",
        EscapeError::NoBraceInUnicodeEscape => {
            r"no brace in unicode escape: '\u' not followed by '{'"
        }
        EscapeError::InvalidCharInUnicodeEscape => r"non-hexadecimal value in '\u{..}'",
        EscapeError::EmptyUnicodeEscape => "empty unicode escape",
        EscapeError::UnclosedUnicodeEscape => r"no closing brace in '\u{..}'",
        EscapeError::LeadingUnderscoreUnicodeEscape => "leading underscore unicode escape",
        EscapeError::OverlongUnicodeEscape => r"more than 6 characters in '\u{..}'",
        EscapeError::LoneSurrogateUnicodeEscape => {
            "lone surrogate in unicode escape: invalid in-bound unicode character code"
        }
        EscapeError::OutOfRangeUnicodeEscape => "out of bounds unicode character code",
        // bytestring and char parsing errors that we don't use
        EscapeError::NonAsciiCharInByte
        | EscapeError::NonAsciiCharInByteString
        | EscapeError::UnicodeEscapeInByte
        | EscapeError::MoreThanOneChar
        | EscapeError::ZeroChars => unreachable!(),
    }
    .to_owned()
}

impl From<(&str, Range<usize>, EscapeError)> for Unescape {
    fn from(value: (&str, Range<usize>, EscapeError)) -> Self {
        let (s, range, err) = value;
        let (span, span_utf16) = Span::from_offsets(s, range);
        let message = escape_error_message(&err);
        Self {
            message,
            span,
            span_utf16,
            source: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_span_offset() {
        let s = "abc😊\ndef";
        assert_eq!(
            Span::from_offsets(s, 0..1),
            (make_span(0..1, 1..2, 0..1), make_span(0..1, 1..2, 0..1))
        );
        assert_eq!(
            Span::from_offsets(s, 2..9),
            (make_span(2..9, 1..3, 2..1), make_span(2..7, 1..3, 2..1))
        );
    }

    fn make_span(offset: Range<usize>, line: Range<usize>, column: Range<usize>) -> Span {
        Span::new(
            Position::new(offset.start, line.start, column.start),
            Position::new(offset.end, line.end, column.end),
        )
    }
}
