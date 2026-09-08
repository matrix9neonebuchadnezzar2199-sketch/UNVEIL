# 16 — DEPTH PHASE 管理計画（U10–U16）

- 設計版: 1.0.0 / 作成日: 2026-09-08
- 状態: **実装仕様済み。** U11–U16 実装完了（2026-09-08）。正本は [17-depth-implementation-spec.md](17-depth-implementation-spec.md)。Gate 文面は本書。
- 関係:
  - 難読化 MVP: [01-phase-plan.md](01-phase-plan.md) PHASE 00–06（完了）
  - 統合骨格: [12-upgrade-phase-plan.md](12-upgrade-phase-plan.md) U00–U07（**骨格完了**。機能完成ではない）
  - 本書: 骨格の上に載せる **深度**。番号衝突回避のため **U10 から**（U08/U09 は TEST プローブ ID と被らせない）
  - 実装手順・ファイル・TEST アサーション: [17](17-depth-implementation-spec.md)

## 0. 最小テンプレ（ALG）

### 0.1 混同表（ALG-00）追加行

| 用語 | 本 DEPTH での意味 | 混同してはならないもの |
|---|---|---|
| 骨格完了（U07） | Broker・manifest・sidecar 起動・TEST プローブが通る | マルウェア解析が DIE/capa/Ghidra 相当、USB が製品 UI 完成 |
| Magika ok | 同梱モデルがラベルを返した | 悪意判定、packer 製品特定 |
| 表層実働 | MalCheck `run_surface_analysis` が scanner 結果を返す | sidecar-minimal の hashes-only を「表層完了」と書く |
| Ghidra skipped | 能力不足または非対象のため **開始していない** | 解析したが何も出なかった（ok + 空） |
| フォルダスキャン | lab 明示の root 指定 | リムーバブル USB 検査と同一の安全主張 |

### 0.2 直交軸と禁止表示（ALG-01）

U00 の四軸を維持する。DEPTH で足す禁止:

- Magika `status=ok` を「AI 解析成功」と出さない
- `phase3_static.status=ok` を Docker 無しで出さない
- USB `overall=clean` を「媒体は安全」と出さない
- 表層 stub を scanner ヒット数 0 の成功円グラフにしない

### 0.3 能力 C（ALG-02）— DEPTH で厳格化する項目

| 能力 | 骨格（U07） | DEPTH 合格条件 |
|---|---|---|
| C_magika | スクリプト有無。未導入なら unavailable | **同梱モデル**で固定 fixture が `status=ok` |
| C_malcheck_surface | `mau/` ディレクトリ存在 | `run_surface_analysis` が JSON を返す **または** `skipped` + 理由（成功偽装なし） |
| C_docker | `docker info` | 指定 Ghidra イメージが **ローカルに存在する**（pull しない） |
| C_usb | Win32 removable フラグ（列挙空でも true） | 列挙 API が呼べる。非リムーバブルは既定拒否 |
| C_sidecar_* | スクリプト + 上流ツリー | 既存維持 |

### 0.4 信頼境界（ALG-03）

変更しない。DEPTH で足す監督:

- Magika モデルは Broker が読む。起動時ネット取得禁止
- Ghidra は sidecar が MalCheck `mau.static_analyzer` を呼ぶ（内部 `network_mode=none` / `images.get`、pull なし）。Coordinator は eligible とタイムアウトのみ。docker.sock を UI に渡さない（ADR-U009）
- USB 列挙も Broker。UI はドライブ **トークン**のみ

### 0.5 FR × 試験（ALG-09）

| ID | 要求 | PHASE | 試験 |
|---|---|---|---|
| FR-U11 | Magika 同梱。fixture で `status=ok`。ネット取得しない | U11 | U11 magika bundled |
| FR-U12 | USB: リムーバブル列挙。非リムーバブル既定拒否。結果ツリー | U12 | U12 removable refuse |
| FR-U13 | MalCheck 表層は実スキャナ **または** 明示 skipped。hashes-only を完了としない | U13 | U13 surface honest |
| FR-U14 | Ghidra は eligible かつ C_docker。否则 skipped。ok 偽装禁止 | U14 | U14 ghidra skipped-or-real |
| FR-U15 | UI handoff は Artifact ID のみ。Dashboard にモジュール Job | U15 | U15 handoff ui |
| FR-U16 | sidecar SBOM / 版固定。08 に AT-U* 定義追記 | U16 | U16 sbom |
| NFR-U02 | 難読化 smoke 回帰 | 全体 | 既存 TEST.py |
| NFR-U04 | Magika / verdict / USB overall を 1 本％に畳まない | 全体 | UI 文言レビュー |

### 0.6 失敗導線（ALG-10）

