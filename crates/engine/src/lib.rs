//! Static analysis engine. Bytes are data. No eval, import, or sample execution.

mod detect;
mod features;
mod js;
mod probe;
mod transform;

pub use detect::detect;
pub use probe::{probe, ProbeKind};
pub use transform::{apply_adapter, preview_adapter};

use sha2::{Digest, Sha256};
use unveil_contracts::{
    AnalysisResult, AnalysisVersions, Assessment, Coverage, CoverageStatus, Finding,
    KNOWLEDGE_PACK, PROFILE_STATIC_SAFE, RULE_VERSION, SCHEMA_VERSION, TransformPreview,
    TransformResultKind, Validation, ValidationCheck, ValidationVerdict,
};

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn excerpt(bytes: &[u8], start: usize, end: usize) -> String {
    let end = end.min(bytes.len());
    let start = start.min(end);
    let slice = &bytes[start..end];
    let take = slice.len().min(unveil_contracts::MAX_EXCERPT_BYTES);
    let mut out = String::new();
    for &b in &slice[..take] {
        if (0x20..=0x7e).contains(&b) {
            out.push(b as char);
        } else {
            out.push('·');
        }
    }
    out
}

pub fn text_preview(bytes: &[u8]) -> String {
    let take = bytes.len().min(unveil_contracts::MAX_PREVIEW_BYTES as usize);
    match std::str::from_utf8(&bytes[..take]) {
        Ok(text) => text.replace('\0', "·"),
        Err(_) => excerpt(bytes, 0, take),
    }
}

pub fn analyze(artifact_id: &str, job_id: &str, bytes: &[u8]) -> AnalysisResult {
    let probe = probe(bytes);
    let features = features::extract(bytes, probe.kind);
    let mut findings = detect(artifact_id, bytes, &probe);
    findings.sort_by(|a, b| {
        a.technique_id
            .cmp(&b.technique_id)
            .then(a.range.start.cmp(&b.range.start))
            .then(a.id.cmp(&b.id))
    });
    let coverage = coverage_for(&probe, bytes.len() as u64, &findings);
    let assessment = assess(&probe, &findings);
    let mut limitations = vec![
        "Detection is not a safety verdict.".to_string(),
        "Encoding success is not decryption or deobfuscation of intent.".to_string(),
    ];
    if matches!(probe.kind, ProbeKind::Pe | ProbeKind::Elf | ProbeKind::MachO) {
        limitations.push(
            "Executable container detected. Internal analysis is PHASE 07 / unsupported.".into(),
        );
    }
    if bytes.len() as u64 > unveil_contracts::MAX_TEXT_PARSE_BYTES {
        limitations.push("Text/AST analysis limited to 10 MiB; coarse features used.".into());
    }
    AnalysisResult {
        job_id: job_id.to_string(),
        artifact_id: artifact_id.to_string(),
        sha256: sha256_hex(bytes),
        coverage,
        assessment,
        findings,
        features,
        limitations,
        probe_kind: probe.kind.as_str().to_string(),
        text_preview: text_preview(bytes),
        versions: AnalysisVersions {
            schema_version: SCHEMA_VERSION.to_string(),
            rule_version: RULE_VERSION.to_string(),
            knowledge_pack: KNOWLEDGE_PACK.to_string(),
            profile: PROFILE_STATIC_SAFE.to_string(),
        },
    }
}

fn coverage_for(probe: &probe::Probe, len: u64, findings: &[Finding]) -> Coverage {
    match probe.kind {
        ProbeKind::Pe | ProbeKind::Elf | ProbeKind::MachO | ProbeKind::UnknownBinary => Coverage {
            status: CoverageStatus::Unsupported,
            checked_rules: vec!["magic".into()],
            skipped: vec![
                "enc.base64".into(),
                "layout.minify".into(),
                "internal-sections".into(),
            ],
            inspected_bytes: len.min(192 * 1024),
            notes: vec!["Only magic and coarse entropy. Not a packer identification.".into()],
        },
        ProbeKind::Gzip => Coverage {
            status: CoverageStatus::Full,
            checked_rules: vec!["compression.gzip".into(), "magic".into()],
            skipped: vec!["multi-member-gzip".into()],
            inspected_bytes: len,
            notes: Vec::new(),
        },
        _ => {
            let status = if findings.len() >= 1000 {
                CoverageStatus::Partial
            } else {
                CoverageStatus::Full
            };
            Coverage {
                status,
                checked_rules: vec![
                    "enc.base64".into(),
                    "enc.base64url".into(),
                    "enc.base16".into(),
                    "enc.percent".into(),
                    "enc.js-escape".into(),
                    "layout.minify".into(),
                    "data.literal-concat".into(),
                    "control.dynamic-eval".into(),
                ],
                skipped: Vec::new(),
                inspected_bytes: len,
                notes: Vec::new(),
            }
        }
    }
}

