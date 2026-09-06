//! Tauri entry. Typed commands only. No path strings, no shell.

use std::path::PathBuf;

use tauri::{Emitter, Manager, WindowEvent};
use unveil_contracts::{
    AnalysisResult, AppInfo, Approval, ArtifactView, ComparisonPage, DashboardOverview,
    DetectorToggle, ErrorCode, GlossaryEntry, ImportedArtifact, IpcError, IsolationDiagnosis,
    ReportDocument, SelectionPreview, TimelineItem, TransformApplyResult, TransformPlan,
    TransformPreview, WikiArticle, WikiHit,
};
use unveil_coordinator::{
    app_info, default_knowledge_dir, diagnose_isolation, Coordinator,
};

#[tauri::command]
fn isolation_diagnose() -> IsolationDiagnosis {
    diagnose_isolation()
}

#[tauri::command]
fn app_info_cmd() -> AppInfo {
    app_info()
}

#[tauri::command]
fn dashboard_overview(coordinator: tauri::State<'_, Coordinator>) -> DashboardOverview {
    coordinator.overview()
}

#[tauri::command]
fn detectors_cmd() -> Vec<DetectorToggle> {
    DetectorToggle::mvp_set()
}

#[tauri::command]
fn file_choose(coordinator: tauri::State<'_, Coordinator>) -> Result<SelectionPreview, IpcError> {
    let path = rfd::FileDialog::new()
        .set_title("UNVEIL")
        .pick_file()
        .ok_or_else(|| IpcError::new(ErrorCode::Cancelled, "error.cancelled"))?;
    coordinator.register_path(&path)
}

#[tauri::command]
fn file_import(
    token: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<ImportedArtifact, IpcError> {
    coordinator.import_token(&token)
}

#[tauri::command]
fn lesson_import(coordinator: tauri::State<'_, Coordinator>) -> Result<ImportedArtifact, IpcError> {
    coordinator.import_lesson()
}

#[tauri::command]
fn analysis_start(
    artifact_id: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<AnalysisResult, IpcError> {
    coordinator.start_analysis(&artifact_id)
}

#[tauri::command]
fn analysis_last(coordinator: tauri::State<'_, Coordinator>) -> Option<AnalysisResult> {
    coordinator.last_analysis()
}

#[tauri::command]
fn artifact_view_cmd(
    artifact_id: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<ArtifactView, IpcError> {
    coordinator.artifact_view(&artifact_id)
}

#[tauri::command]
fn transform_plan_cmd(
    artifact_id: String,
    finding_id: String,
    adapter_id: Option<String>,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<TransformPlan, IpcError> {
    coordinator.plan_from_finding(&artifact_id, &finding_id, adapter_id.as_deref())
}

#[tauri::command]
fn transform_preview_cmd(
    plan_id: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<TransformPreview, IpcError> {
    coordinator.preview_plan(&plan_id)
}

#[tauri::command]
fn transform_approve_cmd(
    plan_id: String,
    plan_hash: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<Approval, IpcError> {
    coordinator.approve_plan(&plan_id, &plan_hash)
}

#[tauri::command]
fn transform_apply_cmd(
    plan_id: String,
    approval_id: String,
    plan_hash: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<TransformApplyResult, IpcError> {
    coordinator.apply_plan(&plan_id, &approval_id, &plan_hash)
}

#[tauri::command]
fn comparison_cmd(
    left_id: String,
    right_id: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<ComparisonPage, IpcError> {
    coordinator.comparison(&left_id, &right_id)
}

#[tauri::command]
fn wiki_search_cmd(query: String, coordinator: tauri::State<'_, Coordinator>) -> Vec<WikiHit> {
    coordinator.wiki_search(&query)
}

#[tauri::command]
fn wiki_get_cmd(
    article_id: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<WikiArticle, IpcError> {
    coordinator.wiki_get(&article_id)
}

#[tauri::command]
fn wiki_timeline_cmd(coordinator: tauri::State<'_, Coordinator>) -> Vec<TimelineItem> {
    coordinator.wiki_timeline()
}

#[tauri::command]
fn wiki_glossary_cmd(coordinator: tauri::State<'_, Coordinator>) -> Vec<GlossaryEntry> {
    coordinator.wiki_glossary()
}

#[tauri::command]
fn session_save_cmd(
    name: String,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<String, IpcError> {
    coordinator.save_session(&name)
}

#[tauri::command]
fn session_delete_cmd(coordinator: tauri::State<'_, Coordinator>) -> Result<(), IpcError> {
    coordinator.delete_session()
}

#[tauri::command]
fn report_export_cmd(
    include_bodies: bool,
    coordinator: tauri::State<'_, Coordinator>,
) -> Result<ReportDocument, IpcError> {
    coordinator.export_report(include_bodies)
}

fn register_drop(coordinator: &Coordinator, paths: &[PathBuf]) -> Option<SelectionPreview> {
    let path = paths.first()?;
    coordinator.register_path(path).ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let workspace = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("unveil"))
                .join("workspace");
            std::fs::create_dir_all(&workspace).ok();
            let knowledge = app
                .path()
                .resource_dir()
                .ok()
                .map(|p| p.join("knowledge").join("pack-1.0.0"))
                .filter(|p| p.is_dir())
                .unwrap_or_else(default_knowledge_dir);
            app.manage(Coordinator::with_knowledge(workspace, knowledge));

            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, .. }) = event {
                        let Some(coordinator) = handle.try_state::<Coordinator>() else {
                            return;
                        };
                        if let Some(preview) = register_drop(coordinator.inner(), paths) {
                            let _ = handle.emit("selection-ready", preview);
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            isolation_diagnose,
            app_info_cmd,
            dashboard_overview,
            detectors_cmd,
            file_choose,
            file_import,
            lesson_import,
            analysis_start,
            analysis_last,
            artifact_view_cmd,
            transform_plan_cmd,
            transform_preview_cmd,
            transform_approve_cmd,
            transform_apply_cmd,
            comparison_cmd,
            wiki_search_cmd,
            wiki_get_cmd,
            wiki_timeline_cmd,
            wiki_glossary_cmd,
            session_save_cmd,
            session_delete_cmd,
            report_export_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
