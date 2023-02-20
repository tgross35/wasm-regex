//! All the messy-ish error handling code

use crate::utf16_index_bytes;
use crate::utf16_index_chars;
use regex_syntax::ast::Span as ReSpan;
use serde::Serialize;
use std::str;

/// Wrapper so we can serialize regex errors
#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
#[serde(tag = "errorClass", content = "error")]
pub enum Error {
    /// An error from regex
    RegexSyntax(Box<ReSyntax>),
    /// Regex compiled larger than the limit (unlikely, unless we set a limit)
    RegexCompiledTooBig(String),
    /// Unspecified error (very unlikely)
    RegexUnspecified(String),
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
            let (span_u8, span_u16) = make_spans(e.pattern(), e.span());
            let (aux_span_u8, aux_span_u16) = e
                .auxiliary_span()
                .map(|sp| make_spans(e.pattern(), sp))
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
            let (span_u8, span_u16) = make_spans(e.pattern(), e.span());
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
#[derive(Default, Debug, Serialize)]
struct Span {
    start: Position,
    end: Position,
}

/// Direct serializable map of `regex_syntax::ast::Position`
///
/// See: <https://docs.rs/regex-syntax/latest/regex_syntax/ast/struct.Position.html>
#[derive(Default, Debug, Serialize)]
struct Position {
    offset: usize,
    line: usize,
    column: usize,
}

/// Creates a utf8 span and a utf16 span
fn make_spans(s: &str, span: &ReSpan) -> (Span, Span) {
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
