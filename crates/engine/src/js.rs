//! Tiny JS scanners. Not a runtime. Strings and comments are skipped for eval search.

pub fn looks_minified(text: &str) -> bool {
    if text.len() < 80 {
        return false;
    }
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return false;
    }
    let avg = lines.iter().map(|l| l.len()).sum::<usize>() as f64 / lines.len() as f64;
    let ws = text.chars().filter(|c| c.is_whitespace()).count() as f64 / text.len() as f64;
    avg >= 180.0 && ws < 0.12
}

pub fn find_eval_ranges(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut hits = Vec::new();
    while i + 5 <= bytes.len() {
        if let Some(skip) = skip_string_or_comment(bytes, i) {
            i = skip;
            continue;
        }
        if bytes[i..].starts_with(b"eval") && (i == 0 || !is_ident(bytes[i - 1])) {
            let after = i + 4;
            if after < bytes.len() && !is_ident(bytes[after]) {
                let mut j = after;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'(' {
                    hits.push((i, j + 1));
                }
            }
        }
        i += 1;
    }
    hits
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

pub fn find_literal_concats(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut hits = Vec::new();
    while i < bytes.len() {
        if let Some((lit_end, _)) = parse_string(bytes, i) {
            let mut j = lit_end;
            let mut end = lit_end;
            let mut parts = 1;
            loop {
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] != b'+' {
                    break;
                }
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if let Some((next_end, _)) = parse_string(bytes, j) {
                    parts += 1;
                    end = next_end;
                    j = next_end;
                } else {
                    break;
                }
            }
            if parts >= 2 {
                hits.push((i, end));
                i = end;
                continue;
            }
            i = lit_end;
            continue;
        }
        i += 1;
    }
    hits
}

pub fn fold_literal_concats(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if let Some((end, value)) = parse_string(bytes, i) {
            let mut j = end;
            let mut acc = value;
            let mut last = end;
            let mut folded = false;
            loop {
                let mut k = j;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k >= bytes.len() || bytes[k] != b'+' {
                    break;
                }
                k += 1;
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if let Some((next_end, next)) = parse_string(bytes, k) {
                    acc.push_str(&next);
                    last = next_end;
                    j = next_end;
                    folded = true;
                } else {
                    break;
                }
            }
            if folded {
                out.push('"');
                out.push_str(&escape_js(&acc));
                out.push('"');
                i = last;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

pub fn pretty_print(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut depth: i32 = 0;
    let mut last_was_space = false;
    while i < bytes.len() {
        if let Some(skip) = skip_string_or_comment_copy(bytes, i, &mut out) {
            i = skip;
            last_was_space = false;
            continue;
        }
        let b = bytes[i];
        match b {
            b'{' => {
                out.push('{');
                depth += 1;
                out.push('\n');
                push_indent(&mut out, depth);
                last_was_space = true;
            }
            b'}' => {
                depth = (depth - 1).max(0);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                push_indent(&mut out, depth);
                out.push('}');
                last_was_space = false;
            }
            b';' => {
                out.push(';');
                out.push('\n');
                push_indent(&mut out, depth);
                last_was_space = true;
            }
            b' ' | b'\n' | b'\r' | b'\t' => {
                if !last_was_space {
                    out.push(' ');
                    last_was_space = true;
                }
            }
            _ => {
                out.push(b as char);
                last_was_space = false;
            }
        }
        i += 1;
    }
    out
}

fn push_indent(out: &mut String, depth: i32) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn skip_string_or_comment(bytes: &[u8], i: usize) -> Option<usize> {
    if let Some((end, _)) = parse_string(bytes, i) {
        return Some(end);
    }
    if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
        let mut j = i + 2;
        while j < bytes.len() && bytes[j] != b'\n' {
            j += 1;
        }
        return Some(j);
    }
    if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
        let mut j = i + 2;
        while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
            j += 1;
        }
        return Some((j + 2).min(bytes.len()));
    }
    None
}

fn skip_string_or_comment_copy(bytes: &[u8], i: usize, out: &mut String) -> Option<usize> {
    if let Some((end, _)) = parse_string(bytes, i) {
        out.push_str(std::str::from_utf8(&bytes[i..end]).unwrap_or(""));
        return Some(end);
    }
    if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
        let mut j = i + 2;
        while j < bytes.len() && bytes[j] != b'\n' {
            j += 1;
        }
        out.push_str(std::str::from_utf8(&bytes[i..j]).unwrap_or(""));
        return Some(j);
    }
    if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
        let mut j = i + 2;
        while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
            j += 1;
        }
        let end = (j + 2).min(bytes.len());
        out.push_str(std::str::from_utf8(&bytes[i..end]).unwrap_or(""));
        return Some(end);
    }
    None
}

pub fn parse_string(bytes: &[u8], i: usize) -> Option<(usize, String)> {
    if i >= bytes.len() {
        return None;
    }
    let quote = bytes[i];
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut j = i + 1;
    let mut out = String::new();
    while j < bytes.len() {
        let b = bytes[j];
        if b == quote {
            return Some((j + 1, out));
        }
        if b == b'\\' {
            j += 1;
            if j >= bytes.len() {
                return None;
            }
            match bytes[j] {
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'\\' => out.push('\\'),
                b'\'' => out.push('\''),
                b'"' => out.push('"'),
                b'x' if j + 2 < bytes.len() => {
                    let h = std::str::from_utf8(&bytes[j + 1..j + 3]).ok()?;
                    out.push(u8::from_str_radix(h, 16).ok()? as char);
                    j += 2;
                }
                b'u' if j + 4 < bytes.len() => {
                    let h = std::str::from_utf8(&bytes[j + 1..j + 5]).ok()?;
                    let cp = u32::from_str_radix(h, 16).ok()?;
                    out.push(char::from_u32(cp)?);
                    j += 4;
                }
                other => out.push(other as char),
            }
            j += 1;
            continue;
        }
        if b == b'\n' {
            return None;
        }
        out.push(b as char);
        j += 1;
    }
    None
}

fn escape_js(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

pub fn find_js_escapes(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut hits = Vec::new();
    while i < bytes.len() {
        if let Some((end, value)) = parse_string(bytes, i) {
            if bytes[i..end].windows(2).any(|w| w == b"\\x" || w == b"\\u") && value.len() >= 1 {
                hits.push((i, end));
            }
            i = end;
            continue;
        }
        i += 1;
    }
    hits
}
