use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use unveil_coordinator::{
    app_info, default_knowledge_dir, diagnose_isolation, worker_path, Coordinator,
};
use unveil_contracts::{Assessment, CoverageStatus, Identification};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("unveil-ctl diagnose | smoke");
        std::process::exit(2);
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "diagnose" => {
            let d = diagnose_isolation();
            println!("{}", serde_json::to_string_pretty(&d).unwrap());
            std::process::exit(if d.available { 0 } else { 1 });
        }
        "smoke" => {
            if let Err(err) = smoke() {
                eprintln!("SMOKE FAIL: {err}");
                std::process::exit(1);
            }
        }
        "info" => {
            println!("{}", serde_json::to_string_pretty(&app_info()).unwrap());
        }
        _ => {
            eprintln!("unknown command");
            std::process::exit(2);
        }
    }
}

fn smoke() -> Result<(), String> {
    let started = Instant::now();
    let worker = worker_path();
    if !worker.is_file() {
        return Err(format!("worker missing: {}", worker.display()));
    }
    env::set_var("UNVEIL_WORKER", worker.as_os_str());
    let diagnosis = diagnose_isolation();
    if !diagnosis.available {
        return Err(format!(
            "isolation unavailable: {}",
            serde_json::to_string(&diagnosis).unwrap_or_default()
        ));
    }
    let tmp = env::temp_dir().join(format!("unveil-smoke-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let knowledge = knowledge_dir();
    let coord = Coordinator::with_knowledge(tmp.join("ws"), knowledge.clone());

    // AT-01 lesson import + hash
    let lesson = coord.import_lesson().map_err(|e| e.to_string())?;
    let lesson_bytes = read_blob(&tmp.join("ws"), &lesson)?;
    if lesson.sha256.len() != 64 {
        return Err("sha256".into());
    }

    // AT-03/04 analyze lesson
    let analysis = coord
        .start_analysis(&lesson.artifact_id)
        .map_err(|e| e.to_string())?;
    if analysis.assessment == Assessment::IndicatorsPresent {
        return Err("lesson should not be treated as obfuscation-positive".into());
    }
    let b64 = analysis
        .findings
        .iter()
        .find(|f| f.technique_id == "enc.base64")
        .ok_or("lesson base64 finding")?;
    if !matches!(
        b64.identification,
        Identification::Confirmed | Identification::Probable | Identification::Possible
    ) {
        return Err("base64 identification".into());
    }

    // AT-06 preview/approve/apply
    let plan = coord
        .plan_from_finding(&lesson.artifact_id, &b64.id, Some("enc.base64"))
        .map_err(|e| e.to_string())?;
    let _preview = coord.preview_plan(&plan.plan_id).map_err(|e| e.to_string())?;
    let approval = coord
        .approve_plan(&plan.plan_id, &plan.plan_hash)
        .map_err(|e| e.to_string())?;
    let applied = coord
        .apply_plan(&plan.plan_id, &approval.approval_id, &plan.plan_hash)
        .map_err(|e| e.to_string())?;
    if applied.output.sha256.is_empty() {
        return Err("apply hash".into());
    }
    let after = read_blob(&tmp.join("ws"), &lesson)?;
    if after != lesson_bytes {
        return Err("original mutated".into());
    }
    let stale = coord.approve_plan(&plan.plan_id, "deadbeef");
    if stale.is_ok() {
        return Err("stale approval must fail".into());
    }

    // hello text: no indicators
    let hello = coord
        .import_bytes(b"Hello, UNVEIL!", "hello.txt")
        .map_err(|e| e.to_string())?;
    let hello_a = coord
        .start_analysis(&hello.artifact_id)
        .map_err(|e| e.to_string())?;
    if hello_a.assessment != Assessment::NoIndicators {
        return Err(format!("hello assessment {:?}", hello_a.assessment));
    }

    // gzip
    let gz = gzip_hello()?;
    let gz_art = coord.import_bytes(&gz, "hello.gz").map_err(|e| e.to_string())?;
    let gz_a = coord
        .start_analysis(&gz_art.artifact_id)
        .map_err(|e| e.to_string())?;
    let gz_f = gz_a
        .findings
        .iter()
        .find(|f| f.technique_id == "compression.gzip")
        .ok_or("gzip finding")?;
    let gz_plan = coord
        .plan_from_finding(&gz_art.artifact_id, &gz_f.id, Some("compression.gzip"))
        .map_err(|e| e.to_string())?;
    let _ = coord.preview_plan(&gz_plan.plan_id).map_err(|e| e.to_string())?;
    let gz_ok = coord
        .approve_plan(&gz_plan.plan_id, &gz_plan.plan_hash)
        .map_err(|e| e.to_string())?;
    let gz_out = coord
        .apply_plan(&gz_plan.plan_id, &gz_ok.approval_id, &gz_plan.plan_hash)
        .map_err(|e| e.to_string())?;
    let decoded = read_blob(&tmp.join("ws"), &gz_out.output)?;
    if decoded != b"Hello, UNVEIL!" {
        return Err("gzip decode mismatch".into());
    }

    // concat
    let concat = coord
        .import_bytes(b"const a = \"He\" + \"llo\";\n", "concat.js")
        .map_err(|e| e.to_string())?;
    let concat_a = coord
        .start_analysis(&concat.artifact_id)
        .map_err(|e| e.to_string())?;
    if concat_a.assessment != Assessment::IndicatorsPresent {
        return Err("concat should be indicators_present".into());
    }

    // PE magic AT-02
    let mut pe = vec![b'M', b'Z'];
    pe.extend(std::iter::repeat(0u8).take(64));
    let pe_art = coord.import_bytes(&pe, "sample.dat").map_err(|e| e.to_string())?;
    let pe_a = coord
        .start_analysis(&pe_art.artifact_id)
        .map_err(|e| e.to_string())?;
    if pe_a.coverage.status != CoverageStatus::Unsupported {
        return Err("pe coverage".into());
    }
    if pe_a.assessment != Assessment::Inconclusive {
        return Err("pe must not be no_indicators".into());
    }

    // WIKI AT-10
    let hits = coord.wiki_search("base64");
    if hits.is_empty() {
        return Err(format!("wiki search empty, knowledge={}", knowledge.display()));
    }
    let article = coord
        .wiki_get(&hits[0].article_id)
        .map_err(|e| e.to_string())?;
    if article.body_markdown.len() < 80 {
        return Err("wiki body".into());
    }
    if coord.wiki_timeline().is_empty() || coord.wiki_glossary().is_empty() {
        return Err("timeline/glossary".into());
    }

    // AT-13 report + save
    let _ = coord.save_session("smoke").map_err(|e| e.to_string())?;
    let report = coord.export_report(false).map_err(|e| e.to_string())?;
    if report.html.contains("<script") {
        return Err("report script".into());
    }
    if report.included_original {
        return Err("original must stay excluded".into());
    }

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "ms": started.elapsed().as_millis(),
            "isolation": true,
            "lesson": lesson.sha256,
            "findings": analysis.findings.len(),
            "wiki_hits": hits.len(),
            "phase": app_info().phase,
        })
    );
    Ok(())
}

fn knowledge_dir() -> PathBuf {
    if let Ok(p) = env::var("UNVEIL_KNOWLEDGE") {
        return PathBuf::from(p);
    }
    default_knowledge_dir()
}

fn read_blob(workspace: &std::path::Path, artifact: &unveil_contracts::ImportedArtifact) -> Result<Vec<u8>, String> {
    let path = workspace
        .join("sessions")
        .join(&artifact.session_id)
        .join("blobs")
        .join(&artifact.sha256);
    fs::read(path).map_err(|e| e.to_string())
}

fn gzip_hello() -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(b"Hello, UNVEIL!")
        .map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}
