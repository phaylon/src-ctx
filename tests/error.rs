use src_ctx::{ContextError, normalize};
use test_util::{test_map, test_map_file};


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

#[test]
fn context_error_origins() {
    let (map, index) = test_map("abcdef");
    let input = map.input(index);
    let skipped = input.skip(3);

    let error_a = skipped.error("test-error", "test-note")
        .with_context(input.offset())
        .into_context_error(&map);
    assert_eq!(error_a.error(), &"test-error");

    let error_b = ContextError::from_origins("test-error", [
        map.context_error_origin(skipped.offset(), "test-note", Some(input.offset())),
    ]);
    assert_eq!(error_a, error_b);
}

#[test]
fn context_error_display_named() {
    let (map, index) = test_map("abc\ndef\nghi");
    let input = map.input(index);

    let error = input.skip(6).error("test-error", "test-note")
        .into_context_error(&map);
    assert_eq!(&format!("{error}"), "test-error in `test`, line 2, column 3");
    assert_eq!(&format!("{}", error.display_with_context()), &normalize("
        |error: test-error
        |--> `test`, line 2, column 3
        | 2 | def
        |   |   ^ test-note
    "));

    let error = input.skip(10).error("test-error", "test-note")
        .with_context(input.offset())
        .into_context_error(&map);
    assert_eq!(&format!("{}", error.display_with_context()), &normalize("
        |error: test-error
        |--> `test`, line 3, column 3
        | 1 | abc
        |   | ...
        | 3 | ghi
        |   |   ^ test-note
    "));

    let error = input.skip(10).error("test-error", "test-note")
        .with_context(input.skip(6).offset())
        .into_context_error(&map);
    assert_eq!(&format!("{}", error.display_with_context()), &normalize("
        |error: test-error
        |--> `test`, line 3, column 3
        | 2 | def
        | 3 | ghi
        |   |   ^ test-note
    "));
}

#[test]
fn context_error_display_file() {
    let (map, index) = test_map_file("abc\ndef\nghi");
    let input = map.input(index);

    let error = input.skip(6).error("test-error", "test-note")
        .into_context_error(&map);
    assert_eq!(&format!("{error}"), "test-error at test:2:3");
    assert_eq!(&format!("{}", error.display_with_context()), &normalize("
        |error: test-error
        |--> test:2:3
        | 2 | def
        |   |   ^ test-note
    "));

    let error = input.skip(10).error("test-error", "test-note")
        .with_context(input.offset())
        .into_context_error(&map);
    assert_eq!(&format!("{}", error.display_with_context()), &normalize("
        |error: test-error
        |--> test:3:3
        | 1 | abc
        |   | ...
        | 3 | ghi
        |   |   ^ test-note
    "));

    let error = input.skip(10).error("test-error", "test-note")
        .with_context(input.skip(6).offset())
        .into_context_error(&map);
    assert_eq!(&format!("{}", error.display_with_context()), &normalize("
        |error: test-error
        |--> test:3:3
        | 2 | def
        | 3 | ghi
        |   |   ^ test-note
    "));
}