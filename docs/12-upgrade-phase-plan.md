# 12 — UPGRADE PHASE 管理計画（U00–U07）

- 設計版: 1.0.0 / 作成日: 2026-09-08
- 状態: U00–U07 **骨格実装済み**（2026-09-08 `python TEST.py` の U00–U08 プローブ合格）。これは機能完成ではない。深度は [16-depth-phase-plan.md](16-depth-phase-plan.md)（U10–U16）。
- 関係: 既存 [01-phase-plan.md](01-phase-plan.md) の PHASE 00–06（難読化 MVP）は **完了済み実装**として維持。本書の U 系列は **統合拡張の骨格**専用。番号衝突を避けるため **U** プレフィックスを用いる。

## 1. 管理方式

各 U-PHASE は `未着手 → 設計中 → 設計レビュー → 実装中 → 検証中 → 完了` で管理する。Gate 記録: `phase / build_commit / evidence_links / metrics / outstanding_items / approvers / date`。

**完了の定義:**

- U00–U06: 成果物 + TEST.py プローブ合格 + レビュー署名
- U07: 全統合プローブ合格。**出荷承認は U07 とは別**（既存 G06 + 統合回帰）

**Gate に載せないもの:** 未測定の precision / recall ％（[08](08-quality-validation.md) 継承）

```text
U00 用語・manifest 契約
  → U01 シェル（サイドバーレジストリ）
  → U02 Broker 拡張（ModuleJob・diagnose）
  → U03 Magika Probe
  → U04 USB sidecar
  → U05 MalCheck sidecar
  → U06 横断導線
  → U07 統合 TEST.py・回帰
```

U03 を U04/U05 より前に置く理由: MalCheck / USB は拡張子・libmagic 依存が強く、**ルーティング契約を先に固定**する。

---

## U00 — 用語・脅威・manifest 契約

**狙い:** 統合の混同を防ぎ、manifest と sidecar 方針を文書で固定する。

- 入口: 本 UPGRADE 設計承認依頼
- 担当: PM（責任）、セキュリティ・解析リード（レビュー）
- U00-01: [10-unified-workbench.md](10-unified-workbench.md) の混同表・FR をレビュー
- U00-02: [11-module-architecture.md](11-module-architecture.md) の信頼境界・能力 C をレビュー
- U00-03: OQ-U01–U03 を [09](09-decisions-operations.md) に追記
- U00-04: ADR-U001–U004 を 09 に追記
- 成果物: 承認済み 10–15 設計書、manifest 例、Gate 記録テンプレ
- **Gate G-U00:** FR-U01–U08 が試験 ID に割当済み。sidecar 方式がセキュリティ承認。マスターが統合方針を承認
- 失敗時: スコープを 1 モジュールに縮小（例: 難読化 + Magika のみ）

---

## U01 — シェル（サイドバーレジストリ）

**狙い:** Dashboard + 3 モジュールのサイドバーを manifest 駆動にする。難読化は既存 Analyze を `/deobfuscation` に移す。

- 入口: G-U00
- 担当: UI / アプリ担当
- U01-01: `modules.manifest` 読込、サイドバー描画、`status=planned` は disabled
- U01-02: ルーティング: `/dashboard`, `/malware`, `/deobfuscation`, `/usb`
- U01-03: モック HTML（[mockup.html](mockup.html)）をサイドバー 4+1 項目に更新（**既存サイバーテーマ維持**）
- U01-04: ヘッダにアクティブモジュール名・隔離状態を表示
- 成果物: manifest 駆動 UI、更新モック、E2E AT-U01/02
- **Gate G-U01:** 3 モジュール + Dashboard が表示。planned は押せない。既存難読化 smoke が通る
- 失敗時: サイドバーを固定 4 項目に戻し manifest は読むだけ（書込なし）

---

## U02 — 共通 Broker 拡張

**狙い:** ModuleJob、能力 diagnose 集約、sidecar 監督の骨格。

- 入口: G-U01
- 担当: Rust Coordinator 担当、セキュリティ
- U02-01: `ModuleJob` エンティティ（04 拡張）を SQLite に追加
- U02-02: `diagnose_module(module_id)` — 能力 AND 評価
- U02-03: Sidecar Supervisor: 起動・タイムアウト・JSON schema 検証
- U02-04: 既存 Import / Artifact をモジュール非依存に維持
- 成果物: Coordinator API、契約 v1.1 草案、単体試験
- **Gate G-U02:** U02 isolation プローブ合格。sidecar 未配置時は malware/usb が fail-closed
- 失敗時: sidecar なしで難読化 + Dashboard のみ出荷可能状態を維持

---

## U03 — Magika Probe（共通 content_type）

**狙い:** 全モジュールが参照するコンテンツタイプ推定を Broker に置く。

- 入口: G-U02
- 担当: 解析担当 + アプリ担当
- U03-01: 同梱 Magika モデル（OQ-U02: オフライン同梱、自動 DL 禁止）
- U03-02: `ContentTypeProbe` 生成（拡張子 / magic / Magika 三面）
- U03-03: `high-confidence` でルーティング表（[15](15-magika-absorption.md)）
- U03-04: `C_magika` 不合格時は unavailable（libmagic へ黙って fallback しない）
- 成果物: Probe worker、ルーティング表、fixture 試験
- **Gate G-U03:** AT-U04/05 合格。`.txt` 拡張子 PE 合成で mismatch 表示。JS を Ghidra 経路に送らない
- 失敗時: Magika 無効化し、拡張子 + magic のみ（UI に degraded 明示）

---

## U04 — USBチェック sidecar

**狙い:** USB-GuardDuty エンジンを UNVEIL USB モジュールとして接続。

