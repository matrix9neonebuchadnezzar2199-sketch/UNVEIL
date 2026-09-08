use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use unveil_coordinator::{
    app_info, default_knowledge_dir, diagnose_isolation, diagnose_module, list_modules_json,
    worker_path, Coordinator,
};
use unveil_contracts::{Assessment, CoverageStatus, Identification};

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!(
            "unveil-ctl diagnose | smoke | info | modules | diagnose-module <id> | probe | usb-drives | usb-scan | malcheck | handoff | handoff-file | export-md"
        );
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
        "modules" => match list_modules_json() {
            Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        "diagnose-module" => {
            let id = args.first().cloned().unwrap_or_default();
            if id.is_empty() {
                eprintln!("diagnose-module <module_id>");
                std::process::exit(2);
            }
            match diagnose_module(&id) {
                Ok(d) => {
                    println!("{}", serde_json::to_string_pretty(&d).unwrap());
                    std::process::exit(if d.available { 0 } else { 1 });
                }
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
        "probe" => {
            if let Err(err) = cmd_probe(&args) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        "usb-drives" => {
            if let Err(err) = cmd_usb_drives() {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        "usb-scan" => {
            if let Err(err) = cmd_usb(&args) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        "malcheck" => {
            if let Err(err) = cmd_malcheck(&args) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        "handoff" => {
            if let Err(err) = cmd_handoff(&args) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        "handoff-file" => {
            if let Err(err) = cmd_handoff_file(&args) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        "export-md" => {
            if let Err(err) = cmd_export_md(&args) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("unknown command");
            std::process::exit(2);
        }
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn temp_coordinator() -> Result<Coordinator, String> {
    let tmp = env::temp_dir().join(format!("unveil-ctl-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    Ok(Coordinator::with_knowledge(tmp.join("ws"), knowledge_dir()))
}

fn cmd_probe(args: &[String]) -> Result<(), String> {
    let path = flag(args, "--path").ok_or("--path required")?;
    let name = flag(args, "--name").unwrap_or_else(|| {
        std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("input.bin")
            .to_string()
    });
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    let coord = temp_coordinator()?;
    let probe = coord.probe_named_bytes(&name, &bytes).map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&probe).unwrap());
    Ok(())
}

fn cmd_usb_drives() -> Result<(), String> {
    let coord = temp_coordinator()?;
    let payload = coord.list_usb_drives().map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    Ok(())
}

fn cmd_usb(args: &[String]) -> Result<(), String> {
    let root = flag(args, "--root").ok_or("--root required")?;
    let workers = flag(args, "--workers")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(4)
        .clamp(1, 8);
    let folder_mode = args.iter().any(|a| a == "--folder-mode")
        || std::env::var("UNVEIL_USB_FOLDER_MODE").ok().as_deref() == Some("1");
    let coord = temp_coordinator()?;
    let job = coord
        .usb_scan_root(std::path::Path::new(&root), workers, folder_mode)
        .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&job).unwrap());
    Ok(())
}

fn cmd_malcheck(args: &[String]) -> Result<(), String> {
    let sample = flag(args, "--sample").ok_or("--sample required")?;
    let coord = temp_coordinator()?;
    let job = coord
        .malcheck_sample(std::path::Path::new(&sample))
        .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&job).unwrap());
    Ok(())
}

fn cmd_handoff(args: &[String]) -> Result<(), String> {
    let root = flag(args, "--root").ok_or("--root required")?;
    let sample = flag(args, "--sample").ok_or("--sample required")?;
    let coord = temp_coordinator()?;
    let job = coord
        .handoff_usb_to_malware(std::path::Path::new(&root), std::path::Path::new(&sample))
        .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&job).unwrap());
    Ok(())
}

fn cmd_handoff_file(args: &[String]) -> Result<(), String> {
    let root = flag(args, "--root").ok_or("--root required")?;
    let name = flag(args, "--name").ok_or("--name required")?;
    let coord = temp_coordinator()?;
    let workers = flag(args, "--workers")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 8);
    let folder_mode = args.iter().any(|a| a == "--folder-mode")
        || std::env::var("UNVEIL_USB_FOLDER_MODE").ok().as_deref() == Some("1");
    let usb = coord
        .usb_scan_root(std::path::Path::new(&root), workers, folder_mode)
        .map_err(|e| e.to_string())?;
    let job = coord
        .handoff_usb_file(&usb.job_id, &name)
        .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&job).unwrap());
    Ok(())
}

fn cmd_export_md(args: &[String]) -> Result<(), String> {
    let module = flag(args, "--module").ok_or("--module required")?;
    let coord = temp_coordinator()?;
    if module == "deobfuscation" {
        let lesson = coord.import_lesson().map_err(|e| e.to_string())?;
        let _ = coord.start_analysis(&lesson.artifact_id).map_err(|e| e.to_string())?;
    }
    let body = coord.export_markdown(&module).map_err(|e| e.to_string())?;
    if let Some(out) = flag(args, "--out") {
        coord
            .write_markdown_export(&module, std::path::Path::new(&out))
            .map_err(|e| e.to_string())?;
    }
    print!("{body}");
    Ok(())
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

    let lesson = coord.import_lesson().map_err(|e| e.to_string())?;
    let lesson_bytes = read_blob(&tmp.join("ws"), &lesson)?;
    if lesson.sha256.len() != 64 {
        return Err("sha256".into());
    }

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

    let hello = coord
        .import_bytes(b"Hello, UNVEIL!", "hello.txt")
        .map_err(|e| e.to_string())?;
    let hello_a = coord
        .start_analysis(&hello.artifact_id)
        .map_err(|e| e.to_string())?;
    if hello_a.assessment != Assessment::NoIndicators {
        return Err(format!("hello assessment {:?}", hello_a.assessment));
    }

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

    let concat = coord
        .import_bytes(b"const a = \"He\" + \"llo\";\n", "concat.js")
        .map_err(|e| e.to_string())?;
    let concat_a = coord
        .start_analysis(&concat.artifact_id)
        .map_err(|e| e.to_string())?;
    if concat_a.assessment != Assessment::IndicatorsPresent {
        return Err("concat should be indicators_present".into());
    }

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
