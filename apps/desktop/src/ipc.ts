import { invoke } from "@tauri-apps/api/core";
import type {
  AnalysisResult,
  AppInfo,
  Approval,
  DashboardOverview,
  DetectorToggle,
  ImportedArtifact,
  IsolationDiagnosis,
  ModuleDiagnosis,
  ModuleJob,
  ModuleSpecView,
  ContentTypeProbe,
  UsbDrive,
  ReportDocument,
  SelectionPreview,
  TransformPlan,
  TransformPreview,
  WikiArticle,
  WikiHit,
} from "./types";

export const loadAppInfo = () => invoke<AppInfo>("app_info_cmd");
export const diagnoseIsolation = () => invoke<IsolationDiagnosis>("isolation_diagnose");
export const loadOverview = () => invoke<DashboardOverview>("dashboard_overview");
export const loadDetectors = () => invoke<DetectorToggle[]>("detectors_cmd");
export const chooseFile = () => invoke<SelectionPreview>("file_choose");
export const importFile = (token: string) => invoke<ImportedArtifact>("file_import", { token });
export const importLesson = () => invoke<ImportedArtifact>("lesson_import");
export const startAnalysis = (artifactId: string) =>
  invoke<AnalysisResult>("analysis_start", { artifact_id: artifactId });
export const planTransform = (artifactId: string, findingId: string) =>
  invoke<TransformPlan>("transform_plan_cmd", { artifact_id: artifactId, finding_id: findingId });
export const previewTransform = (planId: string) =>
  invoke<TransformPreview>("transform_preview_cmd", { plan_id: planId });
export const approveTransform = (planId: string, planHash: string) =>
  invoke<Approval>("transform_approve_cmd", { plan_id: planId, plan_hash: planHash });
export const applyTransform = (planId: string, approvalId: string, planHash: string) =>
  invoke("transform_apply_cmd", {
    plan_id: planId,
    approval_id: approvalId,
    plan_hash: planHash,
  });
export const wikiSearch = (query: string) => invoke<WikiHit[]>("wiki_search_cmd", { query });
export const wikiGet = (articleId: string) => invoke<WikiArticle>("wiki_get_cmd", { article_id: articleId });
export const saveSession = (name: string) => invoke<string>("session_save_cmd", { name });
export const exportReport = (includeBodies: boolean) =>
  invoke<ReportDocument>("report_export_cmd", { include_bodies: includeBodies });
export const loadModules = () => invoke<{ modules: ModuleSpecView[] }>("modules_cmd");
export const diagnoseModule = (moduleId: string) =>
  invoke<ModuleDiagnosis>("diagnose_module_cmd", { module_id: moduleId });
export const lastProbe = () => invoke<ContentTypeProbe | null>("probe_last_cmd");
export const usbDrives = () => invoke<{ ok: boolean; drives: UsbDrive[] }>("usb_drives_cmd");
export const usbScan = (args?: { driveToken?: string; folderMode?: boolean; workers?: number }) =>
  invoke<ModuleJob>("usb_scan_cmd", {
    drive_token: args?.driveToken ?? null,
    folder_mode: args?.folderMode ?? false,
    workers: args?.workers ?? 4,
  });
export const usbCancel = () => invoke<void>("usb_cancel_cmd");
export const malcheckStart = (artifactId: string) =>
  invoke<ModuleJob>("malcheck_cmd", { artifact_id: artifactId });
export const handoffUsbFile = (jobId: string, name: string) =>
  invoke<ModuleJob>("handoff_usb_cmd", { job_id: jobId, name });
export const handoffDeobfuscation = (artifactId: string) =>
  invoke<ImportedArtifact>("handoff_deobfuscation_cmd", { artifact_id: artifactId });
export const loadCurrentArtifact = () =>
  invoke<ImportedArtifact | null>("current_artifact_cmd");
export const exportMarkdown = (moduleId: string, dest: "clipboard" | "workspace" | "picker") =>
  invoke<string>("markdown_export_cmd", { module_id: moduleId, dest });

export function formatIpcError(error: unknown): string {
  if (typeof error === "object" && error !== null && "message_key" in error) {
    const record = error as { message_key: string; code?: string };
    return `${record.code ?? "ERROR"}: ${record.message_key}`;
  }
  if (typeof error === "string") {
    return error;
  }
  return String(error);
}