- 入口: G-U03
- 担当: Python sidecar + UI 担当
- U04-01: `usb-sidecar` ラッパー（`backend/pipeline.py` 再利用）
- U04-02: UI: ドライブ一覧、スキャン進捗、結果ツリー（UGD 画面相当）
- U04-03: 読み取り専用検証（ST-U01）、`--online` 既定 OFF
- U04-04: YARA ルールは `%APPDATA%` 配下（UGD 互換）または UNVEIL workspace
- U04-05: worker pool / config 未配線の UGD 既知ギャップを解消（workers=4 等）
- 成果物: USB モジュール MVP、UGD `check_all` 相当の統合 smoke
- **Gate G-U04:** AT-U07、U07 usb readonly 合格。対象 USB へ書込なし
- 失敗時: USB モジュール `status=disabled`、他モジュールは維持

---

## U05 — マルウェア解析 sidecar

**狙い:** MalCheck `mau/` パイプラインを UNVEIL に接続。schema 2.1 を正本とする。

- 入口: G-U03（U04 と並行可。G-U04 非依存）
- 担当: Python + Docker 担当
- U05-01: `malcheck-sidecar`（`python -m mau.main` ラッパー）
- U05-02: Docker Ghidra `network_mode: none` を Coordinator が監督
- U05-03: UI: 表層 / 静的 / レポートタブ（MalCheck HTML 契約を WebView 表示または React 再描画）
- U05-04: dynamic 既定 skip、LLM 既定 OFF、hub 既定 OFF（明示 opt-in）
- U05-05: OQ-U03: Ghidra イメージ参照方針を確定
- 成果物: マルウェアモジュール MVP、EICAR + 合成 PE fixture
- **Gate G-U05:** AT-U06、U06 合格。schema 2.1、`phase2_dynamic.skipped`
- 失敗時: 表層のみ（Ghidra 無し）に縮退。Docker 必須を UI に明示

---

## U06 — 横断導線

**狙い:** USB → マルウェア、マルウェア疑似 C → 難読化など **Artifact handoff**。

- 入口: G-U04 **かつ** G-U05
- 担当: UI + Coordinator
- U06-01: 結果画面から「マルウェア解析へ」「難読化判定へ」— Artifact ID のみ渡す
- U06-02: handoff 時に Magika Probe を再実行するか（同一 sha256 ならキャッシュ）を ADR 化
- U06-03: Dashboard にモジュール横断 Job 一覧
- U06-04: 統合レポート export（モジュール別 JSON を 1 zip にまとめる optional）
- 成果物: Handoff API、E2E 導線、AT-U08
- **Gate G-U06:** USB スキャン PE → マルウェア deep で同一 SHA-256。難読化回帰無し
- 失敗時: handoff ボタンを無効化し単モジュール運用

---

## U07 — 品質・統合 TEST.py

**狙い:** 統合プローブを TEST.py に載せ、既存 PHASE 06 回帰を壊さないことを証明する。

- 入口: G-U06
- 担当: QA / 全リード
- U07-01: TEST.py に U01–U08 プローブ追加（下表）
- U07-02: 既存プローブ（隔離 smoke、wiki、cargo test）の回帰
- U07-03: sidecar SBOM / 依存版固定
- U07-04: [08](08-quality-validation.md) に AT-U* / ST-U* を追記（試験定義のみ。合格％は空）
- U07-05: 統合開発日記・カタログ更新（UNVEIL エントリ説明更新）
- 成果物: TEST.py 拡張、試験証跡、Gate 記録
- **Gate G-U07:** 下表全プローブ合格。既存 `unveil-ctl smoke` 合格。Critical/High 既知問題ゼロ（統合範囲）
- 失敗時: 不合格モジュールを manifest で `disabled` にし再 Gate

### TEST.py 統合プローブ一覧（ALG-14）

| プローブ | 内容 | PHASE |
|---|---|---|
| U01 manifest renders | 3 モジュール + planned disabled | U01 |
| U02 isolation still fail-closed | 隔離不合格で解析開始不可 | U02 |
| U03 magika model loads | 失敗時 unavailable（SKIP 不可） | U03 |
| U04 mismatch fixture | `.txt` 中身 PE、三面不一致 | U03 |
| U05 routing | JS を Ghidra に送らない | U03, U05 |
| U06 malcheck schema 2.1 | EICAR/合成 PE、dynamic skipped | U05 |
| U07 usb readonly | スキャン中対象へ書込なし | U04 |
| U08 handoff | USB artifact → マルウェア同一 SHA-256 | U06 |

---

## 2. 規模の目安（暫定・実測で更新）

| PHASE | 人週目安 | 依存 |
|---|---|---|
| U00 | 0.5–1 | — |
| U01 | 1–2 | U00 |
| U02 | 2–3 | U01 |
| U03 | 1–2 | U02 |
| U04 | 3–4 | U03 |
| U05 | 4–6 | U03 |
| U06 | 1–2 | U04, U05 |
| U07 | 2–3 | U06 |

U04/U05 並行時は 3–4 人週短縮可能。Docker / Ghidra 環境構築は U05 に含む。

## 3. 変更管理

- U-PHASE 完了後のモジュール追加は、manifest + 個別 Gate + 脅威レビュー + TEST プローブを必須とする
- 10–15 設計書と 00–09 の衝突は **09 の変更管理**で同 PR 更新。安全緩和はセキュリティ承認必須
- MalCheck / USB-GuardDuty 上流更新を取り込む場合は schema 差分評価を U05/U04 Gate で実施
- 骨格の次の作業は [16-depth-phase-plan.md](16-depth-phase-plan.md)。実装手順は [17-depth-implementation-spec.md](17-depth-implementation-spec.md)。U00–U07 を「やり直す」のではなく残差だけを U10 以降に載せる
