use unveil_contracts::{AnalysisResult, ReportDocument};

pub fn build_report(
    session_id: &str,
    analysis: Option<&AnalysisResult>,
    versions: &unveil_contracts::AppInfo,
    include_bodies: bool,
) -> ReportDocument {
    let mut redactions = vec![
        "original_path".into(),
        "raw_excerpt_default".into(),
        "hash_optional".into(),
    ];
    let mut gaps = vec!["Original bytes are omitted unless a reproduce package is approved.".into()];
    if !include_bodies {
        redactions.push("artifact_bodies".into());
        gaps.push("Bodies omitted: not fully reproducible from this report.".into());
    }
    let payload = serde_json::json!({
        "schema_version": versions.schema_version,
        "app": versions,
        "session_id": session_id,
        "included_original": false,
        "included_bodies": include_bodies,
        "analysis": analysis,
        "notice": "Not a safety verdict. Encoding success is not decryption.",
    });
    let json = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into());
    let html = html_report(&json, analysis);
    ReportDocument {
        report_id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        format: "json+html".into(),
        json,
        html,
        included_original: false,
        included_bodies: include_bodies,
        redactions,
        reproducibility_gaps: gaps,
    }
}

fn html_report(json: &str, analysis: Option<&AnalysisResult>) -> String {
    let mut body = String::from(
        "<!DOCTYPE html><html lang=\"ja\"><head><meta charset=\"utf-8\"><title>UNVEIL report</title></head><body>",
    );
    body.push_str("<h1>UNVEIL report</h1>");
    body.push_str("<p>This HTML contains no scripts. It is not a safety verdict.</p>");
    if let Some(a) = analysis {
        body.push_str(&format!(
            "<p>assessment={} coverage={:?} findings={}</p>",
            format!("{:?}", a.assessment).to_lowercase(),
            a.coverage.status,
            a.findings.len()
        ));
        body.push_str("<ul>");
        for f in &a.findings {
            body.push_str(&format!(
                "<li>{} {} score {}</li>",
                escape(&f.technique_id),
                format!("{:?}", f.identification).to_lowercase(),
                f.score.value
            ));
        }
        body.push_str("</ul>");
    }
    body.push_str("<pre>");
    body.push_str(&escape(json));
    body.push_str("</pre></body></html>");
    body
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
