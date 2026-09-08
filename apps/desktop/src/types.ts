export type ErrorCode =
  | "SANDBOX_UNAVAILABLE"
  | "INPUT_NOT_REGULAR"
  | "INPUT_CHANGED"
  | "INPUT_TOO_LARGE"
  | "CANCELLED"
  | "PERMISSION_DENIED"
  | "LIMIT_REACHED"
  | "NOT_FOUND"
  | "ADAPTER_UNSUPPORTED"
  | "PLAN_STALE"
  | "APPROVAL_EXPIRED"
  | "VALIDATION_FAILED"
  | "PARSE_FAILED"
  | "WORKER_CRASHED"
  | "STORAGE_FULL"
  | "SCHEMA_MISMATCH"
  | "MODULE_UNAVAILABLE";

export type AppInfo = {
  name: string;
  version: string;
  schema_version: string;
  phase: string;
  knowledge_pack: string;
  rule_version: string;
};

export type IsolationCheck = {
  id: string;
  passed: boolean;
  detail: string;
};

export type IsolationDiagnosis = {
  available: boolean;
  platform: string;
  error_code: ErrorCode | null;
  checks: IsolationCheck[];
};

export type SelectionPreview = {
  token: string;
  display_name: string;
  byte_length: number;
  kind_hint: string;
};

export type ImportedArtifact = {
  artifact_id: string;
  session_id: string;
  display_name: string;
  byte_length: number;
  sha256: string;
  kind_hint: string;
  save_mode: string;
};

export type NamedCount = {
  id: string;
  count: number;
};

export type SessionRow = {
  session_id: string;
  name: string;
  findings: number;
  status: string;
};

export type DashboardOverview = {
  saved_sessions: number;
  jobs_total: number;
  findings_total: number;
  isolation_passed: number;
  categories: NamedCount[];
  techniques: NamedCount[];
  job_outcomes: NamedCount[];
  module_jobs?: NamedCount[];
  sessions: SessionRow[];
};

export type DetectorToggle = {
  id: string;
  enabled: boolean;
  mvp: boolean;
  summary: string;
};

export type Finding = {
  id: string;
  artifact_id: string;
  technique_id: string;
  category: string;
  range: { start: number; end: number };
  identification: string;
  score: { value: number; kind: string; rule_version: string };
  evidence: { observation: string; polarity: string; excerpt: string }[];
  transformability: string;
  limitations: string[];
  article_id: string;
};

export type AnalysisResult = {
  job_id: string;
  artifact_id: string;
  sha256: string;
  coverage: { status: string; checked_rules: string[]; skipped: string[]; inspected_bytes: number; notes: string[] };
  assessment: string;
  findings: Finding[];
  limitations: string[];
  probe_kind: string;
  text_preview: string;
};

export type TransformPlan = {
  plan_id: string;
  plan_hash: string;
  adapter_id: string;
  technique_id: string;
  warnings: string[];
  lossy: boolean;
};

export type TransformPreview = {
  plan_id: string;
  candidate_sha256: string;
  candidate_preview: string;
  byte_length: number;
  result: string;
  validation: { verdict: string; checks: { kind: string; detail: string }[]; limitations: string[] };
};

export type Approval = {
  approval_id: string;
  plan_hash: string;
  expires_at: string;
};

export type WikiHit = {
  article_id: string;
  title: string;
  excerpt: string;
  engine_support: string;
};

export type WikiArticle = {
  article_id: string;
  title: string;
  engine_support: string;
  body_markdown: string;
  citations: string[];
};

export type ReportDocument = {
  report_id: string;
  html: string;
  json: string;
  included_original: boolean;
  included_bodies: boolean;
  reproducibility_gaps: string[];
};

export type IpcError = {
  code: ErrorCode;
  message_key: string;
};

export type PageId = "dashboard" | "deobfuscation" | "malware" | "usb";

export type ModuleSpecView = {
  id: PageId | string;
  label_ja: string;
  route: string;
  status: string;
  disabled: boolean;
};

export type CapabilityCheck = {
  id: string;
  available: boolean;
  detail: string;
};

export type ModuleDiagnosis = {
  module_id: string;
  available: boolean;
  status: string;
  checks: CapabilityCheck[];
};

export type MagikaOutput = {
  status: string;
  dl_label: string | null;
  output_label: string | null;
  score: number | null;
  prediction_mode: string;
  mime_type: string | null;
  is_text: boolean | null;
};

export type ContentTypeProbe = {
  probe_id: string;
  artifact_id: string;
  declared_extension: string;
  magic_hint: string;
  magika: MagikaOutput;
  mismatch_flags: string[];
  ghidra_eligible: boolean;
};

export type ModuleJob = {
  job_id: string;
  module_id: string;
  input_artifact_id: string | null;
  sha256: string | null;
  status: string;
  payload: unknown;
};

export type UsbDrive = {
  token: string;
  token_hint: string;
  letter: string;
  bus?: string;
  vid?: string | null;
  pid?: string | null;
  composite_suspect?: boolean;
  capacity_bytes?: number;
};

export type UsbInventoryRow = {
  name: string;
  sha256?: string | null;
  size?: number | null;
};

export type UsbScanPayload = {
  inventory?: UsbInventoryRow[];
  overall?: string;
  findings_count?: number;
  notice?: string;
  workers?: number;
};
