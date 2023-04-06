use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::{ContextErrorLocation, Offset, Span, ContextErrorOrigin, Input};


/// An identifier for a specific source in a [`SourceMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceIndex {
    map_id: u32,
    data_index: u32,
}

/// A map storing source contents and their [`Origin`].
///
/// Every map has its own internal ID to prevent use of a [`SourceIndex`]
/// with a map it didn't originate from.
///
/// Because every map and index have an associated internal ID, maps are
/// not clonable as this would invalidate all prior indices.
///
/// # Panics
///
/// A panic will occur if the internal ID or the number of entries exceeds
/// [`u32::MAX`].
pub struct SourceMap {
    id: u32,
    origin_indices: HashMap<Origin, u32>,
    data: Vec<SourceData>,
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceMap {
    /// Construct an empty [`SourceMap`].
    pub fn new() -> Self {
        Self {
            id: fetch_next_source_map_id(),
            origin_indices: HashMap::new(),
            data: Vec::new(),
        }
    }

    /// Verify that an [`SourceIndex`] belongs to this map.
    pub fn contains(&self, idx: SourceIndex) -> bool {
        self.id == idx.map_id
    }

    /// Retrieve the [`Origin`] associated with a [`SourceIndex`].
    ///
    /// # Panics
    ///
    /// This function will panic if the index does not belong to this map.
    #[track_caller]
    pub fn origin(&self, idx: SourceIndex) -> &Origin {
        assert_eq!(self.id, idx.map_id, "origin index must belong to source map");
        &self.data[idx.data_index as usize].origin
    }


    /// Retrieve the content associated with a [`SourceIndex`].
    ///
    /// # Panics
    ///
    /// This function will panic if the index does not belong to this map.
    #[track_caller]
    pub fn content(&self, idx: SourceIndex) -> &str {
        assert_eq!(self.id, idx.map_id, "content index must belong to source map");
        &self.data[idx.data_index as usize].content
    }

    /// Construct an [`Input`] for the content associated with a [`SourceIndex`].
    ///
    /// # Panics
    ///
    /// This function will panic if the index does not belong to this map.
    #[track_caller]
    pub fn input(&self, idx: SourceIndex) -> Input<'_> {
        assert_eq!(self.id, idx.map_id, "input index must belong to source map");
        Input::new(idx, &self.data[idx.data_index as usize].content)
    }

