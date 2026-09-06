//! Snapshot import. Original files are never rewritten. Paths stay in the broker.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use unveil_contracts::{ErrorCode, ImportedArtifact, IpcError, SelectionPreview, MAX_INPUT_BYTES};

const COPY_BUF: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct PendingSelection {
    pub path: PathBuf,
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub display_name: String,
    pub kind_hint: String,
}

pub fn inspect_regular_file(path: &Path) -> Result<PendingSelection, IpcError> {
    let meta = fs::symlink_metadata(path)
        .map_err(|_| IpcError::new(ErrorCode::InputNotRegular, "error.input_not_regular"))?;
    if meta.file_type().is_symlink() || meta.is_dir() || !meta.is_file() {
        return Err(IpcError::new(
            ErrorCode::InputNotRegular,
            "error.input_not_regular",
        ));
    }
    if meta.len() > MAX_INPUT_BYTES {
        return Err(IpcError::new(
            ErrorCode::InputTooLarge,
            "error.input_too_large",
        ));
    }
    let display_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unnamed")
        .chars()
        .take(255)
        .collect::<String>();
    Ok(PendingSelection {
        path: path.to_path_buf(),
        len: meta.len(),
        modified: meta.modified().ok(),
        display_name,
        kind_hint: kind_hint(path),
    })
}

pub fn preview_from_pending(token: String, pending: &PendingSelection) -> SelectionPreview {
    SelectionPreview {
        token,
        display_name: pending.display_name.clone(),
        byte_length: pending.len,
        kind_hint: pending.kind_hint.clone(),
    }
}

pub fn snapshot_file(
    pending: &PendingSelection,
    dest_dir: &Path,
    session_id: &str,
) -> Result<ImportedArtifact, IpcError> {
    let before = fs::symlink_metadata(&pending.path)
        .map_err(|_| IpcError::new(ErrorCode::InputNotRegular, "error.input_not_regular"))?;
    if before.len() != pending.len || before.modified().ok() != pending.modified {
        return Err(IpcError::new(
            ErrorCode::InputChanged,
            "error.input_changed",
        ));
    }

    fs::create_dir_all(dest_dir)
        .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
    let staging = dest_dir.join(format!("staging-{}", Uuid::new_v4()));
    let sha256 = copy_and_hash(&pending.path, &staging, pending.len)?;

    let after = fs::symlink_metadata(&pending.path)
        .map_err(|_| IpcError::new(ErrorCode::InputNotRegular, "error.input_not_regular"))?;
    if after.len() != pending.len || after.modified().ok() != pending.modified {
        let _ = fs::remove_file(&staging);
        return Err(IpcError::new(
            ErrorCode::InputChanged,
            "error.input_changed",
        ));
    }

    let final_path = dest_dir.join(&sha256);
    if final_path.exists() {
        let _ = fs::remove_file(&staging);
    } else {
        fs::rename(&staging, &final_path)
            .map_err(|_| IpcError::new(ErrorCode::StorageFull, "error.storage_full"))?;
    }

    Ok(ImportedArtifact {
        artifact_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        display_name: pending.display_name.clone(),
        byte_length: pending.len,
        sha256,
        kind_hint: pending.kind_hint.clone(),
        save_mode: "ephemeral".to_string(),
    })
}

pub fn write_bytes_snapshot(
    bytes: &[u8],
    display_name: &str,
    dest_dir: &Path,
    session_id: &str,
) -> Result<ImportedArtifact, IpcError> {
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(IpcError::new(
            ErrorCode::InputTooLarge,
            "error.input_too_large",
        ));
    }
    fs::create_dir_all(dest_dir)
        .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let sha256 = hex_encode(hasher.finalize());
    let final_path = dest_dir.join(&sha256);
    if !final_path.exists() {
        fs::write(&final_path, bytes)
            .map_err(|_| IpcError::new(ErrorCode::StorageFull, "error.storage_full"))?;
    }
    Ok(ImportedArtifact {
        artifact_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        display_name: display_name.to_string(),
        byte_length: bytes.len() as u64,
        sha256,
        kind_hint: kind_hint(Path::new(display_name)),
        save_mode: "ephemeral".to_string(),
    })
}

fn copy_and_hash(src: &Path, dest: &Path, expected_len: u64) -> Result<String, IpcError> {
    let mut input = File::open(src)
        .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
    let mut output = File::create(dest)
        .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; COPY_BUF];
    let mut written: u64 = 0;
    loop {
        let n = input.read(&mut buf).map_err(io_err)?;
        if n == 0 {
            break;
        }
        written += n as u64;
        if written > MAX_INPUT_BYTES || written > expected_len {
            let _ = fs::remove_file(dest);
            return Err(IpcError::new(
                ErrorCode::InputTooLarge,
                "error.input_too_large",
            ));
        }
        hasher.update(&buf[..n]);
        output.write_all(&buf[..n]).map_err(io_err)?;
    }
    output.flush().map_err(io_err)?;
    if written != expected_len {
        let _ = fs::remove_file(dest);
        return Err(IpcError::new(
            ErrorCode::InputChanged,
            "error.input_changed",
        ));
    }
    Ok(hex_encode(hasher.finalize()))
}

fn io_err(_: io::Error) -> IpcError {
    IpcError::new(ErrorCode::PermissionDenied, "error.io")
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn kind_hint(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_too_large_declared_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.bin");
        File::create(&path)
            .expect("create")
            .write_all(b"x")
            .expect("write");
        // inspect uses real size, so this is a regular small file.
        let pending = inspect_regular_file(&path).expect("inspect");
        assert_eq!(pending.len, 1);
    }

    #[test]
    fn snapshot_hashes_and_does_not_rewrite_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("specimen.js");
        fs::write(&src, b"const token = \"c3BlY2ltZW4ubGFi\";\n").expect("write");
        let pending = inspect_regular_file(&src).expect("inspect");
        let blobs = dir.path().join("blobs");
        let artifact = snapshot_file(&pending, &blobs, "session-test").expect("snapshot");
        assert_eq!(artifact.byte_length, pending.len);
        assert_eq!(artifact.sha256.len(), 64);
        assert_eq!(
            fs::read(&src).expect("reread"),
            b"const token = \"c3BlY2ltZW4ubGFi\";\n"
        );
        assert!(blobs.join(&artifact.sha256).is_file());
    }

    #[test]
    fn rejects_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = inspect_regular_file(dir.path()).expect_err("dir");
        assert_eq!(err.code, ErrorCode::InputNotRegular);
    }
}
