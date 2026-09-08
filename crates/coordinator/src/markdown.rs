//! Markdown export. Original path is never included.

use std::path::Path;

use unveil_contracts::ContentTypeProbe;
use unveil_contracts::ModuleJob;

pub fn deobfuscation_markdown(
    display_name: &str,
    sha256: &str,
    analysis: &unveil_contracts::AnalysisResult,
    probe: Option<&ContentTypeProbe>,
) -> String {
    let mut out = String::new();
    out.push_str("# 難読化判定レポート\n\n");
    out.push_str("- 未検出は安全の証明ではない\n");
    out.push_str("- Magika score はタイプ信頼度であり悪意確率ではない\n\n");
    out.push_str("## 対象\n\n");
    out.push_str(&format!("- 表示名: `{display_name}`\n"));
    out.push_str(&format!("- SHA-256: `{sha256}`\n"));
    if let Some(p) = probe {
        out.push_str(&format!(
            "- 拡張子: `{}` / magic: `{}` / Magika: `{}`\n",
            p.declared_extension,
            p.magic_hint,
            p.magika.status
        ));
    }
    out.push_str(&format!(
        "\n## 四軸\n\n| coverage | assessment | findings |\n|---|---|---|\n| {:?} | {:?} | {} |\n\n",
        analysis.coverage.status,
        analysis.assessment,
        analysis.findings.len()
    ));
    out.push_str("## Finding\n\n");
    for f in &analysis.findings {
        out.push_str(&format!(
            "- `{}` identification={:?} score={} kind={}\n",
            f.technique_id, f.identification, f.score.value, f.score.kind
        ));
    }
    out.push('\n');
    out
}

pub fn module_job_markdown(job: &ModuleJob) -> String {
    format!(
        "# {} レポート\n\n- job_id: `{}`\n- status: {}\n- sha256: {}\n\n```json\n{}\n```\n",
        job.module_id,
        job.job_id,
        job.status,
        job.sha256.as_deref().unwrap_or("-"),
        serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| "{}".into())
    )
}

pub fn write_markdown(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, body).map_err(|e| e.to_string())
}
