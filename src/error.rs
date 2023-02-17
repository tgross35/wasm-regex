//! All the messy-ish error handling code

use serde::Serialize;
use std::str;

/// Wrapper so we can serialize regex errors
#[derive(Debug, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
#[serde(tag = "error_type", content = "error")]
pub enum Error {
    /// An error from regex
    RegexSyntax(Box<ReSyntaxError>),
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

/// Serializable wrapper for a regex error
///
/// Should represent both these types:
/// - <https://docs.rs/regex-syntax/latest/regex_syntax/ast/struct.Error.html>
/// - <https://docs.rs/regex-syntax/latest/regex_syntax/hir/struct.Error.html>
#[derive(Default, Debug, Serialize)]
pub struct ReSyntaxError {
    kind: String,
    pattern: String,
    span: Span,
    aux_span: Option<Span>,
}

impl From<regex_syntax::Error> for ReSyntaxError {
    fn from(value: regex_syntax::Error) -> Self {
        if let regex_syntax::Error::Parse(e) = value {
            Self {
                kind: e.kind().to_string(),
                pattern: e.pattern().to_owned(),
                span: e.span().into(),
                aux_span: e.auxiliary_span().map(|s| s.into()),
            }
        } else if let regex_syntax::Error::Translate(e) = value {
            Self {
                kind: e.kind().to_string(),
                pattern: e.pattern().to_owned(),
                span: e.span().into(),
                aux_span: None,
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

impl From<&regex_syntax::ast::Span> for Span {
    fn from(value: &regex_syntax::ast::Span) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

/// Direct serializable map of `regex_syntax::ast::Position`
#[derive(Default, Debug, Serialize)]
struct Position {
    offset: usize,
    line: usize,
    column: usize,
}

impl From<regex_syntax::ast::Position> for Position {
    fn from(value: regex_syntax::ast::Position) -> Self {
        Self {
            offset: value.offset,
            line: value.line,
            column: value.column,
        }
    }
}
