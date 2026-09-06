use unveil_contracts::{ByteRange, Feature};

use crate::probe::ProbeKind;

pub fn extract(bytes: &[u8], kind: ProbeKind) -> Vec<Feature> {
    let mut out = Vec::new();
    out.push(Feature {
        id: "len".into(),
        range: ByteRange::new(0, bytes.len() as u64),
        extractor_id: "length".into(),
        value: bytes.len() as f64,
        unit: "bytes".into(),
    });
    let entropy = window_entropy(bytes);
    out.push(Feature {
        id: "entropy".into(),
        range: ByteRange::new(0, bytes.len() as u64),
        extractor_id: "shannon-4096".into(),
        value: entropy,
        unit: "bits/byte".into(),
    });
    let printable = bytes
        .iter()
        .filter(|b| {
            let v = **b;
            v == b'\n' || v == b'\r' || v == b'\t' || (0x20..=0x7e).contains(&v)
        })
        .count();
    out.push(Feature {
        id: "printable".into(),
        range: ByteRange::new(0, bytes.len() as u64),
        extractor_id: "printable-ratio".into(),
        value: if bytes.is_empty() {
            0.0
        } else {
            printable as f64 / bytes.len() as f64
        },
        unit: "ratio".into(),
    });
    if matches!(kind, ProbeKind::Utf8 | ProbeKind::JavaScript | ProbeKind::Json) {
        if let Ok(text) = std::str::from_utf8(bytes) {
            let lines: Vec<&str> = text.lines().collect();
            let avg = if lines.is_empty() {
                0.0
            } else {
                lines.iter().map(|l| l.len()).sum::<usize>() as f64 / lines.len() as f64
            };
            out.push(Feature {
                id: "avg-line".into(),
                range: ByteRange::new(0, bytes.len() as u64),
                extractor_id: "avg-line-len".into(),
                value: avg,
                unit: "chars".into(),
            });
        }
    }
    out
}

pub fn window_entropy(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    let window = 4096.min(bytes.len());
    shannon(&bytes[..window])
}

fn shannon(bytes: &[u8]) -> f64 {
    let mut counts = [0u32; 256];
    for &b in bytes {
        counts[b as usize] += 1;
    }
    let n = bytes.len() as f64;
    let mut h = 0.0;
    for c in counts {
        if c == 0 {
            continue;
        }
        let p = c as f64 / n;
        h -= p * p.log2();
    }
    h
}
