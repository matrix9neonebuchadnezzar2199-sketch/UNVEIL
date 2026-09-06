use uuid::Uuid;
use unveil_contracts::{
    ByteRange, Evidence, Finding, Identification, RULE_VERSION, Score, Transformability,
};

use crate::excerpt;
use crate::js::{find_eval_ranges, find_js_escapes, find_literal_concats, looks_minified};
use crate::probe::{Probe, ProbeKind};

const MAX_FINDINGS: usize = 1000;
const MIN_B64: usize = 16;

pub fn detect(artifact_id: &str, bytes: &[u8], probe: &Probe) -> Vec<Finding> {
    let mut out = Vec::new();
    match probe.kind {
        ProbeKind::Pe | ProbeKind::Elf | ProbeKind::MachO | ProbeKind::UnknownBinary => {
            return out;
        }
        ProbeKind::Gzip => {
            out.push(gzip_finding(artifact_id, bytes));
            return out;
        }
        _ => {}
    }
    scan_base64(artifact_id, bytes, &mut out);
    scan_base16(artifact_id, bytes, &mut out);
    scan_percent(artifact_id, bytes, &mut out);
    if let Ok(text) = std::str::from_utf8(bytes) {
        scan_js_escape(artifact_id, text, &mut out);
        scan_concat(artifact_id, text, &mut out);
        scan_eval(artifact_id, text, &mut out);
        if looks_minified(text) && matches!(probe.kind, ProbeKind::JavaScript) {
            out.push(minify_finding(artifact_id, bytes.len()));
        }
    }
    out.truncate(MAX_FINDINGS);
    out
}

fn gzip_finding(artifact_id: &str, bytes: &[u8]) -> Finding {
    finding(
        artifact_id,
        "compression.gzip",
        "compression",
        0,
        bytes.len() as u64,
        Identification::Confirmed,
        92,
        Transformability::Supported,
        "wiki.compression.gzip.ja",
        vec![ev(
            "magic",
            0,
            3.min(bytes.len() as u64),
            "1f 8b gzip magic",
            "supports",
            "structure",
            excerpt(bytes, 0, 8),
        )],
        vec![
            "Single-member inflate only.".into(),
            "Compression is not obfuscation by itself.".into(),
        ],
    )
}

fn minify_finding(artifact_id: &str, len: usize) -> Finding {
    finding(
        artifact_id,
        "layout.minify",
        "layout",
        0,
        len as u64,
        Identification::Probable,
        55,
        Transformability::Partial,
        "wiki.layout.minify.ja",
        vec![ev(
            "line-density",
            0,
            len as u64,
            "High average line length and low whitespace ratio",
            "supports",
            "layout",
            "".to_string(),
        )],
        vec![
            "Normal bundled JS looks the same.".into(),
            "Pretty-print cannot restore names or comments.".into(),
        ],
    )
}

fn scan_base64(artifact_id: &str, bytes: &[u8], out: &mut Vec<Finding>) {
    let mut i = 0;
    while i < bytes.len() && out.len() < MAX_FINDINGS {
        if !is_b64(bytes[i], false) {
            i += 1;
            continue;
        }
        let start = i;
        let mut saw_url = false;
        while i < bytes.len() && is_b64(bytes[i], true) {
            if bytes[i] == b'-' || bytes[i] == b'_' {
                saw_url = true;
            }
            i += 1;
        }
        let mut end = i;
        let mut pad = 0;
        while pad < 2 && end < bytes.len() && bytes[end] == b'=' {
            end += 1;
            pad += 1;
        }
        let raw_len = i - start;
        if raw_len < MIN_B64 {
            i = end.max(i + 1);
            continue;
        }
        let slice = &bytes[start..end];
        let text = match std::str::from_utf8(slice) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if saw_url {
            if let Some(f) = b64_finding(artifact_id, bytes, start, end, text, true) {
                out.push(f);
            }
        } else if let Some(f) = b64_finding(artifact_id, bytes, start, end, text, false) {
            let also_url = decode_b64(text, true).is_some();
            let mut finding = f;
            if also_url {
                finding.identification = Identification::Probable;
                finding.limitations.push("Base64url is also possible.".into());
            }
            out.push(finding);
        }
        i = end.max(start + 1);
    }
}

