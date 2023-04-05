use test_util::test_map;


mod test_util;

#[test]
fn inputs() {
    let (map, index) = test_map("abcdef");
    let input = map.input(index);

    assert_eq!(input.len(), 6);
    assert_eq!(input.content(), "abcdef");
    assert!(! input.is_empty());
    assert!(input.offset().is_at_start());

    let end = input.end();
    assert_eq!(end.len(), 0);
    assert_eq!(end.content(), "");
    assert!(end.is_empty());
    assert!(! end.offset().is_at_start());
    assert_eq!(end.offset().byte(), 6);

    let skip = input.skip(3);
    assert_eq!(skip.len(), 3);
    assert_eq!(skip.content(), "def");
    assert_eq!(skip.offset().byte(), 3);

    let trunc = input.truncate(3);
    assert_eq!(trunc.len(), 3);
    assert_eq!(trunc.content(), "abc");
    assert_eq!(trunc.offset().byte(), 0);

    let (left, right) = input.split(3);
    assert_eq!(left.content(), "abc");
    assert_eq!(left.offset().byte(), 0);
    assert_eq!(right.content(), "def");
    assert_eq!(right.offset().byte(), 3);

    assert_eq!(input.char(), Some('a'));
    assert_eq!(input.end().char(), None);

    let skip_char = input.skip_char('a').unwrap();
    assert_eq!(skip_char.content(), "bcdef");
    assert_eq!(skip_char.offset().byte(), 1);
    assert!(input.skip_char('X').is_none());
    assert!(input.end().skip_char('X').is_none());

    let (taken, take_char) = input.take_char().unwrap();
    assert_eq!(taken, 'a');
    assert_eq!(take_char.content(), "bcdef");
    assert_eq!(take_char.offset().byte(), 1);
    assert!(input.end().take_char().is_none());
}