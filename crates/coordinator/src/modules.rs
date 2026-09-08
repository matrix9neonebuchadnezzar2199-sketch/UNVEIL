//! Manifest load and capability diagnose. Missing capability is fail-closed, not silent fallback.

use std::path::{Path, PathBuf};
use std::process::Command;

use unveil_contracts::{
    CapabilityCheck, ModuleDiagnosis, ModuleManifest, ModuleSpec,
};

pub fn repo_root() -> PathBuf {
    if let Ok(p) = std::env::var("UNVEIL_ROOT") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub fn manifest_path() -> PathBuf {
    if let Ok(p) = std::env::var("UNVEIL_MANIFEST") {
        return PathBuf::from(p);
    }
    let root = repo_root();
    let primary = root.join("modules.manifest.json");
    if primary.is_file() {
        return primary;
    }
    root.join("docs/modules.manifest.example.json")
}

pub fn load_manifest() -> Result<ModuleManifest, String> {
    let path = manifest_path();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn python_bin() -> String {
    std::env::var("UNVEIL_PYTHON").unwrap_or_else(|_| "python".into())
}

pub fn sidecar_dir() -> PathBuf {
    repo_root().join("sidecars")
}

pub fn cursor_root() -> PathBuf {
    repo_root()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(repo_root)
}

fn check_isolation() -> CapabilityCheck {
    let d = crate::diagnose_isolation();
    CapabilityCheck {
        id: "C_isolation".into(),
        available: d.available,
        detail: if d.available {
            "AppContainer+Job".into()
        } else {
            d.error_code
                .map(|c| format!("{c:?}"))
                .unwrap_or_else(|| "unavailable".into())
        },
    }
}

fn ghidra_image_tag() -> String {
    std::env::var("UNVEIL_GHIDRA_IMAGE").unwrap_or_else(|_| "ghidra-headless:latest".into())
}

fn check_docker() -> CapabilityCheck {
    let info = Command::new("docker").arg("info").output();
    let info_ok = info.as_ref().map(|o| o.status.success()).unwrap_or(false);
    if !info_ok {
        return CapabilityCheck {
            id: "C_docker".into(),
            available: false,
            detail: "docker CLI/engine not available".into(),
        };
    }
    let image = ghidra_image_tag();
    let inspect = Command::new("docker")
        .args(["image", "inspect", &image])
        .output();
    let image_ok = inspect.map(|o| o.status.success()).unwrap_or(false);
    CapabilityCheck {
        id: "C_docker".into(),
        available: image_ok,
        detail: if image_ok {
            format!("docker info ok; image {image} present")
        } else {
            format!("Ghidra image not loaded: {image}")
        },
    }
}

fn check_usb_enum() -> CapabilityCheck {
    #[cfg(windows)]
    {
        CapabilityCheck {
            id: "C_usb".into(),
            available: true,
            detail: "Win32 removable enum (drive list may be empty)".into(),
        }
    }
    #[cfg(not(windows))]
    {
        CapabilityCheck {
            id: "C_usb".into(),
            available: false,
            detail: "USB enumeration is Windows-only".into(),
        }
    }
}

fn check_magika() -> CapabilityCheck {
    let script = sidecar_dir().join("magika_probe.py");
    if !script.is_file() {
        return CapabilityCheck {
            id: "C_magika".into(),
            available: false,
            detail: "sidecars/magika_probe.py missing".into(),
        };
    }
    let out = Command::new(python_bin())
        .env("UNVEIL_ROOT", repo_root())
        .args([script.to_string_lossy().as_ref(), "--self-test"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout);
            let available = text.contains("\"status\": \"ok\"");
            CapabilityCheck {
                id: "C_magika".into(),
                available,
                detail: if available {
                    "Magika model loaded".into()
                } else {
                    "capability=unavailable (no silent libmagic fallback)".into()
                },
            }
        }
        _ => CapabilityCheck {
            id: "C_magika".into(),
            available: false,
            detail: "magika probe failed to start".into(),
        },
    }
}

fn check_yara() -> CapabilityCheck {
    let ugd = cursor_root().join("USB-GuardDuty").join("backend").join("rules").join("eicar.yar");
    CapabilityCheck {
        id: "C_yara".into(),
        available: ugd.is_file(),
        detail: if ugd.is_file() {
            "bundled eicar.yar present".into()
        } else {
            "YARA rules not found".into()
        },
    }
}

fn check_malcheck_surface() -> CapabilityCheck {
    let mau = cursor_root().join("MalCheck").join("mau").join("phase_router.py");
    CapabilityCheck {
        id: "C_malcheck_surface".into(),
        available: mau.is_file(),
        detail: if mau.is_file() {
            "MalCheck mau/ present".into()
        } else {
            "MalCheck tree not found".into()
        },
    }
}

fn check_sidecar_malcheck() -> CapabilityCheck {
    let script = sidecar_dir().join("malcheck_sidecar.py");
    let mau = cursor_root().join("MalCheck");
    let ok = script.is_file() && mau.join("mau").is_dir();
    CapabilityCheck {
        id: "C_sidecar_malcheck".into(),
        available: ok,
        detail: if ok {
            "malcheck_sidecar.py + MalCheck/mau".into()
        } else {
            "malcheck sidecar or mau/ missing".into()
        },
    }
}

fn check_sidecar_usb() -> CapabilityCheck {
    let script = sidecar_dir().join("usb_sidecar.py");
    let pipeline = cursor_root()
        .join("USB-GuardDuty")
        .join("backend")
        .join("pipeline.py");
    let ok = script.is_file() && pipeline.is_file();
    CapabilityCheck {
        id: "C_sidecar_usb".into(),
        available: ok,
        detail: if ok {
            "usb_sidecar.py + USB-GuardDuty pipeline".into()
        } else {
            "usb sidecar or UGD pipeline missing".into()
        },
    }
}

pub fn check_capability(id: &str) -> CapabilityCheck {
    match id {
        "C_isolation" => check_isolation(),
        "C_docker" => check_docker(),
        "C_usb" => check_usb_enum(),
        "C_magika" => check_magika(),
        "C_yara" => check_yara(),
        "C_malcheck_surface" => check_malcheck_surface(),
        "C_sidecar_malcheck" => check_sidecar_malcheck(),
        "C_sidecar_usb" => check_sidecar_usb(),
        other => CapabilityCheck {
            id: other.into(),
            available: false,
            detail: "unknown capability".into(),
        },
    }
}

fn required_for_jobs(spec: &ModuleSpec) -> Vec<String> {
    match spec.id.as_str() {
        "dashboard" => Vec::new(),
        "deobfuscation" => vec!["C_isolation".into()],
        "malware" => vec!["C_isolation".into(), "C_sidecar_malcheck".into()],
        "usb" => vec!["C_isolation".into(), "C_sidecar_usb".into()],
        _ => spec.capabilities.clone(),
    }
}

pub fn list_modules_json() -> Result<serde_json::Value, String> {
    let man = load_manifest()?;
    let modules: Vec<serde_json::Value> = man
        .modules
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "label_ja": m.label_ja,
                "label_en": m.label_en,
                "route": m.route,
                "status": m.status,
                "disabled": m.status == "planned" || m.status == "disabled",
                "capabilities": m.capabilities,
                "worker": m.worker,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "schema_version": man.schema_version,
        "modules": modules,
    }))
}