| 状況 | 表示 | 次 |
|---|---|---|
| Magika モデル欠落 | `unavailable` + degraded（拡張子+magic） | U11 同梱までルーティングに使わない |
| 表層コンテナ無し | `phase1_surface.status=skipped` | 表層タブに理由。Ghidra に進まない |
| Ghidra イメージ無し | `phase3_static.status=skipped` | 「未実施」。空の疑似 C を成功表示しない |
| 固定ディスク選択 | スキャン開始不可 | lab フォルダモードは明示フラグ |
| JS を Ghidra へ | 開始しない（既存 U05） | 難読化判定へ handoff 提案 |

### 0.7 OQ / ADR（ALG-12）

09 に追記（提案。ユーザー未承認は「決定済」と書かない）。

### 0.8 PHASE DAG（ALG-08）

機能完成（U15）と出荷（既存 G06 + U16 証跡）を混ぜない。未測定 precision を Gate に置かない。

```text
U10 残差ロック（本書）
  → U11 Magika 同梱・ルーティング実働
       ├─→ U12 USB 深度（列挙・結果 UI・workers）
       └─→ U13 MalCheck 表層実働
              → U14 Ghidra static（network:none）
                   → U15 横断 UI（handoff・Dashboard jobs）
                        → U16 DEPTH 品質（SBOM・08 試験定義）
```

U12 と U13 は G-U11 後に並行可。U14 は G-U11 **かつ** G-U13（ルーティングと正直な表層が先）。

### 0.9 TEST.py プローブ（ALG-14）

既存 U01–U08 は **残す**（骨格回帰）。DEPTH は U11–U16 を追加。SKIP は対象外 PHASE のみ。Magika 未同梱を U11 で SKIP にしない（未同梱なら fail）。

---

## 1. 骨格 U00–U07 の残差（なぜ DEPTH が必要か）

2026-09-08 `python TEST.py` は U00–U08 プローブ合格。ただし [12](12-upgrade-phase-plan.md) の **原本チケットの一部は薄いスライス**で Gate 相当の試験だけを満たしている。

| 原本 | 骨格でできたこと | 未達（DEPTH） |
|---|---|---|
| U03-01 同梱モデル | `status=unavailable` を正当な不合格として通した | モデル同梱、`status=ok` fixture |
| U03-03 ルーティング表 | JS → `ghidra_eligible=false`（magic/ext） | Magika high-confidence を実入力にする |
| U04-02 ドライブ UI | フォルダ選択スキャン | リムーバブル列挙、進捗、結果ツリー |
| U04-05 workers | `workers=1` | manifest の workers=4、config 配線 |
| U05-01 mau.main | `generate_report` + hashes-only surface | `run_surface_analysis` / intake |
| U05-02 Ghidra | static を skipped（偽装なし） | `mau.static_analyzer` 実走（network none、pull なし） |
| U05-03 UI タブ | diagnose + 開始ボタン | 表層 / 静的 / IOC タブ |
| U06-01 UI handoff | ctl 同一 SHA-256 | 結果画面の引き渡しボタン |
| U06-03 Dashboard jobs | 既存難読化件数のみ | モジュール別 Job 一覧 |
| U07-03 SBOM | 未実施 | sidecar 依存版固定 |
| U07-04 08 追記 | 未実施 | AT-U* / ST-U* を 08 に定義のみ追記 |

**禁止:** 骨格合格を「マルウェア解析モジュール完成」と README / カタログで主張すること。カタログは 2026-09-08 時点で sidecar 接続済みと書いてよい。DIE/capa/Ghidra 完了とは書かない。

---

## U10 — 残差ロック

**狙い:** 骨格と深度の境界を文書で固定し、DEPTH の FR に試験 ID を割る。

- 入口: G-U07（骨格 TEST 合格）
- 担当: PM、セキュリティ
- U10-01: 本ファイルの残差表をレビュー
- U10-02: OQ-U04–U06 / ADR-U005–U008 を [09](09-decisions-operations.md) に置く
- U10-03: [docs/README.md](README.md) の PHASE 表を「骨格完了 / 深度未着手」に更新
- 成果物: 承認済み 16、09 追記
- **Gate G-U10:** マスターが DEPTH 範囲を承認。U11 以降の実装に入ってよい
- 失敗時: 深度を Magika 同梱のみに縮小（U11 単体）

---

## U11 — Magika 同梱とルーティング実働

**狙い:** OQ-U02 を実装する。C_magika の合格を「スクリプトがある」から「モデルがラベルを返す」へ上げる。

