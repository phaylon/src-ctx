
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