fn assess(probe: &probe::Probe, findings: &[Finding]) -> Assessment {
    if matches!(
        probe.kind,
        ProbeKind::Pe | ProbeKind::Elf | ProbeKind::MachO | ProbeKind::UnknownBinary
    ) {
        return Assessment::Inconclusive;
    }
    let obfuscation = findings.iter().any(|f| {
        matches!(
            f.technique_id.as_str(),
            "control.dynamic-eval" | "data.literal-concat" | "data.string-table"
        )
    });
    if obfuscation {
        return Assessment::IndicatorsPresent;
    }
    let encodings = findings.iter().any(|f| {
        f.category == "encoding"
            && matches!(
                f.identification,
                unveil_contracts::Identification::Confirmed
                    | unveil_contracts::Identification::Probable
            )
    });
    let minify = findings.iter().any(|f| f.technique_id == "layout.minify");
    if encodings || minify {
        Assessment::Inconclusive
    } else {
        Assessment::NoIndicators
    }
}

pub fn preview_from_finding(
    bytes: &[u8],
    finding: &Finding,
    adapter_id: &str,
) -> Result<TransformPreview, unveil_contracts::IpcError> {
    let output = apply_adapter(bytes, finding, adapter_id)?;
    let validation = validate_adapter(bytes, finding, adapter_id, &output.bytes);
    Ok(TransformPreview {
        plan_id: String::new(),
        candidate_sha256: sha256_hex(&output.bytes),
        candidate_preview: text_preview(&output.bytes),
        byte_length: output.bytes.len() as u64,
        validation,
        result: if output.bytes == bytes {
            TransformResultKind::NoChange
        } else {
            TransformResultKind::Transformed
        },
    })
}

