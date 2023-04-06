use std::ops::Range;

use crate::{SourceIndex, SourceError};


/// A position in a [`SourceMap`](crate::SourceMap) entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset {
    source_index: SourceIndex,
    byte: usize,
}

impl Offset {
    pub(crate) fn new(source_index: SourceIndex, byte: usize) -> Self {
        Self { source_index, byte }
    }

    /// The [`SourceIndex`] of the map entry this offset is associated with.
    pub fn source_index(&self) -> SourceIndex {
        self.source_index
    }

    /// The actual byte-position of the offset.
    pub fn byte(&self) -> usize {
        self.byte
    }

    /// Determine if this offset is at the start of the content.
    pub fn is_at_start(&self) -> bool {
        self.byte == 0
    }

    /// Construct a [`Span`] from one offset to another.
    ///
    /// # Panics
    ///
    /// This function will panic if the two offsets don't come from the same entry
    /// in the same map.
    #[track_caller]
    pub fn span(&self, other: Self) -> Span {
        assert_eq!(self.source_index, other.source_index, "span offsets must be from same source");
        let mut a = self.byte;
        let mut b = other.byte;
        if b < a {
            std::mem::swap(&mut a, &mut b);
        }
        Span {
            offset: Offset {
                source_index: self.source_index,
                byte: a,
            },
            byte_len: b - a,
        }
    }

    /// Construct a [`SourceError`] at this offset.
    pub fn error<E>(&self, error: E, offset_note: &'static str) -> SourceError<E> {
        SourceError::new(error, *self, offset_note)
    }
}

impl From<Span> for Offset {
    fn from(span: Span) -> Self {
        span.offset
    }
}

/// A span of content in a [`SourceMap`](crate::SourceMap) entry.
///
/// Spans are constructed from two [`Offset`]s with [`Offset::span`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    offset: Offset,
    byte_len: usize,
}

impl Span {
    pub(crate) fn new(offset: Offset, byte_len: usize) -> Self {
        Self { offset, byte_len }
    }

    /// The [`SourceIndex`] of the map entry this offset is associated with.
    pub fn source_index(&self) -> SourceIndex {
        self.offset.source_index
    }

    /// The [`Offset`] at the start of the span.
    pub fn start(&self) -> Offset {
        self.offset
    }

    /// The [`Offset`] at the end of the span.
    pub fn end(&self) -> Offset {
        Offset {
            source_index: self.offset.source_index,
            byte: self.offset.byte + self.byte_len,
        }
    }

    /// The length of the span in bytes.
    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    /// The range of the span in bytes.
    pub fn byte_range(&self) -> Range<usize> {
        self.offset.byte..(self.offset.byte + self.byte_len)
    }

    /// Determine if the [`start`](Self::start) [`Offset`] is at the beginning of
    /// the content.
    pub fn is_at_start(&self) -> bool {
        self.offset.is_at_start()
    }
}

/// An input traversal wrapper for contents in a [`SourceMap`](crate::SourceMap).
///
/// Inputs are constructed with [`SourceMap::input`](crate::SourceMap::input).
#[derive(Debug, Clone)]
pub struct Input<'src> {
    source_index: SourceIndex,
    content: &'src str,
    byte: usize,
}

impl<'src> Input<'src> {
    pub(crate) fn new(source_index: SourceIndex, content: &'src str) -> Self {
        Self { source_index, content, byte: 0 }
    }

    /// The byte-length of the remaining input content.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Determine if the input content has been fully consumed.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// The content left for the input.
    pub fn content(&self) -> &'src str {
        self.content
    }

    /// The [`Offset`] corresponding to the current input position.
    pub fn offset(&self) -> Offset {
        Offset {
            source_index: self.source_index,
            byte: self.byte,
        }
    }

    /// Skip to the end of the input.
    #[must_use]
    pub fn end(&self) -> Self {
        self.skip(self.len())
    }

    /// Skip a number of bytes.
    #[must_use]
    pub fn skip(&self, byte_len: usize) -> Self {
        Self {
            source_index: self.source_index,
            content: &self.content[byte_len..],
            byte: self.byte + byte_len,
        }
    }

    /// Truncate the input content to a specific byte length.
    #[must_use]
    pub fn truncate(&self, byte_len: usize) -> Self {
        Self {
            source_index: self.source_index,
            content: &self.content[..byte_len],
            byte: self.byte,
        }
    }

    /// Split the input into two parts at a given byte position.
    #[must_use]
    pub fn split(&self, byte_len: usize) -> (Self, Self) {
        (self.truncate(byte_len), self.skip(byte_len))
    }

    /// The next [`char`] in the input.
    #[must_use]
    pub fn char(&self) -> Option<char> {
        self.content.chars().next()
    }

    /// Try to skip a specific [`char`] in the input.
    #[must_use]
    pub fn skip_char(&self, c: char) -> Option<Self> {
        self.content.starts_with(c).then(|| self.skip(c.len_utf8()))
    }

    /// Try to consume any [`char`] in the input.
    #[must_use]
    pub fn take_char(&self) -> Option<(char, Self)> {
        self.char().map(|c| (c, self.skip(c.len_utf8())))
    }

    /// Construct a [`SourceError`] for the current input position.
    pub fn error<E>(&self, error: E, offset_note: &'static str) -> SourceError<E> {
        SourceError::new(error, self.offset(), offset_note)
    }
}
