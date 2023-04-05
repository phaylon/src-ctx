#![allow(unused)]

use src_ctx::{SourceMap, Origin, SourceIndex};

pub fn test_map(content: &str) -> (SourceMap, SourceIndex) {
    let mut map = SourceMap::new();
    let index = map.insert(Origin::named("test"), content.into())
        .try_into_inserted().unwrap();
    (map, index)
}

pub fn test_map_file(content: &str) -> (SourceMap, SourceIndex) {
    let mut map = SourceMap::new();
    let index = map.insert(Origin::file("test"), content.into())
        .try_into_inserted().unwrap();
    (map, index)
}
