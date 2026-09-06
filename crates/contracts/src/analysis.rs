use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Full,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Assessment {
    IndicatorsPresent,
    NoIndicators,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Identification {
    Confirmed,
    Probable,
    Possible,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Transformability {
    Supported,
    Partial,
    NeedsParameters,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Partial,
    Failed,
    Cancelling,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Import,
    Analyze,
    Preview,
    Transform,
    Export,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    pub fn contains_ok(&self, len: u64) -> bool {
        self.start <= self.end && self.end <= len
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Score {
    pub value: u8,
    pub kind: String,
    pub rule_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Evidence {
    pub id: String,
    pub source_rule: String,
    pub range: ByteRange,
    pub observation: String,
    pub polarity: String,
    pub group: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub id: String,
    pub artifact_id: String,
    pub technique_id: String,
    pub category: String,
    pub range: ByteRange,
    pub identification: Identification,
    pub score: Score,
    pub evidence: Vec<Evidence>,
    pub transformability: Transformability,
    pub limitations: Vec<String>,
    pub article_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Feature {
    pub id: String,
    pub range: ByteRange,
    pub extractor_id: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coverage {
    pub status: CoverageStatus,
    pub checked_rules: Vec<String>,
    pub skipped: Vec<String>,
    pub inspected_bytes: u64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisResult {
    pub job_id: String,
    pub artifact_id: String,
    pub sha256: String,
    pub coverage: Coverage,
    pub assessment: Assessment,
    pub findings: Vec<Finding>,
    pub features: Vec<Feature>,
    pub limitations: Vec<String>,
    pub probe_kind: String,
    pub text_preview: String,
    pub versions: AnalysisVersions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalysisVersions {
    pub schema_version: String,
    pub rule_version: String,
    pub knowledge_pack: String,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobSnapshot {
    pub job_id: String,
    pub session_id: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub stage: String,
    pub artifact_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactView {
    pub artifact_id: String,
    pub sha256: String,
    pub byte_length: u64,
    pub kind_hint: String,
    pub display_name: String,
    pub parent_artifact_id: Option<String>,
    pub created_by: String,
    pub preview: String,
    pub is_original: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonPage {
    pub left_id: String,
    pub right_id: String,
    pub left_preview: String,
    pub right_preview: String,
    pub mapping_mode: String,
    pub notes: Vec<String>,
}
