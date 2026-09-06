use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    SandboxUnavailable,
    InputNotRegular,
    InputChanged,
    InputTooLarge,
    LimitReached,
    WorkerCrashed,
    Interrupted,
    SchemaMismatch,
    PermissionDenied,
    StorageFull,
    Cancelled,
    NotFound,
    AdapterUnsupported,
    PlanStale,
    ApprovalExpired,
    ValidationFailed,
    ParseFailed,
    InvalidEncoding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcError {
    pub code: ErrorCode,
    pub message_key: String,
}

impl IpcError {
    pub fn new(code: ErrorCode, message_key: impl Into<String>) -> Self {
        Self {
            code,
            message_key: message_key.into(),
        }
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:?})", self.message_key, self.code)
    }
}

impl std::error::Error for IpcError {}
