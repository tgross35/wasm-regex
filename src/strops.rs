//! Our helpers for string-related tasks

use core::ops::Range;
use std::borrow::Cow;
use std::fmt::{Display, Write};
use std::str::{self};

use rustc_lexer::unescape::{unescape_str, EscapeError};
use serde::Serialize;

use crate::error::{Span, Unescape};

/// Return a sliced string if valid UTF8. Otherwise, replace invalid unicode with an escape
/// sequence (e.g. "this part is valid \x1f but that wasn't")
pub fn str_from_utf8_rep(text: &str, start: usize, end: usize) -> Cow<str> {
    let mut bslice = &text.as_bytes()[start..end];
    let mut utf8_res = str::from_utf8(bslice);

    // Short circuit: entire slice is valid UTF8
    if let Ok(s) = utf8_res {
        return Cow::Borrowed(s);
    }

    let mut ret = String::new();
    let mut is_first_loop = true;

    loop {
        if is_first_loop {
            // use existing result
            is_first_loop = false;
        } else {
            utf8_res = str::from_utf8(bslice);
        }

        // Exit if our entire string is valid
        if let Ok(s) = utf8_res {
            ret.push_str(s);
            break;
        }

        // At this point we have a utf8 error. So:
        // 1. Push the valid portion of the string
        let loop_err = utf8_res.unwrap_err();
        let valid_end = loop_err.valid_up_to();
        let err_len_res = loop_err.error_len();
        // This could be unsafe from_utf8_unchecked but we'll let the optimizer handle it
        ret.push_str(str::from_utf8(&bslice[..valid_end]).unwrap());
        bslice = &bslice[valid_end..];

        // 2. Push all invalid bytes formatted as "\xff"
        let invalid_end = err_len_res.map_or(bslice.len(), |elen| elen + valid_end);
        for byte in &bslice[..invalid_end] {
            write!(ret, r"\x{byte:02x}").unwrap();
        }

        // 3. Update our remaining slice for the next loop
        bslice = &bslice[invalid_end..];
    }

    Cow::Owned(ret)
}

/// Convert a single utf8 **byte** index to utf16
pub fn utf16_index_bytes(s: &str, i: usize) -> usize {
    s[..i].chars().map(char::len_utf16).sum()
}

/// Take a single utf8 **char** index and convert it to utf16
pub fn utf16_index_chars(s: &str, i: usize) -> usize {
    s.chars().take(i).map(char::len_utf16).sum()
}

/// Take an unsorted list of utf8 indices; sort them, update, and return a
/// map of `utf8_index->utf16_index`
///
/// Panics if an index is outside of the string
pub fn utf16_index_bytes_slice(s: &str, mut indices: Vec<usize>) -> Vec<(usize, usize)> {
    // Sort by first element
    indices.sort_unstable();
    indices.dedup();
    let mut ret: Vec<(usize, usize)> = Vec::with_capacity(indices.len());

    // running total of the u16 string's length
    let mut total_u16_offset = 0usize;
    // Our iterator over indices to match
    let mut indices_iter = indices.iter().copied();
    // Our iterator over characters that could provide a match for our index
    let mut char_iter = s
        .char_indices()
        .map(|(byte_idx, ch)| (byte_idx, ch.len_utf8(), ch.len_utf16()))
        .map(|(byte_idx, ch8_len, ch16_len)| {
            let ret = (byte_idx, ch8_len, total_u16_offset);
            total_u16_offset += ch16_len;
            ret
        });

    // If we find a match that's not exact (for non-utf8 matches), save it for
    // reuse here
    let mut residual_match: Option<(usize, usize)> = None;

    // Iterate through every index we need matched
    while let Some(idxu8) = indices_iter.next() {
        // Case 1: we are exactly at the end. Just consume the char iterator,
        // push the current offset map, and quit
        if idxu8 == s.len() {
            // This is the idiomatic way to consume an iterator
            char_iter.for_each(drop);
            ret.push((idxu8, total_u16_offset));
            break;
        }

        // Case 2: we have a stored value. This is used when we have an index
        // that is in between valid utf8 boundaries. Just push the cached value.
        if let Some((valid_until, last_u16_offset)) = residual_match {
            if idxu8 < valid_until {
                ret.push((idxu8, last_u16_offset));
                continue;
            }
        }

        // Case 3: We have a valid index and we can find it (=), or the next
        // valid index (>).
        let Some((byte_idx, ch8_len, u16_offset) )=
            char_iter.find(|(b_idx, _, _)| *b_idx >= idxu8) else {

            // Case 4: not found. If this is the case, we've hit the end of our
            // chars iterator. Just push the last known value for each remaining
            // index.
            ret.push((idxu8, total_u16_offset));
            indices_iter.for_each(|idxu8_inner| ret.push((idxu8_inner, total_u16_offset)));
            break;
        };

        ret.push((idxu8, u16_offset));

        // If we had a situation where we matched the next valid index instead
        // of our exact index, store that information for use in Case 1.
        if byte_idx > idxu8 {
            // If strictly greater, we will want to reuse this offset
            residual_match = Some((byte_idx + ch8_len, u16_offset));
        } else {
            residual_match = None;
        }
    }

    ret
}

