//! Shared schema types. Worker JSON is untrusted until the coordinator re-validates it.

mod analysis;
mod error;
mod isolation;
mod modules;
mod protocol;
mod transform;
mod wiki;

pub use analysis::*;
pub use error::*;
pub use isolation::*;
pub use modules::*;
pub use protocol::*;
pub use transform::*;
pub use wiki::*;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1.0";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PHASE: &str = "06-mvp";
pub const MAX_INPUT_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_TEXT_PARSE_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_PREVIEW_BYTES: u64 = 200 * 1024;
pub const MAX_EXCERPT_BYTES: usize = 2048;
pub const MAX_GZIP_OUTPUT: u64 = 50 * 1024 * 1024;
pub const MAX_GZIP_RATIO: u64 = 100;
pub const MAX_CHAIN_DEPTH: u32 = 5;
pub const MAX_TRANSFORM_TRIES: u32 = 20;
pub const JOB_WALL_SECS: u64 = 60;
pub const TRANSFORM_WALL_SECS: u64 = 10;
pub const RULE_VERSION: &str = "1.0.0";
pub const KNOWLEDGE_PACK: &str = "1.0.0";
pub const PROFILE_STATIC_SAFE: &str = "static-safe-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub schema_version: String,
    pub phase: String,
    pub knowledge_pack: String,
    pub rule_version: String,
}

impl AppInfo {
    pub fn current() -> Self {
        Self {
            name: "UNVEIL".to_string(),
            version: APP_VERSION.to_string(),
            schema_version: SCHEMA_VERSION.to_string(),
            phase: PHASE.to_string(),
            knowledge_pack: KNOWLEDGE_PACK.to_string(),
            rule_version: RULE_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectionPreview {
    pub token: String,
    pub display_name: String,
    pub byte_length: u64,
    pub kind_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportedArtifact {
    pub artifact_id: String,
    pub session_id: String,
    pub display_name: String,
    pub byte_length: u64,
    pub sha256: String,
    pub kind_hint: String,
    pub save_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NamedCount {
    pub id: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionRow {
    pub session_id: String,
    pub name: String,
    pub findings: u32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardOverview {
    pub saved_sessions: u32,
    pub jobs_total: u32,
    pub findings_total: u32,
    pub isolation_passed: u32,
    pub categories: Vec<NamedCount>,
    pub techniques: Vec<NamedCount>,
    pub job_outcomes: Vec<NamedCount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub module_jobs: Vec<NamedCount>,
    pub sessions: Vec<SessionRow>,
}

impl DashboardOverview {
    pub fn empty() -> Self {
        Self {
            saved_sessions: 0,
            jobs_total: 0,
            findings_total: 0,
            isolation_passed: 0,
            categories: Vec::new(),
            techniques: Vec::new(),
            job_outcomes: Vec::new(),
            module_jobs: Vec::new(),
            sessions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetectorId {
    EncBase64,
    EncBase64Url,
    EncBase16,
    EncPercent,
    EncJsEscape,
    CompressionGzip,
    LayoutMinifyJs,
    DataLiteralConcat,
    ControlDynamicEval,
    PeElfPacker,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectorToggle {
    pub id: DetectorId,
    pub enabled: bool,
    pub mvp: bool,
    pub summary: String,
}

impl DetectorToggle {
    pub fn mvp_set() -> Vec<Self> {
        vec![
            Self {
                id: DetectorId::EncBase64,
                enabled: true,
                mvp: true,
                summary: "字種・padding・strict decode の往復。短い一致は難読化陽性にしない。".into(),
            },
            Self {
                id: DetectorId::EncBase64Url,
                enabled: true,
                mvp: true,
                summary: "URL-safe 字種。通常 Base64 と両立する場合は確定しない。".into(),
            },
            Self {
                id: DetectorId::EncBase16,
                enabled: true,
                mvp: true,
                summary: "偶数長 hex。ハッシュ表記の可能性を併記する。".into(),
            },
            Self {
                id: DetectorId::EncPercent,
                enabled: true,
                mvp: true,
                summary: "%HH 構造。+ を空白へ変えない。".into(),
            },
            Self {
                id: DetectorId::EncJsEscape,
                enabled: true,
                mvp: true,
                summary: "JS 文字列内の \\x / \\u。任意本文には当てない。".into(),
            },
            Self {
                id: DetectorId::CompressionGzip,
                enabled: true,
                mvp: true,
                summary: "マジックと上限付き単一ストリーム。爆弾は LIMIT_REACHED。".into(),
            },
            Self {
                id: DetectorId::LayoutMinifyJs,
                enabled: true,
                mvp: true,
                summary: "行長と空白比。整形のみ。元の識別子は戻せない。".into(),
            },
            Self {
                id: DetectorId::DataLiteralConcat,
                enabled: true,
                mvp: true,
                summary: "純粋な文字列リテラル同士の + のみ連結。".into(),
            },
            Self {
                id: DetectorId::ControlDynamicEval,
                enabled: true,
                mvp: true,
                summary: "eval 等の構文の存在。実行しない。".into(),
            },
            Self {
                id: DetectorId::PeElfPacker,
                enabled: false,
                mvp: false,
                summary: "PHASE 07。基本マジックのみ。内部解析しない。".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_roundtrip() {
        let info = AppInfo::current();
        let json = serde_json::to_string(&info).expect("serialize");
        let parsed: AppInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.schema_version, SCHEMA_VERSION);
        assert_eq!(parsed.phase, PHASE);
    }

    #[test]
    fn error_code_serializes_as_screaming_snake() {
        let json = serde_json::to_string(&ErrorCode::SandboxUnavailable).expect("serialize");
        assert_eq!(json, "\"SANDBOX_UNAVAILABLE\"");
    }
}
