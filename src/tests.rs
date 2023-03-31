use std::borrow::Cow;

use js_sys::JSON;
use pretty_assertions::assert_eq;
use wasm_bindgen_test::*;

// tests marked wasm_bindgen_test must be run with `wasm-pack test --node` (not `cargo test`)
use super::*;
use crate::strops::utf16_index_bytes;

/// UTF8 test string
const TEST_S: &str = "x😀🤣a🤩😛🏴‍☠️🤑";
/// u8 start, u8 end, u16 start, u16 end, substring
const TEST_IDX: &[(usize, usize, usize, usize, &str)] = &[
    (0, 1, 0, 1, "x"),
    (1, 5, 1, 3, "😀"),
    (5, 14, 3, 8, "🤣a🤩"),
    (18, 31, 10, 15, "🏴‍☠️"),
    (31, 35, 15, 17, "🤑"),
];

#[test]
fn test_u16_byte_index() {
    let str16: Vec<u16> = TEST_S.encode_utf16().collect();

    for (s8, e8, s16_ex, e16_ex, r8) in TEST_IDX.iter().copied() {
        let s16 = utf16_index_bytes(TEST_S, s8);
        let e16 = utf16_index_bytes(TEST_S, e8);
        let r16: Vec<u16> = r8.encode_utf16().collect();

        assert_eq!(s16, s16_ex);
        assert_eq!(e16, e16_ex);
        assert_eq!(&TEST_S[s8..e8], r8);
        assert_eq!(&str16[s16..e16], r16);
    }
}

#[test]
fn test_u16_slice() {
    let mut expected_vec: Vec<_> = TEST_IDX
        .iter()
        .copied()
        .map(|(s8, _, s16_ex, _, _)| (s8, s16_ex))
        .collect();
    let mut end_vec: Vec<_> = TEST_IDX
        .iter()
        .copied()
        .map(|(_, e8, _, e16_ex, _)| (e8, e16_ex))
        .collect();

    expected_vec.append(&mut end_vec);
    let input_vec: Vec<usize> = expected_vec.iter().map(|x| x.0).collect();

    expected_vec.sort_by_key(|v| v.0);
    expected_vec.dedup();

    let res = utf16_index_bytes_slice(TEST_S, input_vec.clone());
    assert_eq!(expected_vec, res);

    test_byte_slice_sparse(TEST_S, &input_vec, &expected_vec);
}

#[test]
fn test_u16_byte_slice_index_allemojis() {
    let s = "😀😃😄";
    let input = vec![0, 12];
    let expected = vec![(0, 0), (12, 6)];
    let map = utf16_index_bytes_slice(s, input.clone());
    assert_eq!(expected, map);

    test_byte_slice_sparse(s, &input, &expected);
}

// Test on non-utf8 boundaries
#[test]
fn test_u16_slice_nonutf8_endemoji() {
    let s = "xx😀";
    let input: Vec<usize> = (0..=s.len()).collect();
    let expected = vec![(0, 0), (1, 1), (2, 2), (3, 4), (4, 4), (5, 4), (6, 4)];
    let res = utf16_index_bytes_slice(s, input.clone());
    assert_eq!(expected, res);

    test_byte_slice_sparse(s, &input, &expected);
}

#[test]
fn test_u16_slice_nonutf8_enchar() {
    let s = "xx😀xx";
    let input: Vec<usize> = (0..=s.len()).collect();
    let expected = vec![
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 4),
        (4, 4),
        (5, 4),
        (6, 4),
        (7, 5),
        (8, 6),
    ];
    let res = utf16_index_bytes_slice(s, input.clone());
    assert_eq!(expected, res);

    test_byte_slice_sparse(s, &input, &expected);
}

#[test]
fn test_u16_slice_nonutf8_startemoji() {
    let s = "😀xx";
    let input: Vec<usize> = (0..=s.len()).collect();
    let expected = vec![(0, 0), (1, 2), (2, 2), (3, 2), (4, 2), (5, 3), (6, 4)];
    let res = utf16_index_bytes_slice(s, input.clone());
    assert_eq!(expected, res);

    test_byte_slice_sparse(s, &input, &expected);
}

#[test]
fn test_str_utf8_replace() {
    let s1 = "a😀b";
    let s2 = "😀";
    let expected: &[(usize, usize, &str, &str)] = &[
        (0, 1, s1, "a"),
        (0, 3, s1, r"a\xf0\x9f"),
        (1, 4, s1, r"\xf0\x9f\x98"),
        (2, 3, s1, r"\x9f"),
        (0, 1, s2, r"\xf0"),
        (0, 4, s2, r"😀"),
        (3, 4, s2, r"\x80"),
    ];

    for (start, end, test_str, res) in expected.iter() {
        eprintln!("testing \"{test_str}\"[{start}..{end}]");
        assert_eq!(&str_from_utf8_rep(test_str, *start, *end).as_ref(), res);
    }
}