- 入口: G-U10
- 担当: 解析 + アプリ
- U11-01: 公式 Magika バインディングとモデルをリポ（または app resource）へ **ハッシュ固定で同梱**。起動時 DL 禁止
- U11-02: `sidecars/magika_probe.py` が同梱パスだけを読む（ユーザーサイトパッケージに依存しない）
- U11-03: ルーティング表（[15](15-magika-absorption.md) §4）を Broker が `status=ok` かつ `prediction_mode=high-confidence` のときだけ適用
- U11-04: USB `APPLIES` への Magika 渡しは **U12 の should**（[17](17-depth-implementation-spec.md) §5.5）。G-U11 の必須ではない。不一致は警告であり automatic malicious にしない
- U11-05: NOTICE / ライセンス（Apache-2.0）
- 成果物: 同梱モデル、probe `ok`、UI 三面が Magika 列を埋める
- **Gate G-U11:** TEST **U11 magika bundled** — 固定 JS/PE fixture で `status=ok`。ネット無し。既存 U03（unavailable 許容）は **U11 以降「ok 必須」に更新**するか、U03 を骨格回帰のまま残し U11 を追加（推奨: 両方残す）
- 失敗時: 同梱を諦め `unavailable` 維持。U12/U13 は拡張子+magic のみ（UI degraded）

画面: 既存 Analyze / Malware の Probe カードをモック差分更新（**既存サイバーテーマ維持**。新規デザイン選択なし）。

---

## U12 — USB 深度

**狙い:** [14](14-usb-module.md) の製品 UI と列挙契約。

- 入口: G-U11（並行: G-U13 非依存）
- 担当: Python sidecar + UI
- U12-01: `backend/drives.py` を Broker 経由で列挙。UI はトークン。固定ディスクは既定拒否
- U12-02: 結果ツリー、severity フィルタ、進捗（完了イベントでよい。SSE 必須にしない）
- U12-03: `workers` を manifest / settings から pipeline へ（既定 4、上限明示）
- U12-04: フォルダモードは `UNVEIL_USB_FOLDER_MODE=1` または UI の明示同意（OQ-U06）。TEST の tempfile スキャンは lab フラグで維持
- U12-05: 停止ボタンを実キャンセル（`cancel_event`）に接続
- 成果物: `/usb` が UGD Dashboard/Results 相当（VT UI なし）
- **Gate G-U12:** **U12 removable refuse** — 非リムーバブル root を既定拒否。既存 U07 readonly 回帰。EICAR があれば YARA hit（無ければ rules_loaded=0 を失敗にしない）
- 失敗時: フォルダモードのみ残し列挙を disabled

画面: [mockup_usb.html](mockup_usb.html) をドライブ一覧付きに更新してから実装。

---

## U13 — MalCheck 表層実働

**狙い:** schema 2.1 の `phase1_surface` を stub から外す。無い能力は skipped。

- 入口: G-U11
- 担当: Python sidecar
- U13-01: sidecar が `mau.phase_router` / `run_surface_analysis` を呼ぶ。hashes-only を `status=ok` にしない
- U13-02: コンテナ無しなら `status=skipped` + 理由。ホスト fallback は lab ラベルのみ（ADR-U006）
- U13-03: intake（ZIP/7z、password `infected`）を Broker 一時領域で実行。展開物はホスト temp、試料実行なし
- U13-04: UI 表層タブ: scanner 結果、YARA/capa は **実際に返ったキーだけ**表示
- U13-05: unpack（UPX）は cap 付き。失敗は skipped/failed。成功偽装なし
- 成果物: 合成 PE / EICAR で表層 JSON が stub でないこと
- **Gate G-U13:** **U13 surface honest** — `phase1_surface.status` が `ok` なら `hashes.sha256` 以外の scanner キーが 1 つ以上 **または** DIE/capa 欠落が `skipped` 理由に含まれる。hashes-only `partial` を ok としない
- 失敗時: stub に戻さず skipped を出す。モジュールは表層なしでも難読化/USB は維持

画面: [mockup_malware.html](mockup_malware.html) の表層タブを「stub ではない」サンプルに更新してから実装。

---

## U14 — Ghidra static

**狙い:** OQ-U03 / OQ-U05。静的解析を **開始したときだけ** `ok`。

- 入口: G-U11 かつ G-U13
- 担当: sidecar + Coordinator（docker CLI は mau 内。Rust 新規 supervisor を作らない）
- U14-01: イメージはローカル tag のみ（OQ-U05）。`docker pull` を製品経路に置かない
- U14-02: sidecar が `run_static_analysis` を呼ぶ。Coordinator は `--ghidra-eligible` とタイムアウトのみ（ADR-U009）
- U14-03: 投入条件: Magika high-confidence PE 系 **かつ** `is_analyzable_binary`。JS/txt は開始しない（U05 回帰）
- U14-04: 疑似 C / 関数一覧は blob_ref。UI 生パスなし
- U14-05: イメージ無し・非対象は `phase3_static.status=skipped`。`ok` 空結果で埋めない
- 成果物: lab イメージがある環境でのみ static ok。CI 既定は skipped 合格
- **Gate G-U14:** **U14 ghidra skipped-or-real** — JS fixture で static が skipped/不開始。Docker 無しで `status=ok` が無い。イメージ有り lab は別ジョブで任意（Gate 必須にしない）
- **状態（2026-09-08）:** ✅ G-U14 合格（TEST U14 pass）
- 失敗時: U13 表層のみで出荷可能。malware モジュールは static unsupported を明示

