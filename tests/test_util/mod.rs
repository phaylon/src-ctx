#![allow(unused)]

use src_ctx::{SourceMap, Origin, SourceIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Error(pub &'static str);

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorChain(pub &'static str, pub Error);

impl std::error::Error for ErrorChain {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.1)
    }
}

impl std::fmt::Display for ErrorChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
