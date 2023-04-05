#![allow(unused)]

use src_ctx::{SourceMap, Origin, SourceIndex};

pub fn test_map(content: &str) -> (SourceMap, SourceIndex) {
    let mut map = SourceMap::new();
    let index = map.insert(Origin::Named("test".into()), content.into())
        .try_into_inserted().unwrap();
    (map, index)
}
