//! Freedesktop `.desktop` files, the format ES-DE launches PC games from.

/// Joins arguments into an `Exec` value, quoting those the spec reserves.
pub fn exec<S: AsRef<str>>(args: &[S]) -> String {
    args.iter()
        .map(|a| quote(a.as_ref()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote(arg: &str) -> String {
    const RESERVED: &[char] = &[
        ' ', '\t', '\n', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(',
        ')', '`',
    ];
    if !arg.is_empty() && !arg.contains(RESERVED) {
        return arg.to_owned();
    }
    let mut quoted = String::from("\"");
    for c in arg.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    quoted
}

/// The unlocalised keys of the `[Desktop Entry]` group, in file order.
pub fn main_group(text: &str) -> Vec<(&str, &str)> {
    let mut in_main = false;
    let mut keys = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_main = line == "[Desktop Entry]";
        } else if in_main
            && let Some((key, value)) = line.split_once('=')
            && !key.contains('[')
        {
            keys.push((key.trim(), value.trim()));
        }
    }
    keys
}
