use std::fmt::{self, Write};
use std::sync::Arc;

use crate::{Origin, Offset, SourceMap};
use crate::display::{display_fn, count_digits};


/// A generic error with associated context information.
///
/// The contexts in this error wrapper are fully realized and can be displayed
/// without access to a [`SourceMap`].
///
/// # Display
///
/// The [`Display`](std::fmt::Display) implementation for this type will only
/// print the inner error display followed by in-line source location information.
///
/// Use [`display_with_context`](Self::display_with_context) for the full output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextError<E> {
    error: E,
    origins: Arc<[ContextErrorOrigin]>,
}

impl<E> std::error::Error for ContextError<E>
where
    E: std::error::Error,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
}

impl<E> fmt::Display for ContextError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.error, self.display_origins_as_suffix())
    }
}

impl<E> ContextError<E> {
    /// Construct a context error with a given set of [`ContextErrorOrigin`] values.
    pub fn with_origins<I>(error: E, origins: I) -> Self
    where
        I: IntoIterator<Item = ContextErrorOrigin>,
    {
        Self { error, origins: origins.into_iter().collect() }
    }

    /// The encapsulated error value.
    pub fn error(&self) -> &E {
        &self.error
    }

    /// All contained [`ContextErrorOrigin`] values.
    pub fn error_origins(&self) -> &[ContextErrorOrigin] {
        &self.origins
    }

    /// Map the encapsulated error value to a new value and/or type.
    pub fn map<M, F>(self, map_error: F) -> ContextError<M>
    where
        F: FnOnce(E) -> M,
    {
        ContextError {
            error: map_error(self.error),
            origins: self.origins,
        }
    }

    /// Discard the context and nwrap the encapsulated error value.
    pub fn into_error(self) -> E {
        self.error
    }

    /// Construct a [`Display`](std::fmt::Display) proxy showing a full context.
    ///
    /// This returns a value that when displayed will print
    ///
    /// * The encapsulated error,
    /// * it's source chain,
    /// * and an expanded view of the source context.
    ///
    /// The carried static note will be used to highlight the error position
    /// in the content.
    pub fn display_with_context(&self) -> impl fmt::Display + '_
    where
        E: fmt::Display + std::error::Error,
    {
        display_fn(move |f| {
            writeln!(f, "error: {}", self.error)?;
            let mut curr: &dyn std::error::Error = &self.error;
            while let Some(source) = curr.source() {
                curr = source;
                writeln!(f, "cause: {}", curr)?;
            }
            for origin in self.origins.iter() {
                write!(f, "{origin}")?;
            }
            Ok(())
        })
    }

    /// Construct a [`Display`](std::fmt::Display) proxy showing context without
    /// additional error sources.
    ///
    /// This is functionally the same as [`display_with_context`] just without the
    /// [`std::error::Error`] requirement.
    pub fn display_with_outer_context(&self) -> impl fmt::Display + '_
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

/// The contextual origin of a position in a [`SourceMap`] context.
///
/// Can be displayed directly, or passed to [`ContextError::with_origins`] to associate
/// context objects with an error.
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
            if ctx_lnum < self_lnum {
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

/// A generic error carrying contextual [`Offset`] data.
///
/// These can be constructed without having access to a full source map and later
/// turned into full [`ContextError`] objects.
///
/// An error is centered around a primary error position offset, but can additionally
/// be given a context offset to also include in the contextual output.
///
/// This type carries no allocations unless encapsulated in the inner error
///
/// # Display
///
/// Since contextual information is not available, the [`Display`](std::fmt::Display)
/// implementation will simply output the encapsulated error followed by byte-offset
/// information.
#[derive(Debug, Clone)]
pub struct SourceError<E> {
    error: E,
    offset: Offset,
    offset_note: &'static str,
    context_offset: Option<Offset>,
}

impl<E> std::error::Error for SourceError<E>
where
    E: std::error::Error,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.error.source()
    }
}

impl<E> fmt::Display for SourceError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte offset {}", self.error, self.offset.byte())
    }
}

impl<E> SourceError<E> {
    /// Construct an error at a specific [`Offset`].
    ///
    /// The given note will be used to highlight the error position.
    pub fn new(error: E, offset: Offset, offset_note: &'static str) -> Self {
        Self { error, offset, offset_note, context_offset: None }
    }

    /// Associate some additional context [`Offset`] with the error.
    ///
    /// The line for this offset will also be included in the context.
    pub fn with_context(mut self, offset: Offset) -> Self {
        assert_eq!(self.offset.source_index(), offset.source_index(), "belongs to same source");
        self.context_offset = Some(offset);
        self
    }

    /// The encapsulated error value.
    pub fn error(&self) -> &E {
        &self.error
    }

    /// The [`Offset`] this error is associated with.
    pub fn offset(&self) -> Offset {
        self.offset
    }

    /// The additional context [`Offset`] to be included in the output, if any was given.
    pub fn context_offset(&self) -> Option<Offset> {
        self.context_offset
    }

    /// The note for the error position.
    pub fn note(&self) -> &'static str {
        self.offset_note
    }

    /// Map the encapsulated error value to a new value and/or type.
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

    /// Turn the error into a full [`ContextError`] by resolving it through a
    /// [`SourceMap`].
    pub fn into_context_error(self, map: &SourceMap) -> ContextError<E> {
        ContextError::with_origins(self.error, [
            map.context_error_origin(self.offset, self.offset_note, self.context_offset),
        ])
    }

    /// Discard the context and unwrap the encapsulated error value.
    pub fn into_error(self) -> E {
        self.error
    }
}
