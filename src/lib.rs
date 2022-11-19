use regex::{Regex, RegexBuilder};
use wasm_bindgen::prelude::*;

/// Process default flags to create a usable regex
fn re_build(reg_exp: &str, flags: &str) -> Regex {
    let mut builder = RegexBuilder::new(reg_exp);
    let mut tmp_build = &mut builder;

    if flags.contains('i') {
        tmp_build = tmp_build.case_insensitive(true);
    }
    if flags.contains('m') {
        tmp_build = tmp_build.multi_line(true)
    }
    if flags.contains('s') {
        tmp_build = tmp_build.dot_matches_new_line(true)
    }
    if flags.contains('U') {
        tmp_build = tmp_build.swap_greed(true)
    }
    if flags.contains('u') {
        // Unicode is enabled by default, `u` disables
        tmp_build = tmp_build.unicode(false)
    }
    if flags.contains('x') {
        tmp_build = tmp_build.ignore_whitespace(true)
    }

    tmp_build.build().expect("failed to build regex")
}

/// Run a regular expression on a block of text, returning a JSON string
///
/// # Arguments
///
/// - `flags`: apply global flags, options `imsUux`
/// - `text`: haystack to search in
/// - `reg_exp`: regular expression to match against
///
/// Returns something like:
///
/// ```json5
/// {
///     "matches": [
///         [
///             {
///                 "content": "match content",
///                 "start": 10,
///                 "end": 15
///             }
///             // ... further capturing groups within the match
///         ]
///         // Further matches within text
///     ]
/// }
/// ```
#[wasm_bindgen]
pub fn re_find(text: &str, reg_exp: &str, flags: &str) -> String {
    let mut out = r#"{"matches":["#.to_owned();

    let re = re_build(reg_exp, flags);
    for match_cap in re.captures_iter(text) {
        //
        out.push('[');

        for cap in match_cap.iter() {
            // Single capture group within a match
            out.push('{');

            if let Some(m) = cap {
                let match_fmt = format!(
                    "\"content\":\"{}\",\"start\":{},\"end\":{}",
                    m.as_str(),
                    m.start(),
                    m.end()
                );
                out.push_str(&match_fmt);
            } else {
                out.push_str(r#""content":null,"start":null,"end":null"#);
            }

            out.push_str("},");
        }

        out.pop(); // remove final comma
        out.push_str("],");
    }

    out.pop(); // remove final comma
    out.push_str("]}");

    out
}

/// Perform a regex replacement on a provided string
#[wasm_bindgen]
pub fn re_replace(text: &str, reg_exp: &str, rep: &str, flags: &str) -> String {
    let re = re_build(reg_exp, flags);
    re.replace(text, rep).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace() {
        let res = re_replace("test 1234 end", r#"test (?P<cap>\d+)\s?"#, "$cap: ", "");
        assert_eq!(res, "1234: end");
    }
}
