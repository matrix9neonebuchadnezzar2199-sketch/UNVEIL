#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeKind {
    Utf8,
    Utf16Le,
    Gzip,
    Pe,
    Elf,
    MachO,
    Json,
    JavaScript,
    UnknownBinary,
}

impl ProbeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "utf8",
            Self::Utf16Le => "utf16le",
            Self::Gzip => "gzip",
            Self::Pe => "pe",
            Self::Elf => "elf",
            Self::MachO => "macho",
            Self::Json => "json",
            Self::JavaScript => "javascript",
            Self::UnknownBinary => "unknown-binary",
        }
    }
}

pub struct Probe {
    pub kind: ProbeKind,
    pub utf8_ok: bool,
}

pub fn probe(bytes: &[u8]) -> Probe {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        return Probe {
            kind: ProbeKind::Gzip,
            utf8_ok: false,
        };
    }
    if bytes.len() >= 2 && bytes[0] == b'M' && bytes[1] == b'Z' {
        return Probe {
            kind: ProbeKind::Pe,
            utf8_ok: false,
        };
    }
    if bytes.len() >= 4 && bytes[..4] == [0x7f, b'E', b'L', b'F'] {
        return Probe {
            kind: ProbeKind::Elf,
            utf8_ok: false,
        };
    }
    if bytes.len() >= 4
        && (bytes[..4] == [0xfe, 0xed, 0xfa, 0xce]
            || bytes[..4] == [0xce, 0xfa, 0xed, 0xfe]
            || bytes[..4] == [0xfe, 0xed, 0xfa, 0xcf]
            || bytes[..4] == [0xcf, 0xfa, 0xed, 0xfe])
    {
        return Probe {
            kind: ProbeKind::MachO,
            utf8_ok: false,
        };
    }
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xfe {
        return Probe {
            kind: ProbeKind::Utf16Le,
            utf8_ok: false,
        };
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            let kind = classify_text(text);
            Probe {
                kind,
                utf8_ok: true,
            }
        }
        Err(_) => Probe {
            kind: ProbeKind::UnknownBinary,
            utf8_ok: false,
        },
    }
}

fn classify_text(text: &str) -> ProbeKind {
    let trimmed = text.trim_start();
    if (trimmed.starts_with('{') || trimmed.starts_with('[')) && serde_json::from_str::<serde_json::Value>(text).is_ok()
    {
        return ProbeKind::Json;
    }
    if trimmed.starts_with("#!") && trimmed.contains("node") {
        return ProbeKind::JavaScript;
    }
    let js_hits = ["function ", "const ", "let ", "var ", "=>", "console.", "export ", "import "]
        .iter()
        .filter(|k| text.contains(*k))
        .count();
    if js_hits >= 2 {
        ProbeKind::JavaScript
    } else {
        ProbeKind::Utf8
    }
}
