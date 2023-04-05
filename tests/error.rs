use test_util::test_map;


mod test_util;

#[test]
fn source_errors() {
    let (map, index) = test_map("abcdef");
    let input = map.input(index);
    let skipped = input.skip(3);

    let error = skipped.error("test-error", "test-note");
    assert_eq!(error.error(), &"test-error");
    assert_eq!(error.note(), "test-note");
    assert_eq!(error.offset(), skipped.offset());
    assert!(error.context_offset().is_none());

    let error = error.with_context(input.offset());
    assert_eq!(error.context_offset(), Some(input.offset()));

    let error = error.map(|error| format!("~{error}~"));
    assert_eq!(error.error(), &String::from("~test-error~"));
}