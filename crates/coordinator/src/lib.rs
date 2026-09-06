//! Coordinator broker. Holds privilege; workers and UI do not.
//!
//! Analysis must not start unless [`diagnose_isolation`] reports `available`.

mod import;
mod isolation;
mod report;
mod store;
mod wiki;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use uuid::Uuid;

use unveil_contracts::{
    AnalysisResult, AppInfo, Approval, ArtifactView, ByteRange, ComparisonPage, DashboardOverview,
    DetectorToggle, ErrorCode, ImportedArtifact, IpcError, JobSnapshot,
    PlanState, ReportDocument, SelectionPreview, TransformApplyResult, TransformPlan,
    TransformPreview, WikiArticle, WikiHit, WorkerRequest, MAX_CHAIN_DEPTH, MAX_TRANSFORM_TRIES,
};

use import::{
    inspect_regular_file, preview_from_pending, snapshot_file, write_bytes_snapshot, PendingSelection,
};
use isolation::run_isolated;
use store::Store;
use wiki::WikiPack;

pub use isolation::{diagnose_isolation, worker_path};
pub use wiki::default_knowledge_dir;

const LESSON_JS: &[u8] = b"const token = \"c3BlY2ltZW4ubGFi\";\nconsole.log(token);\n";

pub fn app_info() -> AppInfo {
    AppInfo::current()
}

struct PlanRecord {
    plan: TransformPlan,
    approval: Option<Approval>,
    preview: Option<TransformPreview>,
    preview_bytes: Option<Vec<u8>>,
}

struct ArtifactRecord {
    imported: ImportedArtifact,
    parent: Option<String>,
    created_by: String,
}

struct Inner {
    workspace: PathBuf,
    session_id: String,
    selections: HashMap<String, PendingSelection>,
    artifacts: HashMap<String, ArtifactRecord>,
    current_id: Option<String>,
    analysis: Option<AnalysisResult>,
    plans: HashMap<String, PlanRecord>,
    tries: u32,
    depth: u32,
    store: Store,
}

pub struct Coordinator {
    inner: Mutex<Inner>,
    knowledge: WikiPack,
}

impl Coordinator {
    pub fn new(workspace: PathBuf) -> Self {
        Self::with_knowledge(workspace, default_knowledge_dir())
    }

