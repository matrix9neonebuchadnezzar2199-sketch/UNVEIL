//! Python sidecars for USB-GuardDuty and MalCheck. Output is untrusted JSON.

use std::path::Path;
use std::process::{Child, Command, Stdio};

use uuid::Uuid;

use unveil_contracts::{ErrorCode, IpcError, ModuleJob};

use crate::modules::{cursor_root, python_bin, repo_root, run_python_json, sidecar_dir};

pub struct UsbScanOptions {
    pub workers: u32,
    pub folder_mode: bool,
}

pub fn list_usb_drives() -> Result<serde_json::Value, IpcError> {
    let script = sidecar_dir().join("usb_sidecar.py");
    if !script.is_file() {
        return Err(IpcError::new(ErrorCode::ModuleUnavailable, "error.module_unavailable"));
    }
    run_python_json(&script, &["--list-drives"], 30)
        .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))
}

pub fn scan_usb_readonly(
    root: &Path,
    opts: &UsbScanOptions,
    child_slot: &mut Option<Child>,
) -> Result<serde_json::Value, IpcError> {
    let script = sidecar_dir().join("usb_sidecar.py");
    if !script.is_file() {
        return Err(IpcError::new(ErrorCode::ModuleUnavailable, "error.module_unavailable"));
    }
    let root_arg = root.to_string_lossy();
    let workers = opts.workers.clamp(1, 8).to_string();
    let mut args = vec![
        "--root",
        root_arg.as_ref(),
        "--readonly",
        "--workers",
        workers.as_str(),
    ];
    if opts.folder_mode {
        args.push("--folder-mode");
    }
    let mut cmd = Command::new(python_bin());
    cmd.arg(&script).args(&args);
    cmd.env("UNVEIL_ROOT", repo_root());
    cmd.env("UNVEIL_CURSOR_ROOT", cursor_root());
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd
        .spawn()
        .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
    *child_slot = Some(child);
    let output = child_slot
        .take()
        .expect("child")
        .wait_with_output()
        .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .rev()
        .find(|l| l.trim().starts_with('{'))
        .ok_or_else(|| IpcError::new(ErrorCode::SchemaMismatch, "error.schema_mismatch"))?;
    let payload: serde_json::Value = serde_json::from_str(line)
        .map_err(|_| IpcError::new(ErrorCode::SchemaMismatch, "error.schema_mismatch"))?;
    if payload.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        return Err(IpcError::new(ErrorCode::ModuleUnavailable, "error.module_unavailable"));
    }
    if !output.status.success() {
        return Err(IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"));
    }
    Ok(payload)
}

pub fn usb_folder_mode_from_env() -> bool {
    std::env::var("UNVEIL_USB_FOLDER_MODE")
        .ok()
        .as_deref()
        == Some("1")
}

pub fn run_malcheck(sample: &Path, ghidra_eligible: bool) -> Result<ModuleJob, IpcError> {
    let script = sidecar_dir().join("malcheck_sidecar.py");
    if !script.is_file() {
        return Err(IpcError::new(
            ErrorCode::ModuleUnavailable,
            "error.module_unavailable",
        ));
    }
    let eligible_flag = if ghidra_eligible { "true" } else { "false" };
    let payload = run_python_json(
        &script,
        &[
            "--sample",
            sample.to_string_lossy().as_ref(),
            "--ghidra-eligible",
            eligible_flag,
        ],
        600,
    )
    .map_err(|_| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
    if payload.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        return Err(IpcError::new(
            ErrorCode::ModuleUnavailable,
            "error.module_unavailable",
        ));
    }
    if !schema_is_21(&payload) || !dynamic_skipped(&payload) {
        return Err(IpcError::new(ErrorCode::SchemaMismatch, "error.schema_mismatch"));
    }
    Ok(ModuleJob {
        job_id: Uuid::new_v4().to_string(),
        module_id: "malware".into(),
        input_artifact_id: None,
        sha256: payload
            .pointer("/report/phase1_surface/hashes/sha256")
            .and_then(|v| v.as_str())
            .or_else(|| payload.get("sha256").and_then(|v| v.as_str()))
            .map(str::to_string),
        status: "completed".into(),
        payload,
    })
}

pub fn schema_is_21(payload: &serde_json::Value) -> bool {
    payload
        .pointer("/report/meta/schema_version")
        .or_else(|| payload.pointer("/meta/schema_version"))
        .and_then(|v| v.as_str())
        == Some("2.1")
}

pub fn dynamic_skipped(payload: &serde_json::Value) -> bool {
    let dynv = payload
        .pointer("/report/phase2_dynamic")
        .or_else(|| payload.pointer("/phase2_dynamic"));
    dynv.and_then(|v| v.get("status"))
        .and_then(|s| s.as_str())
        == Some("skipped")
}
