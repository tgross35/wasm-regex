//! All the messy-ish error handling code

use crate::utf16_index_bytes;
use crate::utf16_index_chars;
use regex_syntax::ast::Position as RePosition;
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
    /// Some sort of invalid replacement
    Encoding(String),
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

/// Automatic conversion from string utf8 error to our error type. If a result
/// somehow returns non-valid UTF8/UTF16, this will fire
impl From<std::str::Utf8Error> for Error {
    fn from(value: str::Utf8Error) -> Self {
        Self::Encoding(value.to_string())
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
    /// If applicable, second location of the error (e.g. for duplicates)
    auxiliary_span: Option<Span>,
}

/// Convert regex syntax errors into our common error type
impl From<regex_syntax::Error> for ReSyntax {
    fn from(value: regex_syntax::Error) -> Self {
        if let regex_syntax::Error::Parse(e) = value {
            // AST error
            Self {
                kind: format!("{:?}", e.kind()),
                message: e.kind().to_string(),
                pattern: e.pattern().to_owned(),
                span: make_span(e.pattern(), e.span()),
                auxiliary_span: e.auxiliary_span().map(|sp| make_span(e.pattern(), sp)),
            }
        } else if let regex_syntax::Error::Translate(e) = value {
            // HIR error
            Self {
                kind: format!("{:?}", e.kind()),
                message: e.kind().to_string(),
                pattern: e.pattern().to_owned(),
                span: make_span(e.pattern(), e.span()),
                auxiliary_span: None,
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

/// Create our Span from a regex Span, converting utf8 indices to utf16
fn make_span(s: &str, span: &ReSpan) -> Span {
    let RePosition {
        offset: o8s,
        line: l8s,
        column: c8s,
    } = span.start;
    let RePosition {
        offset: o8e,
        line: l8e,
        column: c8e,
    } = span.end;

    let o16s = utf16_index_bytes(s, o8s);
    let o16e = utf16_index_bytes(s, o8e);

    // Need to recalculate char offset within the line
    let line_start = s.lines().nth(l8s - 1).unwrap();
    let line_end = s.lines().nth(l8e - 1).unwrap();

    let c16s = utf16_index_chars(line_start, c8s - 1) + 1;
    let c16e = utf16_index_chars(line_end, c8e - 1) + 1;

    Span {
        start: Position {
            offset: o16s,
            line: l8s,
            column: c16s,
        },
        end: Position {
            offset: o16e,
            line: l8e,
            column: c16e,
        },
    }
}