///
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub enum StrType {
    /// No preprocessing
    #[default]
    Ignore,
    /// Escape as a str, `"string"`
    Str,
    /// Escape as a raw str, `r"string"`
    RawStr,
    /// Escape as a 1-hashed raw str, `r#"string"#`
    RawStrHash1,
    /// Escape as a 2-hashed raw str, `r##"string"##`
    RawStrHash2,
    RawStrHash3,
    RawStrHash4,
}

/// Give a singular noun description of the string type
impl Display for StrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrType::Ignore => write!(f, "unescaped"),
            StrType::Str => write!(f, "standard"),
            StrType::RawStr => write!(f, "raw"),
            StrType::RawStrHash1 => write!(f, "r#"),
            StrType::RawStrHash2 => write!(f, "r##"),
            StrType::RawStrHash3 => write!(f, "r###"),
            StrType::RawStrHash4 => write!(f, "r####"),
        }
    }
}

impl From<Option<&str>> for StrType {
    fn from(value: Option<&str>) -> Self {
        match value {
            None | Some("ignore") => Self::Ignore,
            Some("str") => Self::Str,
            Some("raw") => Self::RawStr,
            Some("rawhash1") => Self::RawStrHash1,
            Some("rawhash2") => Self::RawStrHash2,
            Some("rawhash3") => Self::RawStrHash3,
            Some("rawhash4") => Self::RawStrHash4,
            _ => panic!("unrecognized string type"),
        }
    }
}

