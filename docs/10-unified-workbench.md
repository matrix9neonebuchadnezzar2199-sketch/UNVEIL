# 10 — 統合ワークベンチ（製品定義）

- 設計版: 1.0.0 / 作成日: 2026-09-08
- 状態: **UPGRADE 骨格は実装済み（U00–U07）。** 深度 Gate は [16](16-depth-phase-plan.md)。実装手順は [17](17-depth-implementation-spec.md)。既存 [00–09](00-product-requirements.md) は **「難読化判定」モジュール**の正本。本書は UNVEIL を **統合研究ワークベンチ**へ拡張する製品定義である。

## 1. 製品の再定義

UNVEIL は、隔離された Windows 検証環境で動作する **ローカル・オフライン研究ワークベンチ**である。共通シェル（取込・隔離・セッション・レポート）の上に、サイドバーで選択する **解析モジュール**を載せる。

| モジュール（サイドバー） | 由来 | 役割 |
|---|---|---|
| Dashboard | 既存 | 全モジュール横断の概要（件数・状態・隔離診断） |
| マルウェア解析 | MalCheck | 静的トリアージ（表層 / Hub / Ghidra）。検体非実行 |
| 難読化判定 | 既存 UNVEIL PHASE 00–06 | 兆候・方式・解除支援・WIKI |
| USBチェック | USB-GuardDuty | リムーバブル媒体の読み取り専用静的検査 |

今後のモジュール追加は、サイドバーに 1 項目を増やす方式とする（[11-module-architecture.md](11-module-architecture.md) の `modules.manifest`）。

## 2. 混同表（ALG-00）

| 用語 | 本製品での意味 | 混同してはならないもの |
|---|---|---|
| ワークベンチ | UNVEIL シェル。取込・隔離・セッション・レポートの共通層 | 単一の万能解析エンジン |
| モジュール | サイドバー 1 項目 = 1 能力境界・1 脅威モデル | 任意コードプラグイン、App Store |
| マルウェア解析 | 静的トリアージ（YARA / capa / Ghidra 等）。動的実行なし | AV、侵害判定、安全証明 |
| 難読化判定 | 表現変換の兆候・方式・解除支援（[00](00-product-requirements.md) 準拠） | 悪意判定、マルウェアスコア |
| USBチェック | USB 上ファイルの読み取り専用静的検査 | BadUSB 検出、媒体消毒、ホスト保護の保証 |
| Magika | AI によるコンテンツタイプ推定（ルーティング用） | マルウェア判定、packer 製品特定（DIE 代替） |
| 融合 | シェル統合 + 既存エンジンを隔離ワーカー化 | 3 リポジトリを 1 プロセスに混在 |
| verdict（MalCheck） | ヒューリスティック集約ラベル | 検体の安全/危険の確定 |
| clean / suspicious / malicious（USB） | Finding 集約ラベル | 媒体が安全である証明 |

## 3. 直交軸と禁止表示（ALG-01）

モジュールをまたいでも **1 本の「危険度％」「安全度ゲージ」に畳まない**。各 Job は最低次の軸を返す（[03](03-analysis-pipeline.md) / [04](04-data-contracts.md) 継承）。

| 軸 | 意味 |
|---|---|
| coverage | 何をどこまで検査したか |
| assessment | 対象ドメインの兆候評価（範囲外は inconclusive） |
| identification | 方式・タイプ候補の証拠強度 |
| capability | 現版で何ができるか（supported / partial / unsupported） |
| job.status | 実行の成否（上記と独立） |

**禁止表示:**

- Magika の `score` を悪意確率・verdict に流用しない
- MalCheck `verdict.score` を UI で「脅威度％」と表示しない（「ヒューリスティック」と明示）
- USB の `malicious` を「USB は安全」と読ませない
- 複数モジュール結果を単一の緑/赤アイコンに畳まない

## 4. 利用者と主要シナリオ

- **研究者:** 許諾済み試料をモジュールごとに解析し、根拠付きレポートを再現する。
- **検証担当者:** USB 受領 → 表層トリアージ → 必要時に Ghidra / 難読化解析へ **スナップショット引き渡し**（原本パスを UI に出さない）。
- **学習者:** 難読化 WIKI と教材で技法を学び、解析画面へ戻る（既存導線維持）。

### 正常導線（ALG-10）

```text
Dashboard → モジュール選択 → 取込 / ドライブ選択 → 能力 diagnose 合格
  → Magika Probe（該当時）→ モジュール解析 → 根拠確認 → レポート / 他モジュールへ引き渡し
```

### 失敗導線（主導線と同等に設計）