pub fn diagnose_module(module_id: &str) -> Result<ModuleDiagnosis, String> {
    let man = load_manifest()?;
    let spec = man
        .modules
        .iter()
        .find(|m| m.id == module_id)
        .ok_or_else(|| format!("unknown module {module_id}"))?;
    let mut checks = Vec::new();
    for cap in &spec.capabilities {
        checks.push(check_capability(cap));
    }
    let required = required_for_jobs(spec);
    let available = required.iter().all(|id| {
        checks
            .iter()
            .find(|c| &c.id == id)
            .map(|c| c.available)
            .unwrap_or_else(|| check_capability(id).available)
    });
    Ok(ModuleDiagnosis {
        module_id: spec.id.clone(),
        available,
        status: spec.status.clone(),
        checks,
    })
}

pub fn run_python_json(script: &Path, args: &[&str], timeout_secs: u64) -> Result<serde_json::Value, String> {
    let mut cmd = Command::new(python_bin());
    cmd.arg(script).args(args);
    cmd.env("UNVEIL_ROOT", repo_root());
    cmd.env("UNVEIL_CURSOR_ROOT", cursor_root());
    cmd.env("PYTHONIOENCODING", "utf-8");
    cmd.env("PYTHONUTF8", "1");
    let _ = timeout_secs;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "sidecar exit {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .rev()
        .find(|l| l.trim().starts_with('{'))
        .ok_or_else(|| format!("no json in sidecar stdout: {stdout}"))?;
    serde_json::from_str(line).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_three_work_modules() {
        let man = load_manifest().expect("load");
        let work: Vec<_> = man
            .modules
            .iter()
            .filter(|m| m.id != "dashboard")
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(work, vec!["malware", "deobfuscation", "usb"]);
        let malware = man.modules.iter().find(|m| m.id == "malware").unwrap();
        assert!(matches!(malware.status.as_str(), "planned" | "mvp"));
    }
}