/// Check for unescaped quotes
fn check_unescaped_quotes(s: &str) -> Result<(), Box<Unescape>> {
    // bad: `"`, `\\"`. ok: `\"`
    assert!(s.len() < usize::MAX - 5); // we leverage wrapping sub
    let bad_opt: Option<Range<usize>> = s.bytes().enumerate().find_map(|(idx, ch)| {
        // skip chars we don't care about
        if ch != b'"' {
            return None;
        }

        let errval = Some(idx..(idx + 1));
        // invalid unicode slice means no preceding slash
        let Some(tmp_slice) = s.get(..idx) else {
            return errval;
        };
        if tmp_slice.is_empty() {
            return errval;
        }

        // give us the index of the leftmost slash in our subslice
        let nonslash_idx = tmp_slice.rfind(|ch| ch != '\\').map_or(0, |v| v + 1);

        // odd number of slashes = ok (quote escaped), even = bad
        if (idx - nonslash_idx) % 2 == 1 {
            None
        } else {
            Some(idx..(idx + 1))
        }
    });
    let Some(bad_range) = bad_opt else {
        // no bad quotes, return OK
        return Ok(())
    };
    let (span, span_utf16) = Span::from_offsets(s, bad_range);
    let err = Unescape {
        message: String::from(r#"unescaped '"' in string"#),
        span,
        span_utf16,
        source: None,
    };
    Err(Box::new(err))
}

/// Actual implementation of `unescape`
fn unescape_impl(s: &str, sep: StrType) -> Result<Cow<str>, Box<Unescape>> {
    if matches!(sep, StrType::Ignore) {
        return Ok(Cow::Borrowed(s));
    }

    // quickcheck patterns that we know our string can't contain
    let check_pat: Option<&str> = match sep {
        StrType::Ignore => unreachable!(),
        // This would only really be `"` but not `\"`, can't check with .find
        StrType::Str => None,
        StrType::RawStr => Some("\""),
        StrType::RawStrHash1 => Some("\"#"),
        StrType::RawStrHash2 => Some("\"##"),
        StrType::RawStrHash3 => Some("\"###"),
        StrType::RawStrHash4 => Some("\"####"),
    };

    if let Some(pat) = check_pat {
        if let Some(idx) = s.find(pat) {
            // error, contains forbidden pattern
            return Err(Box::new(Unescape::from_pat(s, pat, idx, sep)));
        }
    }

    // no need for special behavior if we don't have any escapes
    if !matches!(sep, StrType::Str) || !s.contains('\\') {
        return Ok(Cow::Borrowed(s));
    }

    check_unescaped_quotes(s)?;

    // at this point, we have a `str` that needs unescaping
    let mut ret = String::with_capacity(s.len());
    let mut err: Option<(Range<usize>, EscapeError)> = None;
    unescape_str(s, &mut |range, unescaped_char| {
        if err.is_some() {
            return;
        };
        match unescaped_char {
            Ok(ch) => ret.push(ch),
            Err(e) => err = Some((range, e)),
        }
    });

    let Some(e) = err else {
        // no error, good to go!
        return Ok(Cow::Owned(ret))
    };

    Err(Box::new((s, e.0, e.1).into()))
}

/// Given an optional string type, unescape any `\` characters in a string
///
/// Signature is meant to be easy from js
pub fn unescape<'a>(s: &'a str, seperator: &Option<String>) -> Result<Cow<'a, str>, Box<Unescape>> {
    unescape_impl(s, seperator.as_deref().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape() {
        assert_eq!(unescape_impl("abc", StrType::Ignore).unwrap(), "abc");
        assert_eq!(unescape_impl("abc", StrType::Str).unwrap(), "abc");
        let err = unescape_impl(r"a😊\bc", StrType::Str).unwrap_err();
        assert_eq!(err.span.start.offset, 5);
        assert_eq!(err.span.end.offset, 7);
        assert_eq!(err.span_utf16.start.offset, 3);
        assert_eq!(err.span_utf16.end.offset, 5);
        assert_eq!(unescape_impl(r"a\nb", StrType::Str).unwrap(), "a\nb");
    }

    #[test]
    fn test_unescaped_quotes() {
        assert!(check_unescaped_quotes(r#"abcd"#).is_ok());
        assert!(check_unescaped_quotes(r#"ab\"cd"#).is_ok());
        assert!(check_unescaped_quotes(r#"\""#).is_ok());
        assert!(check_unescaped_quotes(r#"\"ab\"cd\""#).is_ok());
        assert!(check_unescaped_quotes(r#"ab\\cd"#).is_ok());
        assert!(check_unescaped_quotes(r#"ab\\\"cd"#).is_ok());
        assert!(check_unescaped_quotes(r#"ab"cd"#).is_err());
        assert!(check_unescaped_quotes(r#""abcd"#).is_err());
        assert!(check_unescaped_quotes(r#"abcd""#).is_err());
        assert!(check_unescaped_quotes(r#"ab\\"cd"#).is_err());
        assert!(check_unescaped_quotes(r#"ab\\\\"cd"#).is_err());
        assert!(check_unescaped_quotes(r#"""#).is_err());
        let test_sp = check_unescaped_quotes(r#"ab"cd"#).unwrap_err();
        assert_eq!(test_sp.span.start.offset, 2);
        assert_eq!(test_sp.span.end.offset, 3);
    }
}