    pub fn with_knowledge(workspace: PathBuf, knowledge_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&workspace);
        let store = Store::open(&workspace).expect("sqlite");
        let session_id = Uuid::new_v4().to_string();
        let _ = store.ensure_session(&session_id, "untitled", "ephemeral");
        Self {
            inner: Mutex::new(Inner {
                workspace,
                session_id,
                selections: HashMap::new(),
                artifacts: HashMap::new(),
                current_id: None,
                analysis: None,
                plans: HashMap::new(),
                tries: 0,
                depth: 0,
                store,
            }),
            knowledge: WikiPack::load(knowledge_dir),
        }
    }

    pub fn detectors(&self) -> Vec<DetectorToggle> {
        DetectorToggle::mvp_set()
    }

    pub fn wiki_search(&self, query: &str) -> Vec<WikiHit> {
        self.knowledge.search(query)
    }

    pub fn wiki_get(&self, article_id: &str) -> Result<WikiArticle, IpcError> {
        self.knowledge
            .get(article_id)
            .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.not_found"))
    }

    pub fn wiki_timeline(&self) -> Vec<unveil_contracts::TimelineItem> {
        self.knowledge.timeline.clone()
    }

    pub fn wiki_glossary(&self) -> Vec<unveil_contracts::GlossaryEntry> {
        self.knowledge.glossary.clone()
    }

    pub fn register_path(&self, path: &Path) -> Result<SelectionPreview, IpcError> {
        let pending = inspect_regular_file(path)?;
        let token = Uuid::new_v4().to_string();
        let preview = preview_from_pending(token.clone(), &pending);
        self.inner
            .lock()
            .expect("mutex")
            .selections
            .insert(token, pending);
        Ok(preview)
    }

    pub fn import_token(&self, token: &str) -> Result<ImportedArtifact, IpcError> {
        let mut inner = self.inner.lock().expect("mutex");
        let pending = inner
            .selections
            .remove(token)
            .ok_or_else(|| IpcError::new(ErrorCode::InputNotRegular, "error.selection_expired"))?;
        let dest = blob_dir(&inner);
        let session_id = inner.session_id.clone();
        drop(inner);
        let artifact = snapshot_file(&pending, &dest, &session_id)?;
        self.remember(artifact.clone(), None, "import")
    }

    pub fn import_lesson(&self) -> Result<ImportedArtifact, IpcError> {
        let inner = self.inner.lock().expect("mutex");
        let dest = blob_dir(&inner);
        let session_id = inner.session_id.clone();
        drop(inner);
        let artifact = write_bytes_snapshot(LESSON_JS, "lesson-base64.js", &dest, &session_id)?;
        self.remember(artifact, None, "lesson")
    }

    pub fn import_bytes(&self, bytes: &[u8], name: &str) -> Result<ImportedArtifact, IpcError> {
        let inner = self.inner.lock().expect("mutex");
        let dest = blob_dir(&inner);
        let session_id = inner.session_id.clone();
        drop(inner);
        let artifact = write_bytes_snapshot(bytes, name, &dest, &session_id)?;
        self.remember(artifact, None, "import")
    }

    fn remember(
        &self,
        artifact: ImportedArtifact,
        parent: Option<String>,
        created_by: &str,
    ) -> Result<ImportedArtifact, IpcError> {
        let mut inner = self.inner.lock().expect("mutex");
        let _ = inner.store.insert_artifact(
            &artifact.artifact_id,
            &artifact.session_id,
            &artifact.sha256,
            artifact.byte_length,
            &artifact.display_name,
            &artifact.kind_hint,
            parent.as_deref(),
            created_by,
        );
        inner.current_id = Some(artifact.artifact_id.clone());
        inner.artifacts.insert(
            artifact.artifact_id.clone(),
            ArtifactRecord {
                imported: artifact.clone(),
                parent,
                created_by: created_by.into(),
            },
        );
        inner.analysis = None;
        inner.depth = 0;
        Ok(artifact)
    }

    pub fn current_artifact(&self) -> Option<ImportedArtifact> {
        let inner = self.inner.lock().expect("mutex");
        let id = inner.current_id.as_ref()?;
        inner.artifacts.get(id).map(|a| a.imported.clone())
    }

    pub fn overview(&self) -> DashboardOverview {
        let passed = if diagnose_isolation().available { 1 } else { 0 };
        self.inner.lock().expect("mutex").store.overview(passed)
    }

    pub fn start_analysis(&self, artifact_id: &str) -> Result<AnalysisResult, IpcError> {
        if !diagnose_isolation().available {
            return Err(IpcError::new(
                ErrorCode::SandboxUnavailable,
                "error.sandbox_unavailable",
            ));
        }
        let (bytes, session_id, workspace) = {
            let inner = self.inner.lock().expect("mutex");
            let rec = inner
                .artifacts
                .get(artifact_id)
                .ok_or_else(|| IpcError::new(ErrorCode::InputNotRegular, "error.no_artifact"))?;
            let path = blob_dir(&inner).join(&rec.imported.sha256);
            let bytes = std::fs::read(&path)
                .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
            (bytes, inner.session_id.clone(), inner.workspace.clone())
        };
        let job_id = Uuid::new_v4().to_string();
        let sandbox = workspace.join("staging").join(&job_id);
        let req = WorkerRequest::analyze(artifact_id, &job_id);
        let resp = run_isolated(
            &req,
            &bytes,
            &sandbox,
            None,
            Duration::from_secs(unveil_contracts::JOB_WALL_SECS),
        )?;
        let analysis = resp
            .analysis
            .ok_or_else(|| IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed"))?;
        let mut inner = self.inner.lock().expect("mutex");
        let _ = inner.store.insert_job(
            &job_id,
            &session_id,
            "analyze",
            "completed",
            Some(artifact_id),
            None,
        );
        let _ = inner
            .store
            .replace_findings(&session_id, artifact_id, &analysis.findings);
        inner.analysis = Some(analysis.clone());
        inner.current_id = Some(artifact_id.to_string());
        Ok(analysis)
    }

    pub fn last_analysis(&self) -> Option<AnalysisResult> {
        self.inner.lock().expect("mutex").analysis.clone()
    }

    pub fn artifact_view(&self, artifact_id: &str) -> Result<ArtifactView, IpcError> {
        let inner = self.inner.lock().expect("mutex");
        let rec = inner
            .artifacts
            .get(artifact_id)
            .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.not_found"))?;
        let bytes = std::fs::read(blob_dir(&inner).join(&rec.imported.sha256))
            .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
        let preview = match std::str::from_utf8(
            &bytes[..bytes.len().min(unveil_contracts::MAX_PREVIEW_BYTES as usize)],
        ) {
            Ok(t) => t.replace('\0', "·"),
            Err(_) => hex_preview(&bytes),
        };
        Ok(ArtifactView {
            artifact_id: rec.imported.artifact_id.clone(),
            sha256: rec.imported.sha256.clone(),
            byte_length: rec.imported.byte_length,
            kind_hint: rec.imported.kind_hint.clone(),
            display_name: rec.imported.display_name.clone(),
            parent_artifact_id: rec.parent.clone(),
            created_by: rec.created_by.clone(),
            preview,
            is_original: rec.parent.is_none(),
        })
    }

    pub fn dag(&self) -> Vec<unveil_contracts::DagNode> {
        let inner = self.inner.lock().expect("mutex");
        inner
            .artifacts
            .values()
            .map(|a| unveil_contracts::DagNode {
                artifact_id: a.imported.artifact_id.clone(),
                sha256: a.imported.sha256.clone(),
                label: a.imported.display_name.clone(),
                parent_artifact_id: a.parent.clone(),
                adapter_id: if a.created_by == "import" || a.created_by == "lesson" {
                    None
                } else {
                    Some(a.created_by.clone())
                },
            })
            .collect()
    }

    pub fn comparison(&self, left_id: &str, right_id: &str) -> Result<ComparisonPage, IpcError> {
        let left = self.artifact_view(left_id)?;
        let right = self.artifact_view(right_id)?;
        Ok(ComparisonPage {
            left_id: left.artifact_id,
            right_id: right.artifact_id,
            left_preview: left.preview,
            right_preview: right.preview,
            mapping_mode: "approximate".into(),
            notes: vec![
                "Byte-accurate mapping is omitted when decode expands or contracts.".into(),
            ],
        })
    }

    pub fn plan_from_finding(
        &self,
        artifact_id: &str,
        finding_id: &str,
        adapter_id: Option<&str>,
    ) -> Result<TransformPlan, IpcError> {
        let inner = self.inner.lock().expect("mutex");
        let analysis = inner
            .analysis
            .as_ref()
            .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.no_analysis"))?;
        let finding = analysis
            .findings
            .iter()
            .find(|f| f.id == finding_id && f.artifact_id == artifact_id)
            .cloned()
            .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.not_found"))?;
        let rec = inner
            .artifacts
            .get(artifact_id)
            .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.not_found"))?;
        if inner.tries >= MAX_TRANSFORM_TRIES || inner.depth >= MAX_CHAIN_DEPTH {
            return Err(IpcError::new(ErrorCode::LimitReached, "error.limit_reached"));
        }
        let adapter = adapter_id.unwrap_or(&finding.technique_id).to_string();
        if finding.transformability == unveil_contracts::Transformability::Unsupported {
            return Err(IpcError::new(
                ErrorCode::AdapterUnsupported,
                "error.adapter_unsupported",
            ));
        }
        let plan_id = Uuid::new_v4().to_string();
        let params = serde_json::json!({"adapter": adapter, "rule": unveil_contracts::RULE_VERSION});
        let plan_hash = hash_plan(&rec.imported.sha256, &finding.range, &adapter, &params);
        let plan = TransformPlan {
            plan_id: plan_id.clone(),
            session_id: inner.session_id.clone(),
            input_artifact_id: artifact_id.to_string(),
            input_hash: rec.imported.sha256.clone(),
            range: finding.range.clone(),
            adapter_id: adapter,
            technique_id: finding.technique_id,
            parameters: params,
            plan_hash,
            state: PlanState::Draft,
            warnings: finding.limitations.clone(),
            preconditions: vec!["Isolation must remain available.".into()],
            lossy: matches!(
                finding.transformability,
                unveil_contracts::Transformability::Partial
            ),
        };
        drop(inner);
        let mut inner = self.inner.lock().expect("mutex");
        inner.plans.insert(
            plan_id,
            PlanRecord {
                plan: plan.clone(),
                approval: None,
                preview: None,
                preview_bytes: None,
            },
        );
        Ok(plan)
    }

    pub fn preview_plan(&self, plan_id: &str) -> Result<TransformPreview, IpcError> {
        self.transform_job(plan_id, false)
            .map(|(_, preview)| preview)
    }

    pub fn approve_plan(&self, plan_id: &str, plan_hash: &str) -> Result<Approval, IpcError> {
        let mut inner = self.inner.lock().expect("mutex");
        let rec = inner
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.not_found"))?;
        if rec.plan.plan_hash != plan_hash {
            rec.plan.state = PlanState::Invalidated;
            return Err(IpcError::new(ErrorCode::PlanStale, "error.plan_stale"));
        }
        if rec.preview.is_none() {
            return Err(IpcError::new(ErrorCode::PlanStale, "error.plan_stale"));
        }
        let approval = Approval {
            approval_id: Uuid::new_v4().to_string(),
            plan_hash: plan_hash.to_string(),
            expires_at: rfc3339_plus_mins(10),
        };
        rec.approval = Some(approval.clone());
        rec.plan.state = PlanState::Approved;
        Ok(approval)
    }

    pub fn apply_plan(
        &self,
        plan_id: &str,
        approval_id: &str,
        plan_hash: &str,
    ) -> Result<TransformApplyResult, IpcError> {
        {
            let inner = self.inner.lock().expect("mutex");
            let rec = inner
                .plans
                .get(plan_id)
                .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.not_found"))?;
            if rec.plan.plan_hash != plan_hash {
                return Err(IpcError::new(ErrorCode::PlanStale, "error.plan_stale"));
            }
            let approval = rec
                .approval
                .as_ref()
                .ok_or_else(|| IpcError::new(ErrorCode::ApprovalExpired, "error.approval_expired"))?;
            if approval.approval_id != approval_id {
                return Err(IpcError::new(ErrorCode::ApprovalExpired, "error.approval_expired"));
            }
            if expired(&approval.expires_at) {
                return Err(IpcError::new(ErrorCode::ApprovalExpired, "error.approval_expired"));
            }
        }
        let (bytes, preview) = self.transform_job(plan_id, true)?;
        if preview.validation.verdict == unveil_contracts::ValidationVerdict::Failed {
            return Err(IpcError::new(
                ErrorCode::ValidationFailed,
                "error.validation_failed",
            ));
        }
        let name = {
            let inner = self.inner.lock().expect("mutex");
            let rec = inner.plans.get(plan_id).unwrap();
            format!("{}-{}", rec.plan.adapter_id, &preview.candidate_sha256[..8])
        };
        let parent = {
            let inner = self.inner.lock().expect("mutex");
            inner.plans.get(plan_id).unwrap().plan.input_artifact_id.clone()
        };
        let artifact = {
            let inner = self.inner.lock().expect("mutex");
            let dest = blob_dir(&inner);
            let session_id = inner.session_id.clone();
            drop(inner);
            write_bytes_snapshot(&bytes, &name, &dest, &session_id)?
        };
        let imported = self.remember(artifact, Some(parent), &{
            let inner = self.inner.lock().expect("mutex");
            inner.plans.get(plan_id).unwrap().plan.adapter_id.clone()
        })?;
        {
            let mut inner = self.inner.lock().expect("mutex");
            inner.tries += 1;
            inner.depth += 1;
            if let Some(rec) = inner.plans.get_mut(plan_id) {
                rec.plan.state = PlanState::Applied;
            }
        }
        let remaining = self.start_analysis(&imported.artifact_id).ok();
        Ok(TransformApplyResult {
            plan_id: plan_id.to_string(),
            step_id: Uuid::new_v4().to_string(),
            output: imported,
            validation: preview.validation,
            result: preview.result,
            remaining_findings: remaining.map(|a| a.findings).unwrap_or_default(),
        })
    }

    fn transform_job(
        &self,
        plan_id: &str,
        _apply: bool,
    ) -> Result<(Vec<u8>, TransformPreview), IpcError> {
        if !diagnose_isolation().available {
            return Err(IpcError::new(
                ErrorCode::SandboxUnavailable,
                "error.sandbox_unavailable",
            ));
        }
        let (bytes, finding, adapter, artifact_id, workspace, input_hash) = {
            let inner = self.inner.lock().expect("mutex");
            if inner.tries >= MAX_TRANSFORM_TRIES {
                return Err(IpcError::new(ErrorCode::LimitReached, "error.limit_reached"));
            }
            let rec = inner
                .plans
                .get(plan_id)
                .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.not_found"))?;
            let analysis = inner
                .analysis
                .as_ref()
                .ok_or_else(|| IpcError::new(ErrorCode::NotFound, "error.no_analysis"))?;
            let finding = analysis
                .findings
                .iter()
                .find(|f| {
                    f.artifact_id == rec.plan.input_artifact_id
                        && f.technique_id == rec.plan.technique_id
                        && f.range == rec.plan.range
                })
                .cloned()
                .ok_or_else(|| IpcError::new(ErrorCode::PlanStale, "error.plan_stale"))?;
            let art = inner.artifacts.get(&rec.plan.input_artifact_id).unwrap();
            if art.imported.sha256 != rec.plan.input_hash {
                return Err(IpcError::new(ErrorCode::PlanStale, "error.plan_stale"));
            }
            let bytes = std::fs::read(blob_dir(&inner).join(&art.imported.sha256))
                .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
            (
                bytes,
                finding,
                rec.plan.adapter_id.clone(),
                rec.plan.input_artifact_id.clone(),
                inner.workspace.clone(),
                rec.plan.input_hash.clone(),
            )
        };
        let _ = input_hash;
        let job_id = Uuid::new_v4().to_string();
        let req = WorkerRequest::transform(&artifact_id, &job_id, &adapter, finding);
        let sandbox = workspace.join("staging").join(&job_id);
        let resp = run_isolated(
            &req,
            &bytes,
            &sandbox,
            None,
            Duration::from_secs(unveil_contracts::TRANSFORM_WALL_SECS),
        )?;
        if !resp.ok {
            return Err(resp.error.unwrap_or_else(|| {
                IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed")
            }));
        }
        let out_bytes = decode_b64(resp.output_b64.as_deref().unwrap_or(""))
            .ok_or_else(|| IpcError::new(ErrorCode::SchemaMismatch, "error.schema_mismatch"))?;
        let mut preview = resp.preview.ok_or_else(|| {
            IpcError::new(ErrorCode::WorkerCrashed, "error.worker_crashed")
        })?;
        preview.plan_id = plan_id.to_string();
        preview.candidate_sha256 = resp.output_sha256.unwrap_or(preview.candidate_sha256);
        preview.byte_length = out_bytes.len() as u64;
        let mut inner = self.inner.lock().expect("mutex");
        if let Some(rec) = inner.plans.get_mut(plan_id) {
            rec.preview = Some(preview.clone());
            rec.preview_bytes = Some(out_bytes.clone());
            rec.plan.state = PlanState::AwaitingApproval;
        }
        Ok((out_bytes, preview))
    }

    pub fn save_session(&self, name: &str) -> Result<String, IpcError> {
        let inner = self.inner.lock().expect("mutex");
        inner
            .store
            .mark_saved(&inner.session_id, name)
            .map_err(|_| IpcError::new(ErrorCode::StorageFull, "error.storage_full"))?;
        Ok(inner.session_id.clone())
    }

    pub fn delete_session(&self) -> Result<(), IpcError> {
        let inner = self.inner.lock().expect("mutex");
        let session = inner.session_id.clone();
        let blobs = blob_dir(&inner);
        inner
            .store
            .delete_session(&session)
            .map_err(|_| IpcError::new(ErrorCode::PermissionDenied, "error.permission_denied"))?;
        let _ = std::fs::remove_dir_all(blobs);
        Ok(())
    }

    pub fn export_report(&self, include_bodies: bool) -> Result<ReportDocument, IpcError> {
        let inner = self.inner.lock().expect("mutex");
        Ok(report::build_report(
            &inner.session_id,
            inner.analysis.as_ref(),
            &app_info(),
            include_bodies,
        ))
    }

    pub fn job_snapshot(&self, job_id: &str) -> JobSnapshot {
        JobSnapshot {
            job_id: job_id.into(),
            session_id: self.inner.lock().expect("mutex").session_id.clone(),
            kind: unveil_contracts::JobKind::Analyze,
            status: unveil_contracts::JobStatus::Completed,
            stage: "detect".into(),
            artifact_id: self.current_artifact().map(|a| a.artifact_id),
            error: None,
        }
    }
}