fn b64_finding(
    artifact_id: &str,
    bytes: &[u8],
    start: usize,
    end: usize,
    text: &str,
    url: bool,
) -> Option<Finding> {
    let decoded = decode_b64(text, url)?;
    let re = encode_b64(&decoded, url, text.contains('='));
    let round = canonical_eq(text, &re, url);
    let mut score: u8 = 10;
    if text.len() % 4 == 0 || url {
        score += 20;
    }
    if round {
        score += 40;
    }
    let in_string = start > 0 && (bytes[start - 1] == b'"' || bytes[start - 1] == b'\'');
    if in_string {
        score += 20;
    }
    if !round {
        score = score.saturating_sub(40);
    }
    score = score.min(100);
    if score < 30 {
        return None;
    }
    let ident = if round && score >= 80 && !url {
        Identification::Confirmed
    } else if score >= 60 {
        Identification::Probable
    } else {
        Identification::Possible
    };
    let tech = if url { "enc.base64url" } else { "enc.base64" };
    Some(finding(
        artifact_id,
        tech,
        "encoding",
        start as u64,
        end as u64,
        ident,
        score,
        Transformability::Supported,
        if url {
            "wiki.enc.base64.ja"
        } else {
            "wiki.enc.base64.ja"
        },
        vec![
            ev(
                "alphabet",
                start as u64,
                end as u64,
                "Alphabet and length match the profile",
                "supports",
                "charset",
                excerpt(bytes, start, end),
            ),
            ev(
                "roundtrip",
                start as u64,
                end as u64,
                if round {
                    "strict decode + re-encode matched"
                } else {
                    "re-encode mismatch"
                },
                if round { "supports" } else { "contradicts" },
                "roundtrip",
                excerpt(bytes, start, end),
            ),
        ],
        vec![
            "Encoding is not confidentiality.".into(),
            "Short tokens can be coincidental.".into(),
        ],
    ))
}

fn scan_base16(artifact_id: &str, bytes: &[u8], out: &mut Vec<Finding>) {
    let mut i = 0;
    while i < bytes.len() && out.len() < MAX_FINDINGS {
        if !bytes[i].is_ascii_hexdigit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            i += 1;
        }
        let len = i - start;
        if len < 16 || len % 2 != 0 {
            continue;
        }
        if start > 0 && is_b64(bytes[start - 1], true) {
            continue;
        }
        let score = if len == 32 || len == 40 || len == 64 { 45 } else { 55 };
        out.push(finding(
            artifact_id,
            "enc.base16",
            "encoding",
            start as u64,
            i as u64,
            if score >= 60 {
                Identification::Probable
            } else {
                Identification::Possible
            },
            score,
            Transformability::Supported,
            "wiki.enc.base16.ja",
            vec![ev(
                "hex-run",
                start as u64,
                i as u64,
                "Even-length hex alphabet run",
                "supports",
                "charset",
                excerpt(bytes, start, i),
            )],
            vec!["May be a hash fingerprint, not hidden payload.".into()],
        ));
    }
}

fn scan_percent(artifact_id: &str, bytes: &[u8], out: &mut Vec<Finding>) {
    let mut i = 0;
    while i + 3 <= bytes.len() && out.len() < MAX_FINDINGS {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        let start = i;
        let mut n = 0;
        while i + 3 <= bytes.len()
            && bytes[i] == b'%'
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            n += 1;
            i += 3;
        }
        if n >= 4 {
            out.push(finding(
                artifact_id,
                "enc.percent",
                "encoding",
                start as u64,
                i as u64,
                Identification::Probable,
                70,
                Transformability::Supported,
                "wiki.enc.escapes.ja",
                vec![ev(
                    "percent-run",
                    start as u64,
                    i as u64,
                    "%HH sequence",
                    "supports",
                    "structure",
                    excerpt(bytes, start, i),
                )],
                vec!["Does not treat + as space.".into()],
            ));
        } else {
            i = start + 1;
        }
    }
}

fn scan_js_escape(artifact_id: &str, text: &str, out: &mut Vec<Finding>) {
    for (start, end) in find_js_escapes(text) {
        if out.len() >= MAX_FINDINGS {
            break;
        }
        out.push(finding(
            artifact_id,
            "enc.js-escape",
            "encoding",
            start as u64,
            end as u64,
            Identification::Probable,
            72,
            Transformability::Supported,
            "wiki.enc.escapes.ja",
            vec![ev(
                "js-string-escape",
                start as u64,
                end as u64,
                "\\x or \\u inside a JS string literal",
                "supports",
                "syntax",
                excerpt(text.as_bytes(), start, end),
            )],
            vec!["Not applied to arbitrary non-JS text.".into()],
        ));
    }
}

fn scan_concat(artifact_id: &str, text: &str, out: &mut Vec<Finding>) {
    for (start, end) in find_literal_concats(text) {
        if out.len() >= MAX_FINDINGS {
            break;
        }
        out.push(finding(
            artifact_id,
            "data.literal-concat",
            "data",
            start as u64,
            end as u64,
            Identification::Confirmed,
            88,
            Transformability::Supported,
            "wiki.data.literal-concat.ja",
            vec![ev(
                "pure-concat",
                start as u64,
                end as u64,
                "Adjacent string literals joined by +",
                "supports",
                "syntax",
                excerpt(text.as_bytes(), start, end),
            )],
            vec!["Variables, calls, and tagged templates are out of scope.".into()],
        ));
    }
}

