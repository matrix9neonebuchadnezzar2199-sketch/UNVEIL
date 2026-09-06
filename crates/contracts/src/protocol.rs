use serde::{Deserialize, Serialize};

use crate::{AnalysisResult, Finding, IsolationSelfTestReport, IpcError, TransformPreview, SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub schema_version: String,
    pub op: String,
    pub artifact_id: String,
    pub job_id: String,
    pub adapter_id: Option<String>,
    pub finding: Option<Finding>,
    pub canary_path: Option<String>,
}

impl WorkerRequest {
    pub fn analyze(artifact_id: &str, job_id: &str) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(),
            op: "analyze".into(),
            artifact_id: artifact_id.into(),
            job_id: job_id.into(),
            adapter_id: None,
            finding: None,
            canary_path: None,
        }
    }

    pub fn transform(artifact_id: &str, job_id: &str, adapter_id: &str, finding: Finding) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(),
            op: "transform".into(),
            artifact_id: artifact_id.into(),
            job_id: job_id.into(),
            adapter_id: Some(adapter_id.into()),
            finding: Some(finding),
            canary_path: None,
        }
    }

    pub fn self_test(canary_path: Option<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(),
            op: "self_test".into(),
            artifact_id: String::new(),
            job_id: String::new(),
            adapter_id: None,
            finding: None,
            canary_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub schema_version: String,
    pub ok: bool,
    pub op: String,
    pub analysis: Option<AnalysisResult>,
    pub preview: Option<TransformPreview>,
    pub output_sha256: Option<String>,
    pub output_len: Option<u64>,
    pub output_b64: Option<String>,
    pub self_test: Option<IsolationSelfTestReport>,
    pub error: Option<IpcError>,
}

pub fn write_frame(writer: &mut impl std::io::Write, bytes: &[u8]) -> std::io::Result<()> {
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()
}

pub fn read_frame(reader: &mut impl std::io::Read, max: usize) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > max {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}
