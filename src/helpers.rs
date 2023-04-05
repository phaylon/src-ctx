
/// Normalilze content for tests and whitespace-sensitive inputs.
///
/// Removes all lines only containing whitespaces. All other lines need to have
/// `|` as first non-whitespace character marking the start of the line. The output
/// will be the rest of the line after the marker.
///
/// # Panics
///
/// Since this function is not intended for dynamic inputs, it will panic when it
/// encounters a wrongly formatted line.
///
/// # Examples
///
/// ```rust
/// use src_ctx::normalize;
/// assert_eq!(
///     &normalize("
///         |abc
///         |  def
///     "),
///     "abc\n  def\n"
/// );
/// ```
#[track_caller]
pub fn normalize(content: &str) -> String {
    const LEAD: char = '|';

    let mut normalized = String::new();
    'lines: for line in content.lines() {
        if line.trim_start().is_empty() {
            continue 'lines;
        }
        let Some(index) = line.find(LEAD) else {
            panic!("non-empty lines must start with `{LEAD}` character: `{line}`");
        };
        let line = &line[(index + LEAD.len_utf8())..];
        normalized.push_str(line);
        normalized.push('\n');
    }
    normalized
}