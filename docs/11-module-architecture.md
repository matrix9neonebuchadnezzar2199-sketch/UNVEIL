# 11 — モジュールアーキテクチャ

- 設計版: 1.0.0 / 作成日: 2026-09-08
- 状態: UPGRADE 設計案
- 正本: [02-architecture.md](02-architecture.md) の信頼境界を拡張する。衝突時は安全制約（[07](07-security.md)）が優先。

## 1. 設計判断（ADR-U001 提案）

MalCheck（`mau/` + Docker）と USB-GuardDuty（Python analyzers）は **Rust へ直移植しない**。Coordinator が監督する **隔離 sidecar** として再利用する。

| 層 | 技術 | 再利用 |
|---|---|---|
| シェル | Tauri 2 + React + Rust Coordinator | 既存 UNVEIL |
| 難読化エンジン | `unveil-analysis-worker` + `unveil_engine` | 既存 |
| Magika Probe | 同梱 ONNX（Python または Rust バインディング） | [google/magika](https://github.com/google/magika) |
| マルウェア sidecar | Python `mau/` + Docker Ghidra | MalCheck 契約 |
| USB sidecar | Python `backend/` analyzers | USB-GuardDuty 契約 |

**捨てるもの:** MalCheck FastAPI/Jinja Web UI、USB-GuardDuty pywebview/FastAPI シェル。  
**残すもの:** schema 2.1 JSON、UGD findings 契約、Docker 境界、テスト fixture 思想。

## 2. 信頼境界（ALG-03）

```text
利用者
  │ 操作（モジュール選択・取込・承認）
  ▼
UI WebView（非信頼。生パス・任意コマンドなし）
  │ 型付き Tauri IPC のみ
  ▼
Coordinator / Broker（特権）
  ├─ Import Broker ───── スナップショット + SHA-256
  ├─ Module Registry ─── modules.manifest 読込・diagnose 集約
  ├─ Magika Probe ────── content_type（全モジュール共通）
  ├─ Job Supervisor ──── モジュール別 Job・キャンセル・上限
  ├─ Sidecar Supervisor ─ malcheck-sidecar / usb-sidecar 起動・JSON 検証
  ├─ Artifact Store ──── 不変 blob + モジュール間引き渡し
  ├─ SQLite ──────────── Session / Job / Finding メタデータ
  └─ Report Writer ───── エスケープ済み export
           │
           ├─ unveil-analysis-worker（難読化、既存）
           ├─ magika-probe worker（読取のみ、同梱モデル）
           ├─ malcheck-sidecar（Docker socket 監督、network:none Ghidra）
           └─ usb-sidecar（対象ドライブ read-only、ホスト temp のみ write）
```

sidecar から UI / DB / ユーザーホームへ直接接続させない。stdout JSON は Broker が schema・長さ・ハッシュを再検証する。

## 3. 能力 C と diagnose（ALG-02）

| 能力 ID | 意味 | diagnose 合格条件 | 不合格時 |
|---|---|---|---|
| C_isolation | AppContainer + Job | 既存 `diagnose_isolation` 全 check passed | 全解析モジュール開始不可 |
| C_docker | Docker CLI + Ghidra イメージ | `docker info` + イメージ存在 | Ghidra フェーズ unsupported。表層のみ可 |
| C_usb | リムーバブル列挙 | WinAPI で removable 検出 | USB モジュール開始不可 |
| C_magika | 同梱モデル読込 | 固定 fixture で label 返却 | `content_type.status=unavailable`（libmagic へ黙って fallback しない） |
| C_yara | YARA コンパイル | EICAR ルール smoke | YARA 軸のみ unsupported |
| C_malcheck_surface | surface コンテナ or ローカル fallback | EICAR スキャン | 表層 unsupported（fallback 時は UI に明示） |
| C_sidecar_malcheck | `python -m mau.main --probe` | ヘルス JSON | マルウェアモジュール開始不可 |
| C_sidecar_usb | UGD worker import | `check_all` 相当 smoke | USB モジュール開始不可 |

モジュール開始前に **そのモジュールが要求する能力の AND** を評価する。1 つでも欠ければ fail-closed（WIKI / Dashboard は利用可）。

## 4. modules.manifest

サイドバー項目の **唯一の正本**（ハードコード禁止）。例: [modules.manifest.example.json](modules.manifest.example.json)。

### 4.1 スキーマ（schema_version 1.0）

| フィールド | 必須 | 説明 |
|---|---|---|
| `schema_version` | yes | `"1.0"` |
| `modules[]` | yes | サイドバー順 |
| `modules[].id` | yes | 安定 ID（`malware`, `deobfuscation`, `usb`） |
| `modules[].label_ja` | yes | 表示名 |
| `modules[].label_en` | no | 将来 i18n |
| `modules[].route` | yes | UI ルート（`/` 除く） |
| `modules[].capabilities` | yes | 上表の能力 ID 配列 |
| `modules[].status` | yes | `mvp` / `planned` / `disabled` |
| `modules[].worker` | no | `native` / `sidecar:malcheck` / `sidecar:usb` |
| `modules[].test_probes` | no | TEST.py プローブ名配列 |
| `modules[].wiki_article_ids` | no | 失敗導線用 WIKI リンク |

`status=planned` の項目はサイドバーに表示するが **disabled**（クリック不可 + 「準備中」）。

### 4.2 今後モジュールを追加する手順

1. `modules.manifest` に 1 要素追加（id / label / capabilities / route / test_probes）
2. [12-upgrade-phase-plan.md](12-upgrade-phase-plan.md) に **個別 Gate**（U07 拡張と同型）を追記
3. 画面 React view + Coordinator IPC + diagnose + sidecar または native worker
4. TEST.py にプローブ追加（ALG-14）
5. 混同表・脅威モデル・WIKI バッジを同 PR で更新
6. 任意コードを Coordinator が `eval` / `dlopen` しない（[07](07-security.md) TH-07）

## 5. IPC 拡張（04 契約への追加提案）

既存 [04-data-contracts.md](04-data-contracts.md) の Session / Artifact / Job を継承し、次を追加する。

### 5.1 ModuleJob

```json
{
  "job_id": "uuid",
  "module_id": "malware",
  "input_artifact_id": "uuid",
  "content_type_probe_id": "uuid",
  "status": "queued|running|completed|partial|failed|cancelled",
  "module_payload_ref": "blob_ref_to_module_json"
}
```

### 5.2 ContentTypeProbe（Magika 出力）

```json
{
  "probe_id": "uuid",
  "artifact_id": "uuid",
  "declared_extension": "txt",
  "magic_hint": "PE32 executable",
  "magika": {
    "status": "ok|unavailable|too_small|empty",
    "dl_label": "pebin",
    "output_label": "pebin",
    "score": 0.997,
    "prediction_mode": "high-confidence",
    "mime_type": "application/x-dosexec"
  },
  "mismatch_flags": ["extension_vs_magika"]
}
```

### 5.3 Handoff（モジュール間）

```json
{
  "source_module": "usb",
  "source_job_id": "uuid",
  "target_module": "malware",
  "artifact_id": "uuid",
  "sha256": "hex64",
  "reason": "user_requested_deep_analysis"
}
```

Broker は handoff 時に **新 Artifact を同一 bytes で参照**（または blob 共有 + 新 Job）。原本パスは引き渡さない。

## 6. Sidecar プロトコル（暫定）

- 起動: Coordinator が `--isolated` 相当の子プロセス（Job Object / AppContainer 内）
- 入力: 長さ付き JSON 1 フレーム + blob パス（Broker 管理下の staging のみ）
- 出力: 長さ付き JSON（MalCheck report 2.1 または UGD scan result）
- タイムアウト: Broker が wall clock を計数（ワーカー自己申告を信じない）
- 終了: ジョブ完了後プロセスツリー破棄

MalCheck 起動例（設計上）:

```text
malcheck-sidecar run --sample-blob <staging> --config <embedded.yaml> --no-llm --dynamic skip
```

USB 起動例:

```text
usb-sidecar scan --root <drive_letter>: --readonly --online false
```

## 7. 保存レイアウト（拡張）

```text
workspace/
  metadata.sqlite
  modules.manifest          # 同梱または user override（署名検証は G-U07）
  sessions/<id>/blobs/
  sessions/<id>/module_jobs/<module_id>/<job_id>.json
  sidecars/                 # 同梱 Python venv / スクリプト（読取専用）
  knowledge/                # 既存 WIKI
```

## 8. 関連 ADR（09 へ追記予定）

| ADR | 決定案 |
|---|---|
| ADR-U001 | MalCheck / UGD は Python sidecar。Rust 直移植しない |
| ADR-U002 | Magika は公式バインディング同梱。再学習しない |
| ADR-U003 | サイドバーは modules.manifest レジストリ |
| ADR-U004 | Magika は DIE/capa/YARA/Ghidra を置換しない（相補） |
