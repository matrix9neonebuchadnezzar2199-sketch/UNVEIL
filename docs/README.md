# UNVEIL 設計図 — ドキュメント入口

- 設計版: **0.2.0-unified** / 作成日: 2026-09-06（統合拡張: 2026-09-08）
- 状態: **難読化 MVP（PHASE 00–06）は実装済み。** 統合骨格（U00–U07）は 2026-09-08 に TEST 合格。**深度 Gate は [16](16-depth-phase-plan.md)。実装手順は [17](17-depth-implementation-spec.md)（U11–U16・別モデル）。**
- 実行時正本: リポジトリ root の [README.md](../README.md)、検証入口 `TEST.py`
- 目的（拡張後）: 隔離 Windows 上の **統合研究ワークベンチ** — マルウェア静的トリアージ、難読化検知・解除支援、USB 読取専用検査、WIKI — をサイドバーモジュールで提供する。
- UI: サイドバー＋メイン。モック（サイバーテーマ）:
  - [mockup.html](mockup.html) — Dashboard
  - [mockup_malware.html](mockup_malware.html) — マルウェア解析
  - [mockup_analyze.html](mockup_analyze.html) — 難読化判定
  - [mockup_usb.html](mockup_usb.html) — USBチェック

## 文書群の分担

| 範囲 | 番号 | 内容 |
|---|---|---|
| **難読化判定モジュール** | [00–09](00-product-requirements.md) | 要件、PHASE 00–07、パイプライン、IPC、WIKI、UI、セキュリティ、品質、運用 |
| **統合 UPGRADE** | [10–17](10-unified-workbench.md) | ワークベンチ定義、モジュール構成、U00–U07 骨格、MalCheck/USB/Magika、**U10–U16 深度**、[17 実装仕様](17-depth-implementation-spec.md) |

00–09 の本文は難読化の正本として **原則変更しない**。統合で触れるのは 09（OQ/ADR 追記）、04（ModuleJob 拡張）、06（サイドバー IA）など、変更管理に従う差分のみ。

## 読む順番

### 初めて読む

| 文書 | 内容 | 主な読者 |
|---|---|---|
| [10 統合ワークベンチ](10-unified-workbench.md) | 製品再定義、混同表、FR、サイドバー IA | 全員 |
| [12 UPGRADE PHASE 計画](12-upgrade-phase-plan.md) | U00–U07 骨格、Gate、TEST.py U01–U08 | PM・全リード |
| [16 DEPTH PHASE 計画](16-depth-phase-plan.md) | U10–U16 Gate | PM・全リード |
| [17 DEPTH 実装仕様](17-depth-implementation-spec.md) | ファイル・API・TEST アサーション。計画を発明しない | 実装者 |

### 難読化モジュール（00–09）

| 文書 | 内容 | 主な読者 |
|---|---|---|
| [00 要件・対象範囲](00-product-requirements.md) | 目的、利用者、要求ID、対応表、非目標 | 全員 |
| [01 PHASE管理計画](01-phase-plan.md) | PHASE 00〜07（難読化 MVP） | PM・全リード |
| [02 システム構成](02-architecture.md) | 技術選定、信頼境界、プロセス、保存 | アーキテクト |
| [03 検知・特定・解除](03-analysis-pipeline.md) | 四軸、検知器、変換、限界 | 解析エンジン |
| [04 データ・IPC契約](04-data-contracts.md) | エンティティ、状態遷移、API | フロント・バック |
| [05 WIKI・歴史・分類](05-wiki-knowledge.md) | 記事、年表、教材 | 教材・解析 |
| [06 UI・UX](06-ui-ux.md) | 画面、SF調、操作・状態 | デザイナー・フロント |
| [07 セキュリティ](07-security.md) | 隔離、脅威モデル | セキュリティ |
| [08 品質・検証](08-quality-validation.md) | 試験計画、受入 | QA |
| [09 意思決定・運用](09-decisions-operations.md) | OQ、ADR、リスク、変更管理 | PM・運用 |

### 統合 UPGRADE（10–17）

| 文書 | 内容 | 主な読者 |
|---|---|---|
| [11 モジュールアーキテクチャ](11-module-architecture.md) | sidecar、manifest、能力 diagnose、IPC 拡張 | アーキテクト |
| [13 マルウェア解析](13-malware-module.md) | MalCheck `mau/`、schema 2.1 | 解析・Docker |
| [14 USBチェック](14-usb-module.md) | USB-GuardDuty 融合、読取専用 | 解析・Python |
| [15 Magika 吸収](15-magika-absorption.md) | ルーティング、禁止表示 | 解析 |
| [modules.manifest.example.json](modules.manifest.example.json) | サイドバーレジストリ例 | 実装 |
| [16 DEPTH PHASE 計画](16-depth-phase-plan.md) | U10–U16 深度 Gate | PM・実装 |
| [17 DEPTH 実装仕様](17-depth-implementation-spec.md) | U11–U16 の手順・DoD・触るなリスト | 実装者 |

設計アルゴリズム: Obsidian `30_Knowhow/gated-mvp-design-algorithms.md`、Cursor `21-intel-cyber-design.mdc`。

## 設計の中心原則

1. **入力を実行しない。** 読み込み・デコード・整形を理由に対象プログラムを起動しない。
2. **根拠を提示する。** ラベルだけでなく位置、観測値、反証、未解析範囲を示す。
3. **未検出は不在証明ではない。** 対応範囲と解析カバレッジを必ず併記する。
4. **原本を保持する。** すべての出力は派生物とし、変換履歴とハッシュを残す。
5. **モジュールを直交させる。** 1 本の危険度％に畳まない（[10](10-unified-workbench.md) ALG-01）。
6. **能力が無いときは fail-closed。** 静かな劣化禁止（ALG-02）。

## PHASE 一覧

### 難読化 MVP（01-phase-plan.md）

| PHASE | 名称 | 実装状態（2026-09-08） |
|---|---|---|
| 00–06 | 要件〜品質 MVP | **実装済み**（`TEST.py` / `unveil-ctl smoke`） |
| 07 | 高度解析・拡張 | 未着手・MVP 外 |

### 統合 UPGRADE（12-upgrade-phase-plan.md）

| PHASE | 名称 | 状態 |
|---|---|---|
| U00 | 用語・manifest 契約 | 骨格完了（文書+09追記） |
| U01 | サイドバーレジストリ | 骨格完了 |
| U02 | Broker 拡張 | 骨格完了 |
| U03 | Magika Probe | 骨格完了（モデル未同梱 → `unavailable` が正当） |
| U04 | USB sidecar | 骨格完了（フォルダスキャン。列挙 UI は U12） |
| U05 | MalCheck sidecar | 骨格完了（schema 2.1 + 表層 stub。実スキャナは U13） |
| U06 | 横断導線 | 骨格完了（ctl handoff。画面は U15） |
| U07 | 統合 TEST.py | 骨格完了（U01–U08 プローブ） |
| U10–U16 | 深度 | Gate: [16](16-depth-phase-plan.md)。実装: [17](17-depth-implementation-spec.md)（コード未着手） |

## 表記と仕様の優先順位

- **必須**: Gate 条件。**推奨**: 理由記録で変更可。**提案/暫定**: 承認前。
- 難読化の意味: 00, 03。統合製品: 10。工程: 01 + 12 + 16。実装手順: 17。契約: 04 + 11。安全: 07。合否: 08 + 12/16 Gate。
- 衝突時は [09](09-decisions-operations.md) の変更管理で関連文書を同 PR 更新。安全緩和はセキュリティ承認必須。
