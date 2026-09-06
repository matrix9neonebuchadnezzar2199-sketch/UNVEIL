use serde::{Deserialize, Serialize};

use crate::ErrorCode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IsolationCheck {
    pub id: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IsolationDiagnosis {
    pub available: bool,
    pub platform: String,
    pub error_code: Option<ErrorCode>,
    pub checks: Vec<IsolationCheck>,
}

impl IsolationDiagnosis {
    pub fn fail_closed(platform: impl Into<String>, checks: Vec<IsolationCheck>) -> Self {
        Self {
            available: false,
            platform: platform.into(),
            error_code: Some(ErrorCode::SandboxUnavailable),
            checks,
        }
    }

    pub fn ok(platform: impl Into<String>, checks: Vec<IsolationCheck>) -> Self {
        Self {
            available: true,
            platform: platform.into(),
            error_code: None,
            checks,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IsolationSelfTestReport {
    pub child_spawn_denied: bool,
    pub network_denied: bool,
    pub extra_read_denied: bool,
    pub extra_write_denied: bool,
    pub notes: Vec<String>,
}
