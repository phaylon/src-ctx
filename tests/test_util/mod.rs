#![allow(unused)]

use src_ctx::{SourceMap, Origin};

pub fn test_map(content: &str) -> SourceMap {
    let mut map = SourceMap::new();
    map.insert(Origin::Named("test".into()), content.into()).try_into_inserted().unwrap();
    map
}