---

## U15 — 横断 UI

**狙い:** ctl で通った U08 を画面に載せる。

- 入口: G-U12 かつ G-U13（U14 は非必須。Ghidra 無しでも handoff 可）
- 担当: UI + Coordinator
- U15-01: USB 結果 → 「マルウェア解析へ」。Malware 文字列領域 → 「難読化判定へ」。渡すのは Artifact ID
- U15-02: 同一 SHA-256 なら Magika 再実行しない（ADR-U007）
- U15-03: Dashboard にモジュール別 Job 行（module_id / status / sha256 短縮）
- U15-04: Markdown 出力先（ワークスペース / 保存ダイアログ / クリップボード）を **実装**（モック済み）
- 成果物: 画面導線、既存 U08 回帰
- **Gate G-U15:** **U15 handoff ui** — ctl またはヘッドレス IPC で USB 選択ファイルが malware の `input_artifact_id` になる。生パスが JSON 既定に出ない
- **状態（2026-09-08）:** ✅ G-U15 合格（TEST U15 pass）
- 失敗時: ボタンを隠し ctl handoff のみ

画面: Dashboard Job 表を [mockup.html](mockup.html) に足してから実装。

---

## U16 — DEPTH 品質

**狙い:** 証跡。出荷承認（署名・人間 G06）とは別。

- 入口: G-U15
- 担当: QA
- U16-01: sidecar の `requirements.lock` / SBOM（Magika、UGD、MalCheck の版）
- U16-02: [08](08-quality-validation.md) に AT-U11–U15 / ST-U01 を **試験定義のみ**追記（合格％は空）
- U16-03: 既存 U01–U08 + U11–U15 が同一 `TEST.py` で通る
- U16-04: カタログ文言が骨格と DEPTH を混同しない
- 成果物: lockfile、08 追記、TEST 全プローブ
- **Gate G-U16:** **U16 sbom** — lockfile が存在する。Critical/High 既知ゼロ（DEPTH 範囲）。未測定 precision を合格条件にしない
- **状態（2026-09-08）:** ✅ G-U16 合格（TEST U16 pass）。U11–U16 実装済み
- 失敗時: 不合格モジュールを `disabled`。骨格機能（難読化）は残す

---

## 2. TEST.py DEPTH プローブ

| プローブ | 内容 | PHASE |
|---|---|---|
| U11 magika bundled | 同梱モデル、`status=ok`、オフライン | U11 |
| U12 removable refuse | 非リムーバブル既定拒否 | U12 |
| U13 surface honest | stub hashes-only を ok としない | U13 |
| U14 ghidra skipped-or-real | JS 非投入。無 Docker で static ok なし | U14 |
| U15 handoff ui | Artifact ID 引き渡し、パス非露出 | U15 |
| U16 sbom | lockfile 存在 | U16 |

骨格 U01–U08 は常時回帰。

---

## 3. 規模目安（暫定）

| PHASE | 人週 | 依存 |
|---|---|---|
| U10 | 0.2–0.5 | U07 |
| U11 | 1–2 | U10 |
| U12 | 2–3 | U11 |
| U13 | 2–4 | U11 |
| U14 | 2–4 | U11, U13 |
| U15 | 1–2 | U12, U13 |
| U16 | 1 | U15 |

U12/U13 並行で 1–2 人週短縮可。Ghidra イメージ構築は U14 に含む（CI 必須にしない）。

## 4. 変更管理

- 12 の U00–U07 本文は骨格定義として残す。深度の Gate 正本は本書。**実装の正本は [17](17-depth-implementation-spec.md)**
- 10–15 と衝突する安全緩和は 09 同 PR
- 画面変更は既存サイバーモックを先に差分更新（新規 4 択は不要）
- 本書と 17 が衝突したら **安全側（ok 偽装禁止・pull 禁止・パス非露出）を優先**し、手順の細部は 17

## 5. G-U10

2026-09-08: マスター指示「実装を詳細に設計、完了後に別モデルで実装する」により、DEPTH 範囲（U11–U16）と 17 の手順で実装開始を許可。このチャットではコードを書かない。
