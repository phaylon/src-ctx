#![allow(unused)]

use src_ctx::{SourceMap, Origin, SourceIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
#[error("{_0}")]
pub struct Error(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
#[error("{_0}")]
pub struct ErrorChain(pub &'static str, #[source] pub Error);

pub fn test_map(content: &str) -> (SourceMap, SourceIndex) {
    let mut map = SourceMap::new();
    let index = map.insert(Origin::from_named("test"), content.into())
        .try_into_inserted().unwrap();
    (map, index)
}

pub fn test_map_file(content: &str) -> (SourceMap, SourceIndex) {
    let mut map = SourceMap::new();
    let index = map.insert(Origin::from_file("test"), content.into())
        .try_into_inserted().unwrap();
    (map, index)
}
