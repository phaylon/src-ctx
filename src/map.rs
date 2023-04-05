use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::{ContextErrorLocation, Offset, Span, ContextErrorOrigin, Input};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceIndex {
    map_id: u32,
    data_index: u32,
}

pub struct SourceMap {
    id: u32,
    origin_indices: HashMap<Origin, u32>,
    data: Vec<SourceData>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            id: fetch_next_source_map_id(),
            origin_indices: HashMap::new(),
            data: Vec::new(),
        }
    }

    #[track_caller]
    pub fn origin(&self, idx: SourceIndex) -> &Origin {
        assert_eq!(self.id, idx.map_id, "origin index must belong to source map");
        &self.data[idx.data_index as usize].origin
    }

    #[track_caller]
    pub fn content(&self, idx: SourceIndex) -> &str {
        assert_eq!(self.id, idx.map_id, "content index must belong to source map");
        &self.data[idx.data_index as usize].content
    }

    #[track_caller]
    pub fn input(&self, idx: SourceIndex) -> Input<'_> {
        assert_eq!(self.id, idx.map_id, "input index must belong to source map");
        Input::new(idx, &self.data[idx.data_index as usize].content)
    }

    pub fn origins(&self) -> impl Iterator<Item = &Origin> + '_ {
        self.data.iter().map(|data| &data.origin)
    }

    pub fn origin_index(&self, origin: &Origin) -> Option<SourceIndex> {
        self.origin_indices.get(origin).map(|index| {
            SourceIndex { map_id: self.id, data_index: *index }
        })
    }

    pub fn files(&self) -> impl Iterator<Item = &Path> + '_ {
        self.origins().filter_map(|origin| match origin {
            Origin::File(path) => Some(path.as_ref()),
            Origin::Named(_) => None,
        })
    }

    pub fn file_index<P>(&self, path: P) -> Option<SourceIndex>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        for (origin, index) in &self.origin_indices {
            if let Origin::File(origin_file) = origin {
                if origin_file.as_ref() == path {
                    return Some(SourceIndex {
                        map_id: self.id,
                        data_index: *index,
                    });
                }
            }
        }
        None
    }

    pub fn contains_file<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        self.files().any(|origin_path| origin_path == path)
    }

    pub fn insert(&mut self, origin: Origin, content: Box<str>) -> Insert {
        if let Some(prev_index) = self.origin_indices.get(&origin).copied() {
            return Insert::Previous(SourceIndex { map_id: self.id, data_index: prev_index });
        }
        let index: u32 = self.data.len().try_into().expect("maximum map size exceeded");
        self.origin_indices.insert(origin.clone(), index);
        self.data.push(SourceData { origin, content });
        Insert::Inserted(SourceIndex { map_id: self.id, data_index: index })
    }

    fn read_file<P>(&self, path: P) -> Result<Box<str>, ReadError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        if let Some(prev_index) = self.file_index(path) {
            return Err(ReadError::Previous(prev_index));
        }
        let content = std::fs::read_to_string(path)
            .map_err(|error| ReadError::Read(path.into(), error.into()))?;
        Ok(content.into())
    }

    pub fn load_file<P>(&mut self, path: P) -> Result<Insert, LoadError>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let content = match self.read_file(path) {
            Ok(content) => content,
            Err(error) => {
                return match error {
                    ReadError::Previous(index) => Ok(Insert::Previous(index)),
                    ReadError::Read(file, error) => Err(LoadError::Read { file, error }),
                };
            },
        };
        let origin = Origin::File(path.into());
        Ok(Insert::Inserted(self.insert(origin, content).try_into_inserted().unwrap()))
    }

    pub fn load_directory<P>(&mut self, root: P, extension: &str) -> Result<Vec<Insert>, LoadError>
    where
        P: AsRef<Path>,
    {
        let root = root.as_ref();
        let mut open = Vec::new();
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry.map_err(|error| LoadError::Find {
                root: root.into(),
                extension: extension.into(),
                error: error.into(),
            })?;
            let path = entry.path();
            if path.is_file() && path.ends_with(extension) {
                open.push(match self.read_file(path) {
                    Ok(content) => Ok((Origin::File(path.into()), content)),
                    Err(error) => match error {
                        ReadError::Previous(index) => Err(index),
                        ReadError::Read(file, error) => {
                            return Err(LoadError::Read { file, error });
                        },
                    },
                });
            }
        }
        Ok(open.into_iter().map(|open| match open {
            Ok((origin, content)) => {
                Insert::Inserted(self.insert(origin, content).try_into_inserted().unwrap())
            },
            Err(index) => Insert::Previous(index),
        }).collect())
    }

    pub fn span_str(&self, span: Span) -> &str {
        let content = self.content(span.source_index());
        &content[span.byte_range()]
    }

    pub(crate) fn context_error_location(&self, offset: Offset) -> ContextErrorLocation {
        let content = self.content(offset.source_index());
        let start = content[..offset.byte()]
            .rfind('\n').map(|byte| byte + 1)
            .unwrap_or(0);
        let end = content[offset.byte()..]
            .find('\n').map(|byte| byte + offset.byte())
            .unwrap_or_else(|| content.len());
        let line_number = content[..offset.byte()].split('\n').count();
        let column_number = 1 + (offset.byte() - start);
        ContextErrorLocation::new(
            content[start..end].into(),
            line_number,
            column_number,
        )
    }

    pub fn context_error_origin(
        &self,
        offset: Offset,
        note: &'static str,
        context: Option<Offset>,
    ) -> ContextErrorOrigin {
        let location = self.context_error_location(offset);
        let context = context.map(|offset| self.context_error_location(offset));
        ContextErrorOrigin::new(
            self.origin(offset.source_index()).clone(),
            note,
            location,
            context,
        )
    }
}

pub(super) enum ReadError {
    Previous(SourceIndex),
    Read(Arc<Path>, Arc<std::io::Error>),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum LoadError {
    #[error("Failed to search `{}` for `*{}` files: {}", root.display(), extension, error)]
    Find {
        root: Arc<Path>,
        extension: Arc<str>,
        error: Arc<walkdir::Error>,
    },
    #[error("Failed to read from `{}`: {}", file.display(), error)]
    Read {
        file: Arc<Path>,
        error: Arc<std::io::Error>,
    },
}

struct SourceData {
    origin: Origin,
    content: Box<str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insert {
    Inserted(SourceIndex),
    Previous(SourceIndex),
}

impl Insert {
    pub fn try_into_inserted(self) -> Result<SourceIndex, SourceIndex> {
        match self {
            Self::Inserted(idx) => Ok(idx),
            Self::Previous(idx) => Err(idx),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    File(Arc<Path>),
    Named(Arc<str>),
}

impl Origin {
    pub fn file<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self::File(path.as_ref().into())
    }

    pub fn named<N>(name: N) -> Self
    where
        N: AsRef<str>,
    {
        Self::Named(name.as_ref().into())
    }
}

fn fetch_next_source_map_id() -> u32 {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    NEXT.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |next| next.checked_add(1))
        .expect("source map id sequence exhausted")
}
