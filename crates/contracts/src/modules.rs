//! Module registry (`modules.manifest`) and content-type probe types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleManifest {
    pub schema_version: String,
    #[serde(default)]
    pub workbench: Option<serde_json::Value>,
    pub modules: Vec<ModuleSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleSpec {
    pub id: String,
    pub label_ja: String,
    #[serde(default)]
    pub label_en: Option<String>,
    pub route: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub status: String,
    #[serde(default)]
    pub worker: Option<String>,
    #[serde(default)]
    pub test_probes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityCheck {
    pub id: String,
    pub available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleDiagnosis {
    pub module_id: String,
    pub available: bool,
    pub status: String,
    pub checks: Vec<CapabilityCheck>,
}

fn default_prediction_mode() -> String {
    "high-confidence".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MagikaOutput {
    pub status: String,
    #[serde(default)]
    pub dl_label: Option<String>,
    #[serde(default)]
    pub output_label: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default = "default_prediction_mode")]
    pub prediction_mode: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub is_text: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentTypeProbe {
    pub probe_id: String,
    pub artifact_id: String,
    pub declared_extension: String,
    pub magic_hint: String,
    pub magika: MagikaOutput,
    pub mismatch_flags: Vec<String>,
    pub ghidra_eligible: bool,
}

impl ContentTypeProbe {
    pub fn compute_mismatch_and_routing(mut self) -> Self {
        self.mismatch_flags = mismatch_flags(&self.declared_extension, &self.magic_hint, &self.magika);
        self.ghidra_eligible = ghidra_eligible(&self);
        self
    }
}

pub fn magic_hint(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "empty".into();
    }
    if bytes.starts_with(b"MZ") {
        return "PE32 executable".into();
    }
    if bytes.len() >= 4 && bytes[0] == 0x7f && &bytes[1..4] == b"ELF" {
        return "ELF".into();
    }
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return "gzip compressed".into();
    }
    if bytes.starts_with(b"%PDF") {
        return "PDF document".into();
    }
    if bytes.iter().all(|b| *b == 0 || *b == 9 || *b == 10 || *b == 13 || (32..=126).contains(b) || *b >= 128)
        && bytes.iter().filter(|b| **b >= 32 || **b == 9 || **b == 10 || **b == 13).count() * 10
            >= bytes.len() * 8
    {
        return "ASCII text".into();
    }
    "Unknown binary data".into()
}

pub fn declared_extension(name: &str) -> String {
    PathExt::from_name(name)
}

struct PathExt;
impl PathExt {
    fn from_name(name: &str) -> String {
        std::path::Path::new(name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
    }
}

pub fn mismatch_flags(ext: &str, magic: &str, magika: &MagikaOutput) -> Vec<String> {
    let mut flags = Vec::new();
    let magic_l = magic.to_ascii_lowercase();
    let pe_magic = magic_l.contains("pe32") || magic_l.contains("mz");
    if (ext == "txt" || ext == "js" || ext == "md") && pe_magic {
        flags.push("extension_vs_magic".into());
    }
    if magika.status == "ok" {
        if let Some(label) = magika.output_label.as_deref() {
            let pe_label = matches!(label, "pebin" | "exe" | "dll" | "elf" | "macho");
            if pe_magic != pe_label && (pe_magic || pe_label) {
                flags.push("magic_vs_magika".into());
            }
            if (ext == "txt" || ext == "js") && pe_label {
                flags.push("extension_vs_magika".into());
            }
        }
    }
    flags
}

pub fn ghidra_eligible(probe: &ContentTypeProbe) -> bool {
    if probe.magika.status == "ok" {
        if probe.magika.prediction_mode != "high-confidence" {
            return false;
        }
        return magika_pe_family(probe.magika.output_label.as_deref());
    }
    let ext = probe.declared_extension.as_str();
    if matches!(ext, "js" | "mjs" | "ts" | "py" | "ps1" | "css" | "html") {
        return false;
    }
    let magic = probe.magic_hint.to_ascii_lowercase();
    magic.contains("pe32") || magic.contains("elf")
}

fn magika_pe_family(label: Option<&str>) -> bool {
    matches!(label, Some("pebin" | "exe" | "dll" | "elf" | "macho"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleJob {
    pub job_id: String,
    pub module_id: String,
    pub input_artifact_id: Option<String>,
    pub sha256: Option<String>,
    pub status: String,
    pub payload: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_manifest_parses() {
        let raw = include_str!("../../../docs/modules.manifest.example.json");
        let man: ModuleManifest = serde_json::from_str(raw).expect("manifest json");
        assert_eq!(man.schema_version, "1.0");
        let ids: Vec<_> = man.modules.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"dashboard"));
        assert!(ids.contains(&"malware"));
        assert!(ids.contains(&"deobfuscation"));
        assert!(ids.contains(&"usb"));
        let planned: Vec<_> = man
            .modules
            .iter()
            .filter(|m| m.status == "planned")
            .map(|m| m.id.as_str())
            .collect();
        assert!(planned.contains(&"malware"));
        assert!(planned.contains(&"usb"));
    }

    #[test]
    fn js_is_never_ghidra() {
        let probe = ContentTypeProbe {
            probe_id: "p".into(),
            artifact_id: "a".into(),
            declared_extension: "js".into(),
            magic_hint: "ASCII text".into(),
            magika: MagikaOutput {
                status: "unavailable".into(),
                dl_label: None,
                output_label: None,
                score: None,
                prediction_mode: "high-confidence".into(),
                mime_type: None,
                is_text: None,
            },
            mismatch_flags: vec![],
            ghidra_eligible: false,
        };
        assert!(!ghidra_eligible(&probe));
    }

    #[test]
    fn txt_pe_mismatch() {
        let magika = MagikaOutput {
            status: "unavailable".into(),
            dl_label: None,
            output_label: None,
            score: None,
            prediction_mode: "high-confidence".into(),
            mime_type: None,
            is_text: None,
        };
        let flags = mismatch_flags("txt", "PE32 executable", &magika);
        assert!(flags.contains(&"extension_vs_magic".to_string()));
    }

    #[test]
    fn magika_js_blocks_ghidra_even_if_exe_name() {
        let probe = ContentTypeProbe {
            probe_id: "p".into(),
            artifact_id: "a".into(),
            declared_extension: "exe".into(),
            magic_hint: "ASCII text".into(),
            magika: MagikaOutput {
                status: "ok".into(),
                dl_label: Some("javascript".into()),
                output_label: Some("javascript".into()),
                score: Some(0.99),
                prediction_mode: "high-confidence".into(),
                mime_type: Some("text/javascript".into()),
                is_text: Some(true),
            },
            mismatch_flags: vec![],
            ghidra_eligible: true,
        };
        assert!(!ghidra_eligible(&probe));
    }
}
