/// ANSI color helpers. When `color` is false, returns the string as-is.

pub fn bold(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[1m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn green(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[32m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn red(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[31m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn yellow(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[33m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn cyan(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[36m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn dim(s: &str, color: bool) -> String {
    if color {
        format!("\x1b[2m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}
