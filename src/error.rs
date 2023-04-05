use std::fmt::{self, Write};
use std::sync::Arc;

use crate::{Origin, Offset, SourceMap};
use crate::display::{display_fn, count_digits};


#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{error}{}", self.display_origins_as_suffix())]
pub struct ContextError<E> {
    error: E,
    origins: Arc<[ContextErrorOrigin]>,
}

impl<E> ContextError<E> {
    pub fn from_origins<I>(error: E, origins: I) -> Self
    where
        I: IntoIterator<Item = ContextErrorOrigin>,
    {
        Self { error, origins: origins.into_iter().collect() }
    }

    pub fn error(&self) -> &E {
        &self.error
    }

    pub fn error_origins(&self) -> &[ContextErrorOrigin] {
        &self.origins
    }

    pub fn map<M, F>(self, map_error: F) -> ContextError<M>
    where
        F: FnOnce(E) -> M,
    {
        ContextError {
            error: map_error(self.error),
            origins: self.origins,
        }
    }

    pub fn into_error(self) -> E {
        self.error
    }

    pub fn display_with_context(&self) -> impl fmt::Display + '_
    where
        E: fmt::Display,
    {
        display_fn(move |f| {
            writeln!(f, "error: {}", self.error)?;
            for origin in self.origins.iter() {
                write!(f, "{origin}")?;
            }
            Ok(())
        })
    }

    fn display_origins_as_suffix(&self) -> impl fmt::Display + '_ {
        display_fn(move |f| {
            let mut origins = self.origins.as_ref();
            loop {
                break match origins {
                    [] => Ok(()),
                    [o] => {
                        write!(f, " {}", o.display_as_suffix())
                    },
                    [a, b] => {
                        write!(f, " {} and {}", a.display_as_suffix(), b.display_as_suffix())
                    },
                    [o, rest @ ..] => {
                        write!(f, " {},", o.display_as_suffix())?;
                        origins = rest;
                        continue
                    },
                }
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextErrorOrigin {
    origin: Origin,
    note: &'static str,
    location: ContextErrorLocation,
    context: Option<ContextErrorLocation>,
}

impl fmt::Display for ContextErrorOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lnum_width = count_digits(self.location.line_number);
        let self_lnum = self.location.line_number;
        let self_line = &self.location.line;
        writeln!(f, "--> {}", self.display_as_location())?;
        if let Some(ctx_location) = &self.context {
            let ctx_lnum = ctx_location.line_number;
            let ctx_line = &ctx_location.line;
            if ctx_lnum != self_lnum {
                writeln!(f, " {ctx_lnum:lnum_width$} | {ctx_line}")?;
                if ctx_lnum.checked_add(1) != Some(self_lnum) {
                    writeln!(f, " {:lnum_width$} | ...", "")?;
                }
            }
        }
        writeln!(f, " {self_lnum:lnum_width$} | {self_line}")?;
        let skipped = &self_line[..self.location.column_number];
        write!(f, " {:lnum_width$} |", "")?;
        for c in skipped.chars() {
            f.write_char(match c { '\t' => '\t', _ => ' '})?;
        }
        writeln!(f, "^ {}", self.note)?;
        Ok(())
    }
}

impl ContextErrorOrigin {
    pub(crate) fn new(
        origin: Origin,
        note: &'static str,
        location: ContextErrorLocation,
        context: Option<ContextErrorLocation>,
    ) -> Self {
        Self { origin, note, location, context }
    }

    fn display(&self, include_prefix: bool) -> impl fmt::Display + '_ {
        let ContextErrorLocation { line_number, column_number, .. } = &self.location;
        display_fn(move |f| match &self.origin {
            Origin::File(path) => {
                let prefix = if include_prefix { "at " } else { "" };
                write!(f, "{}{}:{}:{}", prefix, path.display(), line_number, column_number)
            },
            Origin::Named(name) => {
                let prefix = if include_prefix { "in " } else { "" };
                write!(f, "{}`{}`, line {}, column {}", prefix, name, line_number, column_number)
            },
        })
    }

    fn display_as_suffix(&self) -> impl fmt::Display + '_ {
        self.display(true)
    }

    fn display_as_location(&self) -> impl fmt::Display + '_ {
        self.display(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextErrorLocation {
    line_number: usize,
    column_number: usize,
    line: Arc<str>,
}

impl ContextErrorLocation {
    pub(crate) fn new(line: Arc<str>, line_number: usize, column_number: usize) -> Self {
        Self { line, line_number, column_number }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{error} at byte offset {}", offset.byte())]
pub struct SourceError<E> {
    error: E,
    offset: Offset,
    offset_note: &'static str,
    context_offset: Option<Offset>,
}

impl<E> SourceError<E> {
    pub fn new(error: E, offset: Offset, offset_note: &'static str) -> Self {
        Self { error, offset, offset_note, context_offset: None }
    }

    pub fn error(&self) -> &E {
        &self.error
    }

    pub fn offset(&self) -> Offset {
        self.offset
    }

    pub fn context_offset(&self) -> Option<Offset> {
        self.context_offset
    }

    pub fn note(&self) -> &'static str {
        self.offset_note
    }

    pub fn with_context(mut self, offset: Offset) -> Self {
        assert_eq!(self.offset.source_index(), offset.source_index(), "belongs to same source");
        self.context_offset = Some(offset);
        self
    }

    pub fn map<M, F>(self, map_error: F) -> SourceError<M>
    where
        F: FnOnce(E) -> M,
    {
        SourceError {
            error: map_error(self.error),
            offset: self.offset,
            offset_note: self.offset_note,
            context_offset: self.context_offset,
        }
    }

    pub fn into_context_error(self, map: &SourceMap) -> ContextError<E> {
        ContextError::from_origins(self.error, [
            map.context_error_origin(self.offset, self.offset_note, self.context_offset),
        ])
    }

    pub fn into_error(self) -> E {
        self.error
    }
}