    /// An iterator over all [`Origin`]s in this map.
    pub fn origins(&self) -> impl Iterator<Item = &Origin> + '_ {
        self.data.iter().map(|data| &data.origin)
    }

    /// Find the [`SourceIndex`] for a given [`Origin`] if there is one.
    pub fn origin_index(&self, origin: &Origin) -> Option<SourceIndex> {
        self.origin_indices.get(origin).map(|index| {
            SourceIndex { map_id: self.id, data_index: *index }
        })
    }

    /// An iterator over all file paths in this map.
    pub fn files(&self) -> impl Iterator<Item = &Path> + '_ {
        self.origins().filter_map(|origin| match origin {
            Origin::File(path) => Some(path.as_ref()),
            Origin::Named(_) => None,
        })
    }

    /// Find the [`SourceIndex`] for a given path if there is one.
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

    /// Determine if a file path is contained in this map.
    pub fn contains_file<P>(&self, path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        self.files().any(|origin_path| origin_path == path)
    }

    /// Try to insert a new source entry into the map.
    ///
    /// Returns a [`Insert::Previous`] if an entry with the same origin already exists
    /// in the map.
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

    /// Try to load a file into the source map.
    ///
    /// Returns a [`Insert::Previous`] if a file with the same path already exists
    /// in the map before attempting to load the path.
    ///
    /// # Errors
    ///
    /// An error will be returned if the file could not be read.
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

    /// Try to load all files with a specific extension below a root path.
    ///
    /// Returns a [`Vec`] of insertion outcomes. The outcome will be an [`Insert::Previous`]
    /// if a file with the same path was already loaded into the map.
    ///
    /// # Errors
    ///
    /// An error will be returned if the directory tree could not be fully searched or
    /// a file could not be loaded.
    ///
    /// No map insertions will be performed until all file
    /// loads are complete. An error will thus not result in an inconsistent set of
    /// loaded entries in the map.
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

    /// Retrieve the string corresponding to a [`Span`] in the map.
    ///
    /// # Panics
    ///
    /// This function will panic if the given span does not belong to this map.
    pub fn span_str(&self, span: Span) -> &str {
        let content = self.content(span.source_index());
        &content[span.byte_range()]
    }

    /// Calculate an offsets byte-position relative to the beginning of the line
    /// it is on.
    pub fn byte_offset_on_line(&self, offset: Offset) -> usize {
        let line = self.line_span(offset);
        offset.byte() - line.start().byte()
    }

    fn line_span(&self, offset: Offset) -> Span {
        let content = self.content(offset.source_index());
        let start = content[..offset.byte()]
            .rfind('\n').map(|byte| byte + 1)
            .unwrap_or(0);
        let end = content[offset.byte()..]
            .find('\n').map(|byte| byte + offset.byte())
            .unwrap_or_else(|| content.len());
        Span::new(Offset::new(offset.source_index(), start), end - start)
    }

    pub(crate) fn context_error_location(&self, offset: Offset) -> ContextErrorLocation {
        let line = self.line_span(offset);
        let start = line.start().byte();
        let end = line.end().byte();
        let content = self.content(offset.source_index());
        let line_number = content[..offset.byte()].split('\n').count();
        let column_number = 1 + (offset.byte() - start);
        ContextErrorLocation::new(
            content[start..end].into(),
            line_number,
            column_number,
        )
    }

    /// Capture a [`ContextErrorOrigin`] for a given [`Offset`].
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

/// Errors that can occur while loading [`SourceMap`] entries from the file system.
#[derive(Debug, Clone)]
pub enum LoadError {
    /// An error occured while trying to find files in a directory tree.
    Find {
        /// The root of the directory tree we searched in.
        root: Arc<Path>,
        /// The extension of the files we're trying to load.
        extension: Arc<str>,
        /// The error that occured during traversal.
        error: Arc<walkdir::Error>,
    },
    /// An error occured while reading a file.
    Read {
        /// The file we tried to read.
        file: Arc<Path>,
        /// The error that occured during reading.
        error: Arc<std::io::Error>,
    },
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Find { error, .. } => Some(error),
            LoadError::Read { error, .. } => Some(error),
        }
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Find { root, extension, .. } => {
                write!(f, "Failed to fully search `{}` for `*{extension}` files", root.display())
            },
            LoadError::Read { file, .. } => {
                write!(f, "Failed to read from file `{}`", file.display())
            },
        }
    }
}

struct SourceData {
    origin: Origin,
    content: Box<str>,
}

/// The outcome of an insertion into a [`SourceMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Insert {
    /// The entry was inserted under the given index.
    Inserted(SourceIndex),
    /// The entry already exists under the given index.
    Previous(SourceIndex),
}

impl Insert {
    /// Map the insertion outcome into a [`Result`].
    ///
    /// An insertion will be treated as success, while any previous source index
    /// will be used as the error.
    pub fn try_into_inserted(self) -> Result<SourceIndex, SourceIndex> {
        match self {
            Self::Inserted(idx) => Ok(idx),
            Self::Previous(idx) => Err(idx),
        }
    }
}

/// The origin of a [`SourceMap`] entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Origin {
    /// The entry is designated as having come from this file.
    File(Arc<Path>),
    /// The entry came from a named source instead of a file path.
    Named(Arc<str>),
}

impl Origin {
    /// Convenience constructor from anything that can be a path.
    pub fn from_file<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self::File(path.as_ref().into())
    }

    /// Convenience constructor from anything that can be a name.
    pub fn from_named<N>(name: N) -> Self
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
