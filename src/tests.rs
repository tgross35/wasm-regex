// tests marked wasm_bindgen_test must be run with `wasm-pack test --node` (not `cargo test`)
use super::*;
use wasm_bindgen_test::*;

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

    let res = utf16_index_bytes_slice(TEST_S, input_vec);
    assert_eq!(expected_vec, res);
}

// Test on non-utf8 boundaries
#[test]
fn test_u16_slice_nonutf8_endemoji() {
    let s = "xx😀";
    let input: Vec<usize> = (0..=s.len()).collect();
    let expected = vec![(0, 0), (1, 1), (2, 2), (3, 4), (4, 4), (5, 4), (6, 4)];
    let res = utf16_index_bytes_slice(s, input);
    assert_eq!(expected, res);
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
    let res = utf16_index_bytes_slice(s, input);
    assert_eq!(expected, res);
}

#[test]
fn test_u16_slice_nonutf8_startemoji() {
    let s = "😀xx";
    let input: Vec<usize> = (0..=s.len()).collect();
    let expected = vec![(0, 0), (1, 2), (2, 2), (3, 2), (4, 2), (5, 3), (6, 4)];
    let res = utf16_index_bytes_slice(s, input);
    assert_eq!(expected, res);
}

// #[wasm_bindgen_test]
// fn test_find_unicode() {
//     let res = re_find("😃", ".", "");
//     // dbg!(&res);
//     assert_eq!(res, "1234: end");
//     assert_eq!(res.as_string().unwrap(), "1234: end");
// }

#[wasm_bindgen_test]
fn test_replace() {
    let res = re_replace("test 1234 end", r#"test (?P<cap>\d+)\s?"#, "$cap: ", "");
    assert_eq!(res.as_string().unwrap(), "1234: end");
}

#[wasm_bindgen_test]
fn test_replace_list() {
    let res = re_replace_list("foo bar!", r#"\w+"#, "$0\n", "g");
    assert_eq!(res.as_string().unwrap(), "foo\nbar\n");
}
