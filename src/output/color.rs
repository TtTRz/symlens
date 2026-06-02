/// ANSI color helpers. When `color` is false, returns `Cow::Borrowed` to avoid allocation.
use std::borrow::Cow;

pub fn bold(s: &str, color: bool) -> Cow<'_, str> {
    if color {
        Cow::Owned(format!("\x1b[1m{}\x1b[0m", s))
    } else {
        Cow::Borrowed(s)
    }
}

pub fn green(s: &str, color: bool) -> Cow<'_, str> {
    if color {
        Cow::Owned(format!("\x1b[32m{}\x1b[0m", s))
    } else {
        Cow::Borrowed(s)
    }
}

pub fn red(s: &str, color: bool) -> Cow<'_, str> {
    if color {
        Cow::Owned(format!("\x1b[31m{}\x1b[0m", s))
    } else {
        Cow::Borrowed(s)
    }
}

pub fn yellow(s: &str, color: bool) -> Cow<'_, str> {
    if color {
        Cow::Owned(format!("\x1b[33m{}\x1b[0m", s))
    } else {
        Cow::Borrowed(s)
    }
}

pub fn cyan(s: &str, color: bool) -> Cow<'_, str> {
    if color {
        Cow::Owned(format!("\x1b[36m{}\x1b[0m", s))
    } else {
        Cow::Borrowed(s)
    }
}

pub fn dim(s: &str, color: bool) -> Cow<'_, str> {
    if color {
        Cow::Owned(format!("\x1b[2m{}\x1b[0m", s))
    } else {
        Cow::Borrowed(s)
    }
}

/// Safely truncate a string to at most `max_chars` Unicode characters.
pub fn truncate_str(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