#[wasm_bindgen_test]
fn test_find_unicode() {
    let s = "😃";
    let res = re_find(s, ".", "u", None, None);
    let expected = MatchSer {
        matches: vec![vec![CapSer {
            group_name: None,
            match_num: 0,
            group_num: 0,
            is_participating: true,
            entire_match: true,
            content: Some(Cow::Borrowed(s)),
            start_utf16: Some(0),
            start: Some(0),
            end_utf16: Some(2),
            end: Some(4),
        }]],
    }
    .to_js_value();

    assert_eq!(stringify(&res), stringify(&expected));
}

#[wasm_bindgen_test]
fn test_find_indices() {
    let s = "😀😃😄";
    let res = re_find(s, ".*", "u", None, None);
    let expected = MatchSer {
        matches: vec![vec![CapSer {
            group_name: None,
            match_num: 0,
            group_num: 0,
            is_participating: true,
            entire_match: true,
            content: Some(Cow::Borrowed(s)),
            start_utf16: Some(0),
            start: Some(0),
            end_utf16: Some(6),
            end: Some(12),
        }]],
    }
    .to_js_value();

    assert_eq!(stringify(&res), stringify(&expected));
}

#[wasm_bindgen_test]
fn test_find_invalid_utf8() {
    // test without unicode flag
    let s = "a😀a";
    let res = re_find(s, "..", "g", None, None);
    let expected = MatchSer {
        matches: vec![
            vec![CapSer {
                group_name: None,
                match_num: 0,
                group_num: 0,
                is_participating: true,
                entire_match: true,
                content: Some(Cow::Borrowed(r"a\xf0")),
                start_utf16: Some(0),
                start: Some(0),
                end_utf16: Some(3),
                end: Some(2),
            }],
            vec![CapSer {
                group_name: None,
                match_num: 1,
                group_num: 0,
                is_participating: true,
                entire_match: true,
                content: Some(Cow::Borrowed(r"\x9f\x98")),
                start_utf16: Some(3),
                start: Some(2),
                end_utf16: Some(3),
                end: Some(4),
            }],
            vec![CapSer {
                group_name: None,
                match_num: 2,
                group_num: 0,
                is_participating: true,
                entire_match: true,
                content: Some(Cow::Borrowed(r"\x80a")),
                start_utf16: Some(3),
                start: Some(4),
                end_utf16: Some(4),
                end: Some(6),
            }],
        ],
    }
    .to_js_value();

    assert_eq!(stringify(&res), stringify(&expected));
}

#[wasm_bindgen_test]
fn test_replace() {
    let res = re_replace(
        "test 1234 end",
        r#"test (?P<cap>\d+)\s?"#,
        "$cap: ",
        "",
        None,
        None,
        None,
    );
    let expected = ReplacdSer {
        result: "1234: end",
    }
    .to_js_value();

    assert_eq!(stringify(&res), stringify(&expected));
}

#[wasm_bindgen_test]
fn test_replace_list() {
    let res = re_replace_list("foo bar!", r#"\w+"#, "$0\n", "g", None, None, None);
    let expected = ReplacdSer {
        result: "foo\nbar\n",
    }
    .to_js_value();

    assert_eq!(stringify(&res), stringify(&expected));
}

/* helpers */

/// Given an input vector and an expected vector, test first, last, and middle
/// items separately. This helps fuzz errors with char counting
fn test_byte_slice_sparse(s: &str, input: &[usize], expected: &[(usize, usize)]) {
    let mut in_srt: Vec<_> = input.to_vec();
    let mut ex_srt: Vec<_> = expected.to_vec();
    in_srt.sort_unstable();
    in_srt.dedup();
    ex_srt.sort_by_key(|k| k.0);
    ex_srt.dedup_by_key(|k| k.0);

    let len = in_srt.len();
    let to_test = [
        (
            vec![*in_srt.first().unwrap()],
            vec![*ex_srt.first().unwrap()],
        ),
        (vec![*in_srt.last().unwrap()], vec![*ex_srt.last().unwrap()]),
        (
            vec![*in_srt.first().unwrap(), *in_srt.last().unwrap()],
            vec![*ex_srt.first().unwrap(), *ex_srt.last().unwrap()],
        ),
        (vec![in_srt[len / 2]], vec![ex_srt[len / 2]]),
    ];

    for (in_t, ex_t) in to_test {
        let fail_msg = format!(
            "\nfailed at input: {input:?}\nexpected: {expected:?}\ntesting: ({in_t:?}, {ex_t:?})"
        );
        let res = utf16_index_bytes_slice(s, in_t);
        assert_eq!(ex_t, res, "{}", fail_msg);
    }
}

// wrap JSON::stringify but use pretty printinf
fn stringify(obj: &JsValue) -> String {
    JSON::stringify_with_replacer_and_space(obj, &JsValue::NULL, &2.into())
        .unwrap()
        .as_string()
        .unwrap()
        .replace("\\n", "\n")
}
