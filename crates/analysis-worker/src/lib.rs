use unveil_contracts::{
    IsolationSelfTestReport, WorkerRequest, WorkerResponse, SCHEMA_VERSION, read_frame,
};
use unveil_engine::{analyze, apply_adapter, preview_from_finding, sha256_hex};

pub fn dispatch(req: &WorkerRequest, body: &[u8]) -> WorkerResponse {
    if req.schema_version != SCHEMA_VERSION {
        return fail(
            &req.op,
            unveil_contracts::ErrorCode::SchemaMismatch,
            "error.schema_mismatch",
        );
    }
    match req.op.as_str() {
        "self_test" => WorkerResponse {
            schema_version: SCHEMA_VERSION.into(),
            ok: true,
            op: req.op.clone(),
            analysis: None,
            preview: None,
            output_sha256: None,
            output_len: None,
            output_b64: None,
            self_test: Some(run_self_test(req.canary_path.as_deref())),
            error: None,
        },
        "analyze" => WorkerResponse {
            schema_version: SCHEMA_VERSION.into(),
            ok: true,
            op: req.op.clone(),
            analysis: Some(analyze(&req.artifact_id, &req.job_id, body)),
            preview: None,
            output_sha256: None,
            output_len: None,
            output_b64: None,
            self_test: None,
            error: None,
        },
        "transform" => {
            let Some(finding) = req.finding.as_ref() else {
                return fail(&req.op, unveil_contracts::ErrorCode::ParseFailed, "error.missing_finding");
            };
            let Some(adapter) = req.adapter_id.as_deref() else {
                return fail(
                    &req.op,
                    unveil_contracts::ErrorCode::AdapterUnsupported,
                    "error.adapter_unsupported",
                );
            };
            match apply_adapter(body, finding, adapter) {
                Ok(out) => WorkerResponse {
                    schema_version: SCHEMA_VERSION.into(),
                    ok: true,
                    op: req.op.clone(),
                    analysis: None,
                    preview: preview_from_finding(body, finding, adapter).ok(),
                    output_sha256: Some(sha256_hex(&out.bytes)),
                    output_len: Some(out.bytes.len() as u64),
                    output_b64: Some(encode_b64(&out.bytes)),
                    self_test: None,
                    error: None,
                },
                Err(err) => WorkerResponse {
                    schema_version: SCHEMA_VERSION.into(),
                    ok: false,
                    op: req.op.clone(),
                    analysis: None,
                    preview: None,
                    output_sha256: None,
                    output_len: None,
                    output_b64: None,
                    self_test: None,
                    error: Some(err),
                },
            }
        }
        _ => fail(&req.op, unveil_contracts::ErrorCode::ParseFailed, "error.unknown_op"),
    }
}

fn fail(op: &str, code: unveil_contracts::ErrorCode, key: &str) -> WorkerResponse {
    WorkerResponse {
        schema_version: SCHEMA_VERSION.into(),
        ok: false,
        op: op.to_string(),
        analysis: None,
        preview: None,
        output_sha256: None,
        output_len: None,
        output_b64: None,
        self_test: None,
        error: Some(unveil_contracts::IpcError::new(code, key)),
    }
}

fn encode_b64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if i + 1 < bytes.len() {
            T[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if i + 2 < bytes.len() {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
        i += 3;
    }
    out
}

pub fn run_self_test(canary: Option<&str>) -> IsolationSelfTestReport {
    let mut notes = Vec::new();
    let child_spawn_denied = match std::process::Command::new(
        std::env::current_exe().unwrap_or_default(),
    )
    .arg("--isolation-self-test-child")
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .spawn()
    {
        Ok(mut child) => {
            let _ = child.kill();
            notes.push("child spawn unexpectedly succeeded".into());
            false
        }
        Err(err) => {
            notes.push(format!("child spawn denied: {err}"));
            true
        }
    };

    let addr = std::net::SocketAddr::from(([192, 0, 2, 1], 80));
    let network_denied =
        match std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(800)) {
            Ok(_) => {
                notes.push("connect to TEST-NET-1 succeeded".into());
                false
            }
            Err(err) => {
                notes.push(format!("connect: {:?} {err}", err.kind()));
                !matches!(err.kind(), std::io::ErrorKind::TimedOut)
            }
        };

    let mut extra_read_denied = true;
    let mut extra_write_denied = true;
    if let Some(path) = canary {
        match std::fs::read(path) {
            Ok(_) => {
                extra_read_denied = false;
                notes.push("canary read succeeded".into());
            }
            Err(err) => notes.push(format!("canary read denied: {err}")),
        }
        let write_path = std::path::Path::new(path).with_extension("write-probe");
        match std::fs::write(&write_path, b"probe") {
            Ok(_) => {
                extra_write_denied = false;
                notes.push("canary write succeeded".into());
                let _ = std::fs::remove_file(&write_path);
            }
            Err(err) => notes.push(format!("canary write denied: {err}")),
        }
    }

    IsolationSelfTestReport {
        child_spawn_denied,
        network_denied,
        extra_read_denied,
        extra_write_denied,
        notes,
    }
}

pub fn read_body(reader: &mut impl std::io::Read) -> std::io::Result<Vec<u8>> {
    read_frame(reader, unveil_contracts::MAX_INPUT_BYTES as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_hello_text() {
        let req = WorkerRequest::analyze("a", "j");
        let resp = dispatch(&req, b"Hello, UNVEIL!");
        assert!(resp.ok);
        let analysis = resp.analysis.expect("analysis");
        assert_eq!(
            analysis.assessment,
            unveil_contracts::Assessment::NoIndicators
        );
    }
}