fn scan_eval(artifact_id: &str, text: &str, out: &mut Vec<Finding>) {
    for (start, end) in find_eval_ranges(text) {
        if out.len() >= MAX_FINDINGS {
            break;
        }
        out.push(finding(
            artifact_id,
            "control.dynamic-eval",
            "control",
            start as u64,
            end as u64,
            Identification::Possible,
            40,
            Transformability::Unsupported,
            "wiki.vm.overview.ja",
            vec![ev(
                "eval-syntax",
                start as u64,
                end as u64,
                "eval( syntax present; not executed",
                "supports",
                "syntax",
                excerpt(text.as_bytes(), start, end),
            )],
            vec![
                "Presence is not proof the call runs.".into(),
                "UNVEIL does not evaluate it.".into(),
            ],
        ));
    }
}

fn finding(
    artifact_id: &str,
    technique: &str,
    category: &str,
    start: u64,
    end: u64,
    identification: Identification,
    score: u8,
    transformability: Transformability,
    article_id: &str,
    evidence: Vec<Evidence>,
    limitations: Vec<String>,
) -> Finding {
    Finding {
        id: Uuid::new_v4().to_string(),
        artifact_id: artifact_id.to_string(),
        technique_id: technique.to_string(),
        category: category.to_string(),
        range: ByteRange::new(start, end),
        identification,
        score: Score {
            value: score,
            kind: "rule_match".into(),
            rule_version: RULE_VERSION.to_string(),
        },
        evidence,
        transformability,
        limitations,
        article_id: article_id.to_string(),
    }
}

fn ev(
    source: &str,
    start: u64,
    end: u64,
    observation: &str,
    polarity: &str,
    group: &str,
    excerpt: String,
) -> Evidence {
    Evidence {
        id: Uuid::new_v4().to_string(),
        source_rule: source.to_string(),
        range: ByteRange::new(start, end),
        observation: observation.to_string(),
        polarity: polarity.to_string(),
        group: group.to_string(),
        excerpt,
    }
}

fn is_b64(b: u8, url: bool) -> bool {
    b.is_ascii_alphanumeric()
        || b == b'+'
        || b == b'/'
        || (url && (b == b'-' || b == b'_'))
}

pub fn decode_b64(text: &str, url: bool) -> Option<Vec<u8>> {
    let mut s = text.replace('\n', "");
    if url {
        s = s.replace('-', "+").replace('_', "/");
    }
    while s.len() % 4 != 0 {
        s.push('=');
    }
    if s.chars().any(|c| {
        !c.is_ascii_alphanumeric() && c != '+' && c != '/' && c != '='
    }) {
        return None;
    }
    decode_b64_strict(&s)
}

fn decode_b64_strict(s: &str) -> Option<Vec<u8>> {
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut map = [0xffu8; 256];
    for (i, b) in table.iter().enumerate() {
        map[*b as usize] = i as u8;
    }
    map[b'=' as usize] = 0;
    let bytes = s.as_bytes();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        if chunk.iter().any(|b| map[*b as usize] == 0xff && *b != b'=') {
            return None;
        }
        let n = ((map[chunk[0] as usize] as u32) << 18)
            | ((map[chunk[1] as usize] as u32) << 12)
            | ((map[chunk[2] as usize] as u32) << 6)
            | (map[chunk[3] as usize] as u32);
        out.push(((n >> 16) & 0xff) as u8);
        if chunk[2] != b'=' {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if chunk[3] != b'=' {
            out.push((n & 0xff) as u8);
        }
        if chunk[2] == b'=' && chunk[3] != b'=' {
            return None;
        }
    }
    Some(out)
}

pub fn encode_b64(data: &[u8], url: bool, keep_pad: bool) -> String {
    let table = if url {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
    } else {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    };
    let mut out = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(table[((n >> 18) & 63) as usize] as char);
        out.push(table[((n >> 12) & 63) as usize] as char);
        if i + 1 < data.len() {
            out.push(table[((n >> 6) & 63) as usize] as char);
        } else if keep_pad && !url {
            out.push('=');
        }
        if i + 2 < data.len() {
            out.push(table[(n & 63) as usize] as char);
        } else if keep_pad && !url {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn canonical_eq(original: &str, reencoded: &str, url: bool) -> bool {
    if original == reencoded {
        return true;
    }
    if url {
        let a = original.trim_end_matches('=');
        let b = reencoded.trim_end_matches('=');
        return a == b;
    }
    false
}

pub fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let s = std::str::from_utf8(&bytes[i..i + 2]).ok()?;
        out.push(u8::from_str_radix(s, 16).ok()?);
    }
    Some(out)
}

pub fn encode_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn decode_percent(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 3 > bytes.len() {
                return None;
            }
            let s = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(s, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}
