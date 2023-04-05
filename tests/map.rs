use src_ctx::{SourceMap, Origin, Insert};
use test_util::test_map;


mod test_util;

#[test]
fn entries() {
    let mut map = SourceMap::new();

    let idx_file = map.insert(Origin::from_file("test-file"), "test-file-content".into())
        .try_into_inserted().unwrap();
    let idx_str = map.insert(Origin::from_named("test-str"), "test-str-content".into())
        .try_into_inserted().unwrap();

    assert!(map.contains_file("test-file"));
    assert!(! map.contains_file("unknown-file"));

    assert_eq!(map.file_index("test-file"), Some(idx_file));
    assert_eq!(map.origin_index(&Origin::from_file("test-file")), Some(idx_file));
    assert_eq!(map.origin_index(&Origin::from_named("test-str")), Some(idx_str));

    assert_eq!(map.content(idx_file), "test-file-content");
    assert_eq!(map.content(idx_str), "test-str-content");

    assert_eq!(map.origin(idx_file), &Origin::from_file("test-file"));
    assert_eq!(map.origin(idx_str), &Origin::from_named("test-str"));

    assert_eq!(
        map.insert(Origin::from_file("test-file"), "other-content".into()),
        Insert::Previous(idx_file)
    );
    assert_eq!(
        map.insert(Origin::from_named("test-str"), "other-content".into()),
        Insert::Previous(idx_str)
    );
    assert_eq!(
        map.load_file("test-file").unwrap(),
        Insert::Previous(idx_file)
    );
}

#[test]
fn map_ids() {
    let (map_a, index_a) = test_map("content a");
    let (map_b, index_b) = test_map("content b");

    assert!(map_a.contains(index_a));
    assert!(map_b.contains(index_b));

    assert!(! map_a.contains(index_b));
    assert!(! map_b.contains(index_a));
}