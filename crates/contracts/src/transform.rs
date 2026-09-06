use serde::{Deserialize, Serialize};

use crate::{ByteRange, Finding};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Draft,
    Previewing,
    AwaitingApproval,
    Approved,
    Applying,
    Applied,
    Expired,
    Rejected,
    Invalidated,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransformResultKind {
    Transformed,
    Partial,
    NoChange,
    Rejected,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationVerdict {
    Passed,
    Failed,
    NotChecked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationCheck {
    pub kind: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Validation {
    pub checks: Vec<ValidationCheck>,
    pub verdict: ValidationVerdict,
    pub limitations: Vec<String>,
    pub expected_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformPlan {
    pub plan_id: String,
    pub session_id: String,
    pub input_artifact_id: String,
    pub input_hash: String,
    pub range: ByteRange,
    pub adapter_id: String,
    pub technique_id: String,
    pub parameters: serde_json::Value,
    pub plan_hash: String,
    pub state: PlanState,
    pub warnings: Vec<String>,
    pub preconditions: Vec<String>,
    pub lossy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Approval {
    pub approval_id: String,
    pub plan_hash: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformPreview {
    pub plan_id: String,
    pub candidate_sha256: String,
    pub candidate_preview: String,
    pub byte_length: u64,
    pub validation: Validation,
    pub result: TransformResultKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransformApplyResult {
    pub plan_id: String,
    pub step_id: String,
    pub output: crate::ImportedArtifact,
    pub validation: Validation,
    pub result: TransformResultKind,
    pub remaining_findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DagNode {
    pub artifact_id: String,
    pub sha256: String,
    pub label: String,
    pub parent_artifact_id: Option<String>,
    pub adapter_id: Option<String>,
}