pub fn validate_adapter(
    original: &[u8],
    finding: &Finding,
    adapter_id: &str,
    output: &[u8],
) -> Validation {
    let mut checks = Vec::new();
    let mut verdict = ValidationVerdict::Passed;
    let mut limitations = Vec::new();
    match adapter_id {
        "enc.base64" | "enc.base64url" | "enc.base16" | "enc.percent" => {
            let start = finding.range.start as usize;
            let end = finding.range.end as usize;
            if end <= original.len() {
                if let Ok(reencoded) = transform::reencode_range(original, start, end, adapter_id, output)
                {
                    let ok = reencoded;
                    checks.push(ValidationCheck {
                        kind: "round_trip".into(),
                        detail: if ok {
                            "Selected range re-encodes to the same bytes.".into()
                        } else {
                            "Re-encode did not match the selected range.".into()
                        },
                    });
                    if !ok {
                        verdict = ValidationVerdict::Failed;
                    }
                }
            }
            limitations.push("Round-trip does not restore author intent.".into());
        }
        "compression.gzip" => {
            checks.push(ValidationCheck {
                kind: "structural".into(),
                detail: "Single-member inflate within size and ratio limits.".into(),
            });
            limitations.push("CRC success is not a malice or equivalence proof.".into());
        }
        "layout.minify" => {
            checks.push(ValidationCheck {
                kind: "structural".into(),
                detail: "Formatter does not execute JS. Identifier names stay lost.".into(),
            });
            limitations.push("Pretty-print is not semantic equivalence.".into());
        }
        "data.literal-concat" => {
            checks.push(ValidationCheck {
                kind: "rule_checked".into(),
                detail: "Only adjacent string literals joined by + were rewritten.".into(),
            });
            limitations.push("Source text / line numbers may change.".into());
        }
        _ => {
            verdict = ValidationVerdict::NotChecked;
            checks.push(ValidationCheck {
                kind: "none".into(),
                detail: "No validator for this adapter.".into(),
            });
        }
    }
    if output.is_empty() && adapter_id != "layout.minify" {
        verdict = ValidationVerdict::Failed;
    }
    Validation {
        checks,
        verdict,
        limitations,
        expected_hash: None,
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use unveil_contracts::{Assessment, Identification};

    #[test]
    fn base64_hello_is_probable_or_confirmed() {
        let input = b"SGVsbG8sIFVOVkVJTCE=";
        let result = analyze("art", "job", input);
        let hit = result
            .findings
            .iter()
            .find(|f| f.technique_id == "enc.base64")
            .expect("base64");
        assert!(matches!(
            hit.identification,
            Identification::Confirmed | Identification::Probable
        ));
        assert_ne!(result.assessment, Assessment::IndicatorsPresent);
    }

    #[test]
    fn hello_text_has_no_indicators() {
        let result = analyze("art", "job", b"Hello, UNVEIL!");
        assert_eq!(result.assessment, Assessment::NoIndicators);
        assert!(result
            .findings
            .iter()
            .all(|f| f.technique_id != "enc.base64" || f.score.value < 30));
    }

    #[test]
    fn pe_magic_is_inconclusive_unsupported() {
        let mut bytes = vec![b'M', b'Z'];
        bytes.extend(std::iter::repeat(0u8).take(64));
        let result = analyze("art", "job", &bytes);
        assert_eq!(result.coverage.status, CoverageStatus::Unsupported);
        assert_eq!(result.assessment, Assessment::Inconclusive);
        assert_eq!(result.probe_kind, "pe");
    }

    #[test]
    fn gzip_round_trip_fixture() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"Hello, UNVEIL!").unwrap();
        let gz = encoder.finish().unwrap();
        let result = analyze("art", "job", &gz);
        assert_eq!(result.probe_kind, "gzip");
        let finding = result
            .findings
            .iter()
            .find(|f| f.technique_id == "compression.gzip")
            .expect("gzip");
        let out = apply_adapter(&gz, finding, "compression.gzip").unwrap();
        assert_eq!(out.bytes, b"Hello, UNVEIL!");
    }

    #[test]
    fn literal_concat_rewrites_only_strings() {
        let src = b"const a = \"He\" + \"llo\";\n";
        let result = analyze("art", "job", src);
        let finding = result
            .findings
            .iter()
            .find(|f| f.technique_id == "data.literal-concat")
            .expect("concat");
        let out = apply_adapter(src, finding, "data.literal-concat").unwrap();
        let text = String::from_utf8(out.bytes).unwrap();
        assert!(text.contains("\"Hello\""));
        assert!(!text.contains("\"He\" + \"llo\""));
    }

    #[test]
    fn gzip_bomb_hits_limit() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        let huge = vec![0u8; 2 * 1024 * 1024];
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&huge).unwrap();
        let gz = encoder.finish().unwrap();
        // tiny gz of 2MiB zeros exceeds 100x if gz < 20KiB
        if (gz.len() as u64) * 100 < huge.len() as u64 {
            let finding = Finding {
                id: "f".into(),
                artifact_id: "a".into(),
                technique_id: "compression.gzip".into(),
                category: "compression".into(),
                range: unveil_contracts::ByteRange::new(0, gz.len() as u64),
                identification: Identification::Confirmed,
                score: unveil_contracts::Score {
                    value: 90,
                    kind: "rule_match".into(),
                    rule_version: RULE_VERSION.into(),
                },
                evidence: vec![],
                transformability: unveil_contracts::Transformability::Supported,
                limitations: vec![],
                article_id: "wiki.compression.gzip.ja".into(),
            };
            let err = apply_adapter(&gz, &finding, "compression.gzip").unwrap_err();
            assert_eq!(err.code, unveil_contracts::ErrorCode::LimitReached);
        }
    }
}