fn blob_dir(inner: &Inner) -> PathBuf {
    inner
        .workspace
        .join("sessions")
        .join(&inner.session_id)
        .join("blobs")
}

fn hash_plan(input_hash: &str, range: &ByteRange, adapter: &str, params: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input_hash.as_bytes());
    hasher.update(range.start.to_le_bytes());
    hasher.update(range.end.to_le_bytes());
    hasher.update(adapter.as_bytes());
    hasher.update(params.to_string().as_bytes());
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(256)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn rfc3339_plus_mins(mins: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + mins * 60;
    format!("{now}")
}

fn expired(stamp: &str) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    stamp.parse::<u64>().unwrap_or(0) < now
}

fn decode_b64(text: &str) -> Option<Vec<u8>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    let mut s = text.to_string();
    while s.len() % 4 != 0 {
        s.push('=');
    }
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut map = [0xffu8; 256];
    for (i, b) in table.iter().enumerate() {
        map[*b as usize] = i as u8;
    }
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            return None;
        }
        let n = ((map[chunk[0] as usize] as u32) << 18)
            | ((map[chunk[1] as usize] as u32) << 12)
            | ((map[chunk[2] as usize] as u32) << 6)
            | (map[chunk[3] as usize] as u32);
        out.push(((n >> 16) & 0xff) as u8);
        if chunk[2] != b'=' {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if chunk[3] != b'=' {
            out.push((n & 0xff) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lesson_snapshot_does_not_need_isolation() {
        let dir = tempfile::tempdir().expect("tempdir");
        let coordinator = Coordinator::new(dir.path().to_path_buf());
        let artifact = coordinator.import_lesson().expect("lesson");
        assert_eq!(artifact.sha256.len(), 64);
        assert_eq!(artifact.display_name, "lesson-base64.js");
    }

    #[test]
    fn overview_starts_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let overview = Coordinator::new(dir.path().to_path_buf()).overview();
        assert_eq!(overview.saved_sessions, 0);
    }

    #[test]
    fn wiki_pack_has_required_articles() {
        let pack = WikiPack::load(default_knowledge_dir());
        assert!(
            pack.articles.len() >= 12,
            "need 12+ wiki articles, got {}",
            pack.articles.len()
        );
        assert!(pack.get("wiki.enc.base64.ja").is_some());
    }
}
