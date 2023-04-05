A small reusable framework for nicer error reporting during language prototyping.

# Description

This library is aimed at providing a set of simple, reusable building blocks for reading,
parsing and producing diagnostic information when prototyping programming languages or
data formats.

Contained are:

* A source map holding content and origin information.
* An input content wrapper allowing access to position information and content traversal.
* A set of error types to compose diagnostic messages with contextual information.

It is not intended for anything long-term or serious, due to it's trade-offs. Namely:

* It's rather limited. There's no tokenization, only input-stream traversal. Errors are
  centered around offsets, not spans. Spans are available, but since things like useful
  multi-line span diagnostics can very much depend on the language, they are just completely
  left out.
* It's probably too slow and wasteful for production use. All offsets and spans carry an
  internal ID to ensure they're not used against the wrong source map. This is great for
  prototyping, but it increases the size of all offsets.
* It lacks maturity, polish and detail, being kept purpusefully simple. Locating the position
  of an offset to highlight in the context is done by a simple routine assuming everything that's
  not a tab has the same text width.
