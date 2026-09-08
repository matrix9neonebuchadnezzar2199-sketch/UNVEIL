//! Content-type probe: extension + magic + Magika. Magika missing => unavailable, not libmagic fallback.

use uuid::Uuid;

use unveil_contracts::{
    magic_hint, ContentTypeProbe, MagikaOutput, declared_extension,
};

use crate::modules::{run_python_json, sidecar_dir};

pub fn probe_bytes(artifact_id: &str, display_name: &str, bytes: &[u8]) -> ContentTypeProbe {
    let ext = declared_extension(display_name);
    let magic = magic_hint(bytes);
    let magika = run_magika(bytes);
    ContentTypeProbe {
        probe_id: Uuid::new_v4().to_string(),
        artifact_id: artifact_id.into(),
        declared_extension: ext,
        magic_hint: magic,
        magika,
        mismatch_flags: Vec::new(),
        ghidra_eligible: false,
    }
    .compute_mismatch_and_routing()
}

fn run_magika(bytes: &[u8]) -> MagikaOutput {
    let script = sidecar_dir().join("magika_probe.py");
    if !script.is_file() {
        return unavailable("probe script missing");
    }
    let tmp = std::env::temp_dir().join(format!("unveil-magika-{}.bin", Uuid::new_v4()));
    if std::fs::write(&tmp, bytes).is_err() {
        return unavailable("cannot stage bytes");
    }
    let result = run_python_json(&script, &["--path", tmp.to_string_lossy().as_ref()], 30);
    let _ = std::fs::remove_file(&tmp);
    match result {
        Ok(v) => serde_json::from_value(v).unwrap_or_else(|_| unavailable("invalid magika json")),
        Err(_) => unavailable("magika sidecar failed"),
    }
}

fn unavailable(detail: &str) -> MagikaOutput {
    let _ = detail;
    MagikaOutput {
        status: "unavailable".into(),
        dl_label: None,
        output_label: None,
        score: None,
        prediction_mode: "high-confidence".into(),
        mime_type: None,
        is_text: None,
    }
}
