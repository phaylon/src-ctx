use std::ops::Range;

use crate::{SourceIndex, SourceError};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset {
    source_index: SourceIndex,
    byte: usize,
}

impl Offset {
    pub fn source_index(&self) -> SourceIndex {
        self.source_index
    }

    pub fn byte(&self) -> usize {
        self.byte
    }

    pub fn is_at_start(&self) -> bool {
        self.byte == 0
    }

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

    pub fn error<E>(&self, error: E, offset_note: &'static str) -> SourceError<E> {
        SourceError::new(error, *self, offset_note)
    }
}

impl From<Span> for Offset {
    fn from(span: Span) -> Self {
        span.offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    offset: Offset,
    byte_len: usize,
}

impl Span {
    pub fn source_index(&self) -> SourceIndex {
        self.offset.source_index
    }

    pub fn start(&self) -> Offset {
        self.offset
    }

    pub fn end(&self) -> Offset {
        Offset {
            source_index: self.offset.source_index,
            byte: self.offset.byte + self.byte_len,
        }
    }

    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    pub fn byte_range(&self) -> Range<usize> {
        self.offset.byte..(self.offset.byte + self.byte_len)
    }

    pub fn is_at_start(&self) -> bool {
        self.offset.is_at_start()
    }
}

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

    pub fn end(&self) -> Self {
        self.skip(self.len())
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn content(&self) -> &'src str {
        self.content
    }

    pub fn offset(&self) -> Offset {
        Offset {
            source_index: self.source_index,
            byte: self.byte,
        }
    }

    #[must_use]
    pub fn skip(&self, byte_len: usize) -> Self {
        Self {
            source_index: self.source_index,
            content: &self.content[byte_len..],
            byte: self.byte + byte_len,
        }
    }

    #[must_use]
    pub fn truncate(&self, byte_len: usize) -> Self {
        Self {
            source_index: self.source_index,
            content: &self.content[..byte_len],
            byte: self.byte,
        }
    }

    #[must_use]
    pub fn split(&self, byte_len: usize) -> (Self, Self) {
        (self.truncate(byte_len), self.skip(byte_len))
    }

    #[must_use]
    pub fn char(&self) -> Option<char> {
        self.content.chars().next()
    }

    #[must_use]
    pub fn skip_char(&self, c: char) -> Option<Self> {
        self.content.starts_with(c).then(|| self.skip(c.len_utf8()))
    }

    #[must_use]
    pub fn take_char(&self) -> Option<(char, Self)> {
        self.char().map(|c| (c, self.skip(c.len_utf8())))
    }

    pub fn error<E>(&self, error: E, offset_note: &'static str) -> SourceError<E> {
        SourceError::new(error, self.offset(), offset_note)
    }
}