| 状況 | 表示 | 次の操作 |
|---|---|---|
| 隔離不可 | `SANDBOX_UNAVAILABLE` | 診断結果・WIKI（隔離要件） |
| Docker 無し（Ghidra 必要時） | 静的解析 capability=unsupported | 表層のみ / 設定 |
| Magika モデル無し | content_type=unavailable | 拡張子ヒントのみ（自動危険判定しない） |
| 非リムーバブル（USB 既定） | スキャン開始不可 | 明示同意付きフォルダモード（将来） |
| 拡張子 vs 実体不一致 | 注意（三面表示） | 根拠・Magika 詳細 |
| schema 不一致 | `SCHEMA_MISMATCH` | ログ・版情報 |
| 上限到達 | partial + LIMIT_REACHED | 確定結果のみ表示 |

## 5. 要求一覧（FR / NFR）

| ID | 要求 | 優先度 | 導入 PHASE | 受入試験 |
|---|---|---|---|---|
| FR-U01 | サイドバーは `modules.manifest` から描画。未実装は無効ボタン | 必須 | U01 | AT-U01 |
| FR-U02 | 既定 3 モジュール: マルウェア解析 / 難読化判定 / USBチェック | 必須 | U01 | AT-U02 |
| FR-U03 | 追加モジュールは manifest 1 行 + 画面 + diagnose + TEST プローブ | 必須 | U01, U02 | AT-U03 |
| FR-U04 | Magika Probe: 宣言拡張子 vs magic vs Magika の三面表示。不一致は注意のみ | 必須 | U03 | AT-U04 |
| FR-U05 | ルーティングは Magika `high-confidence`。低信頼は generic で Ghidra 等へ誤投入しない | 必須 | U03 | AT-U05 |
| FR-U06 | マルウェア解析は MalCheck パイプライン契約（dynamic 既定 skip、LLM 既定 OFF） | 必須 | U05 | AT-U06 |
| FR-U07 | USBチェックは読み取り専用・対象非実行・ホスト一時領域のみ書込 | 必須 | U04 | AT-U07, ST-U01 |
| FR-U08 | モジュール間は Artifact 複製で引き渡し。UI は生パスを持たない | 必須 | U06 | AT-U08 |
| FR-U09 | Dashboard に隔離状態・モジュール別 Job 概要を表示 | 推奨 | U01 | AT-U01 |
| FR-U10 | 各モジュールから JSON / 安全 HTML レポート出力 | 必須 | U06 | AT-U09 継承 |
| NFR-U01 | 入力非実行・試料外送信なし・`--online` 無指定ではルール更新もしない | 必須 | 全体 | ST-01 系 |
| NFR-U02 | 既存難読化 PHASE 06 MVP の回帰を壊さない | 必須 | U01–U07 | 既存 TEST.py |
| NFR-U03 | sidecar 出力は Broker が schema 再検証してから UI へ | 必須 | U02 | ST-07 系 |

## 6. 情報アーキテクチャ（サイドバー）

```text
UNVEIL
 ├─ Dashboard           全モジュール横断概要
 ├─ マルウェア解析       MalCheck パイプライン UI
 ├─ 難読化判定           既存 Analyze（名称変更。中身は 00–06）
 └─ USBチェック          ドライブ選択・スキャン・結果・ルール
```

- **サイドバー幅・折畳み・テーマ切替**は [06](06-ui-ux.md) の契約を継承する。
- モジュール内のタブ（Evidence / Transform / WIKI 等）は **難読化判定**に限定。マルウェア / USB は各 [13](13-malware-module.md) / [14](14-usb-module.md) に従う。
- Settings / Reports / Rules は **グローバル画面としてサイドバーに増やさない**（モジュール内または Dashboard から遷移）。将来 manifest で `settings_route` を指定可能（[11](11-module-architecture.md)）。

## 7. 非目標

- 3 ツールを 1 つの Rust バイナリに直移植すること（sidecar 方式を採用。ADR-U001）
- 動的実行（サンドボックス detonation）を既定 ON にすること
- クラウド AI / VirusTotal / abuse.ch への試料送信
- Magika で DIE / capa / YARA / Ghidra を置換すること
- スタンドアロン MalCheck Web UI / USB-GuardDuty EXE を統合後も製品入口として維持すること（エンジン源リポは残す。OQ-U01）
- 単一の「総合危険度スコア」UI

## 8. 関連文書

| 文書 | 内容 |
|---|---|
| [11-module-architecture.md](11-module-architecture.md) | 信頼境界、sidecar、manifest |
| [12-upgrade-phase-plan.md](12-upgrade-phase-plan.md) | U00–U07、Gate |
| [13-malware-module.md](13-malware-module.md) | MalCheck 融合 |
| [14-usb-module.md](14-usb-module.md) | USB-GuardDuty 融合 |
| [15-magika-absorption.md](15-magika-absorption.md) | Magika 吸収判断 |
| [16-depth-phase-plan.md](16-depth-phase-plan.md) | U10–U16 深度 Gate |
| [17-depth-implementation-spec.md](17-depth-implementation-spec.md) | U11–U16 実装手順（別モデル向け） |
| [09-decisions-operations.md](09-decisions-operations.md) | OQ-U01–U06、ADR-U001–U009 |
