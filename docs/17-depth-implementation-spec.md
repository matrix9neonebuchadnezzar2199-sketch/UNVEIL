# 17 — DEPTH 実装仕様（U11–U16）

- 設計版: 1.0.0 / 作成日: 2026-09-08
- 状態: **実装者向け正本。** PHASE 管理は [16](16-depth-phase-plan.md)。製品定義は [10](10-unified-workbench.md)。
- 読者: 別モデル / 別チャットの実装者。計画を発明しない。本書と 16 の Gate 以外を「ついで」に足さない。

---

## 0. 実装者契約

1. **1 PHASE ずつ。** その PHASE の TEST プローブが pass するまで次へ進まない。
2. 各 PHASE の終わりに `python TEST.py`（`H:\CURSOR\UNVEIL`）。既存 U00–U08 と smoke を壊したら戻す。
3. 日本語コメントは「なぜ」だけ。識別子・コミットメッセージは英語。
4. 呼びかけは不要。コードと TEST を直す。
5. 検体実行・試料の外送信・`docker pull` を製品経路に置かない。`--online` 既定 OFF。
6. Magika score / MalCheck verdict / USB overall を 1 本の危険度％にしない。
7. `phase3_static.status=ok` を、Ghidra を動かしていないときに出さない。
8. 画面変更は既存サイバーテーマ維持。`10-mockup-html` のデザイン 4 択は出さない（`existing`）。
9. コミットはマスターが頼むまでしない（User Rules）。
10. 計画ファイル [12](12-upgrade-phase-plan.md) / [16](16-depth-phase-plan.md) の Gate 文面を「合格したことにして」書き換えない。実装後に 16 の状態行だけ更新してよい。

### 0.1 開始コマンド

```powershell
cd H:\CURSOR\UNVEIL
python TEST.py
```

合格していること（骨格）。不合格なら DEPTH に入らない。

### 0.2 推奨実装順（発明しない）

```text
U11 Magika 同梱 → TEST U11
U12 USB 深度   → TEST U12（U07 は FOLDER_MODE で回帰）
U13 表層実働   → TEST U13（U06 schema 2.1 は維持）
U14 Ghidra     → TEST U14
U15 横断 UI    → TEST U15
U16 SBOM / 08  → TEST U16
```

U12 と U13 は U11 後なら並行可。**U14 は U11 かつ U13 の後。**

---

## 1. 触ってはならないもの

| 禁止 | 理由 |
|---|---|
| `crates/analysis-worker` / `crates/engine` の難読化検知ロジック | NFR-U02 |
| `unveil-ctl smoke` の AT 順序を変えること | 骨格回帰 |
| MalCheck `web_ui/` / UGD FastAPI を UNVEIL に移植 | ADR-U001 / OQ-U01 |
| Magika を verdict に混ぜる / 再学習 | ADR-U004 |
| `docker pull` / Magika の起動時ネット取得 | OQ-U02 / OQ-U05 |
| 実マルウェア fixture を TEST に入れる | 07 / コーパス規約 |
| ブラウザ `Notification` API | ワークスペース UI 方針 |
| Drive 側 Obsidian へのノート | 正本は `H:\CURSOR\ObsidianVault` のみ。DEPTH では日記以外の Vault 新規は不要 |

MalCheck `mau/` と USB-GuardDuty `backend/` は **呼ぶ・薄いパッチ**は可。書き直しは不可。

---

## 2. 現状マップ（2026-09-08 骨格）

| 部品 | パス | 今の動き |
|---|---|---|
| manifest | `UNVEIL/modules.manifest.json` | 3 モジュール `mvp` |
| Magika sidecar | `sidecars/magika_probe.py` | `from magika import Magika()`。未導入なら `status=unavailable` |
| C_magika | `crates/coordinator/src/modules.rs` `check_magika` | stdout に `"status": "ok"` があれば true |
| Probe | `crates/coordinator/src/content_type.rs` | 一時ファイル → magika_probe `--path` |
| ルーティング | `crates/contracts/src/modules.rs` `ghidra_eligible` | Magika `ok` なら PE 系 label のみ。JS は常に false |
| USB sidecar | `sidecars/usb_sidecar.py` | `--readonly` 必須。`run_scan(..., workers=1)`。フォルダ無検査 |
| MalCheck sidecar | `sidecars/malcheck_sidecar.py` | `generate_report` + **hashes-only** `phase1_surface.status=partial`。`phase3_static.status==ok` なら **拒否**（U14 で改める） |
| ctl | `crates/coordinator/src/bin/unveil-ctl.rs` | `modules/diagnose-module/probe/usb-scan/malcheck/handoff/export-md` |
| UI | `apps/desktop/src` | サイドバー manifest。Malware/Usb は diagnose+開始。handoff ボタンなし |
| TEST | `TEST.py` `_run_upgrade_probes` | U00–U08。U03 は `ok` **または** `unavailable` |

上流:

- MalCheck: `H:\CURSOR\MalCheck`（`run_surface_analysis`, `run_static_analysis`, `is_analyzable_binary`, image `ghidra-headless:latest`）
- UGD: `H:\CURSOR\USB-GuardDuty`（`run_scan`, `list_removable_drives`, `APPLIES`）

環境変数（既存）:

| 変数 | 意味 |
|---|---|
| `UNVEIL_ROOT` | リポジトリ root（TEST がセット） |
| `UNVEIL_PYTHON` | python 実行ファイル |
| `UNVEIL_CURSOR_ROOT` | sidecar がセットされる親（`H:\CURSOR`） |
| `UNVEIL_MANIFEST` | manifest 上書き |
| `UNVEIL_WORKER` / `UNVEIL_KNOWLEDGE` | smoke |

DEPTH で追加する変数は各 PHASE に書く。

---

## 3. 共通実装規則

### 3.1 Sidecar JSON

- 成功も失敗も **stdout 最終行が 1 個の JSON object**。ログは stderr。
- 終了コード: 契約違反・import 失敗は 1。Magika `unavailable` は **0**（Broker が status を読む）。
- 絶対パスを UI 向け JSON の既定フィールドに置かない。必要なら Coordinator のメモリにだけ持つ。

### 3.2 `run_python_json`

`crates/coordinator/src/modules.rs`。タイムアウト引数は今未使用。U13/U14 で **実タイムアウト（経過後 kill）** を入れてよい。既定: Magika 30s、USB 120s、MalCheck 180s、Ghidra 経路 600s。

### 3.3 エラー

既存 `ErrorCode::ModuleUnavailable` / `SchemaMismatch`。新しいコードをむやみに増やさない。

### 3.4 TEST ヘルパ

`_ctl_json` / `_run_upgrade_probes` にプローブを **末尾追加**。既存 U00–U08 の判定式を緩めない。U12 で U07 に環境変数を足すのは **指定どおり**（§5.5）。

U00 の docs リストに `16-depth-phase-plan` と `17-depth-implementation-spec` を追加する（U11 着手時の最初の差分で可）。

---

## 4. U11 — Magika 同梱

### 4.1 目的

C_magika を「import できた」ではなく「**同梱モデルで `status=ok`**」にする。実行時ネット禁止。

### 4.2 ディレクトリ契約

```text
UNVEIL/third_party/magika/
  VERSION.txt          # magika PyPI 版 + モデル名
  SHA256SUMS           # model.onnx と config.min.json
  NOTICE               # Apache-2.0
  models/<model_name>/
    model.onnx
    config.min.json
```

- `Magika(model_dir=Path(models/<model_name>))`。ディレクトリに `model.onnx` と `config.min.json` が必須。
- Python パッケージ `magika` は **同梱 venv または sidecars の vendor**。実行時 `pip install` 禁止。
- 実装者のマシンで **1 回だけ** オフライン用に取得して上記へコピーしてよい。その取得をアプリ起動に残さない。
- onnx が大きくて git に載せない判断をするなら、`SHA256SUMS` と欠落時 `unavailable` を必須にし、TEST U11 はファイルが無いと **fail**（SKIP 禁止）。マスターが LFS を使うならその旨を VERSION.txt に書く。

推奨ピン: PyPI `magika` の実装時点の最新安定。`VERSION.txt` に版を書け。再学習しない（ADR-U002）。

### 4.3 `sidecars/magika_probe.py`

変更:

- 環境変数 `UNVEIL_MAGIKA_MODEL_DIR` があればそれを `model_dir` に使う。無ければ `repo/third_party/magika/models/<唯一の子ディレクトリ>`。
- モデルディレクトリが無ければ `unavailable`（例外でプロセスを落とさない）。
- `prediction_mode` は high-confidence 相当（公式 `PredictionMode.HIGH_CONFIDENCE` があれば使う）。
- `--self-test` は JS snippet で `status=ok` を狙う（ラベル名は `javascript` 系を期待するが、**ok が Gate**。ラベル文字列の完全一致は必須にしない）。
- stdout JSON キーは既存 `MagikaOutput` と一致: `status, dl_label, output_label, score, prediction_mode, mime_type, is_text`。余分な `reason` は可（serde は未知フィールド無視）。

### 4.4 Coordinator

- `check_magika`: `--self-test` の `status` が **ok** のときだけ `available=true`。
- `content_type.rs`: 変更最小。モデル欠落は今どおり `unavailable`。libmagic へ落とさない。
- `ghidra_eligible`（contracts）は **触らない**（既に Magika ok 時は PE 系のみ）。

### 4.5 UI

Analyze の Probe カード（既存）で Magika 列が `ok` なら `output_label` を出す。`score` の横に「タイプ信頼度（悪意ではない）」を 1 行。禁止文案は [15](15-magika-absorption.md) §7。

モック: `docs/mockup_analyze.html` の Probe に同じ注意文を足す（サイバー維持）。

### 4.6 TEST **U11 magika bundled**

```
1. (REPO_ROOT / "third_party/magika/models").iterdir() に model.onnx がある
2. magika_probe.py --self-test → status == "ok"
3. unveil-ctl probe --path hello.js --name app.js → magika.status == "ok"
4. 同 JS で ghidra_eligible is False（U05 回帰）
```

既存 **U03** は残す（`ok|unavailable`）。U11 が ok 必須。

### 4.7 DoD

- [ ] 起動時に Magika がネットへ出ない（コードに urllib/requests でモデル取得が無い）
- [ ] `python TEST.py` で U11 pass、U00–U08 pass
- [ ] NOTICE

---

## 5. U12 — USB 深度

### 5.1 目的

リムーバブル列挙。非リムーバブル既定拒否。結果の見える化。workers 配線。Magika を analyzer 選択の追加入力に。

### 5.2 フォルダモード（回帰契約）

| 条件 | 動作 |
|---|---|
| 既定 | `GetDriveTypeW == DRIVE_REMOVABLE` の root だけスキャン可 |
| `UNVEIL_USB_FOLDER_MODE=1` | 任意ディレクトリ可（TEST / lab） |
| 固定ディスクを既定でスキャン | **拒否** JSON `{ok:false, reason:"not_removable"}` |

**既存 U07 / U08 は tempfile を使う。** U12 実装と同時に TEST 側へ `env["UNVEIL_USB_FOLDER_MODE"]="1"` を `_run_upgrade_probes` の USB 節でセットする。これを忘れると骨格 U07 が落ちる。

### 5.3 `sidecars/usb_sidecar.py`

追加サブコマンド（同一ファイル）:

```
--list-drives
  → { ok, drives: [ { token_hint, letter, bus, vid, pid, composite_suspect, capacity_bytes? } ] }
  パス文字列は letter（"E:"）まで。UNC や NT デバイスパスを UI に出さない。

--root PATH --readonly [--workers N] [--folder-mode]
  workers 既定 4。上限 8。TEST は ctl から --workers 1。
```

`--online` は今どおり拒否。

列挙は `USB-GuardDuty/backend/drives.py` の `list_removable_drives` を import。

### 5.4 Coordinator / ctl / IPC

- `Inner` に `usb_scan_roots: HashMap<job_id, PathBuf>` を追加。UI JSON に root を載せない。
- `usb_scan_root`: folder_mode でなければ、root がリムーバブルか sidecar に確認させてからスキャン。拒否は `ModuleUnavailable`。
- 新 API `list_usb_drives() -> Value`（トークン発行: 既存 `register_path` と同じく opaque token → 内部 Path）。
- `unveil-ctl usb-drives` / `usb-scan --root --workers --folder-mode`
- Tauri: `usb_drives_cmd`。`usb_scan_cmd` はドライブ token **または** フォルダ（folder_mode 時だけ rfd）。既定 UI は列挙リスト。フォルダ選択は「lab フォルダ」チェック時のみ。

停止: `cancel_event` を sidecar がサポートするなら `--` で。U12 最小は「停止ボタンが cancel を送る」。未配線の disabled は不可（今 disabled のままは U12 未完）。最小実装: プロセス kill + Job status cancelled。完全な per-file cancel は should。

### 5.5 Magika → APPLIES

UGD `pipeline._applies` に **オプション** `magika_label` を足す（既定 None で現行と同等）。

写像（high-confidence のときだけ）:

| Magika `output_label`（部分一致可） | 足す拡張子扱い |
|---|---|
| javascript / js | `.js` → script_analyze |
| pebin / exe / dll | `.exe` → pe_analyze |
| pdf | `.pdf` |
| zip / gzip | `.zip` / 圧縮 |

不一致は malicious 自動にしない。Coordinator はスキャン前に root 配下を全部 probe しない（高い）。**選択ファイルまたは inventory 後の再スキャンは U12 ではやらない。** 最小: sidecar が各ファイル header だけで magika を呼ぶのは高コストなので、U12 では **拡張子不一致の警告は Broker が取込済み Artifact にだけ**。USB 全件 Magika は U12 の should。必須は列挙+拒否+workers+結果ツリー。

実装者が余力があれば per-file magika を workers 内で。Gate 必須にしない。16 の U11-04 は「USB APPLIES に渡す」と書いてあるが、**本仕様では U12 should、Gate は列挙拒否**。

### 5.6 UI `Usb.tsx`

- ドライブ一覧（letter、複合疑いフラグ）。スキャン対象を選ぶ。
- 結果: `inventory` テーブル（name, sha256 短縮, size）。`overall` はラベル＋「安全証明ではない」。
- findings_count。ツリーはフラット表で可。
- Markdown 出力（既存ボタン）。

モック: `mockup_usb.html` にドライブ行を足す。

### 5.7 TEST **U12 removable refuse**

```
UNVEIL_USB_FOLDER_MODE を消した状態で
  usb-scan --root <tempdir> → rc != 0 かつ ok でない
UNVEIL_USB_FOLDER_MODE=1 で既存 U07 と同じ tempfile → rc==0 かつ対象へ書込なし
```

`C:\Windows` を root にする試験は環境依存なので使わない。tempdir で足りる。

### 5.8 DoD

- [ ] 既定で tempfile スキャンが失敗する
- [ ] FOLDER_MODE=1 で U07/U08 pass
- [ ] `usb-drives` が JSON（空配列可）
- [ ] workers が 1 以外を渡せる（ctl `--workers`）

---

## 6. U13 — MalCheck 表層実働

### 6.1 目的

`phase1_surface` から **unveil sidecar surface-minimal** を消す。スキャナが無ければ `skipped`。hashes-only を `ok` にしない。

### 6.2 `sidecars/malcheck_sidecar.py`

フロー:

```
PYTHONPATH += MalCheck
MAU_EXPORT_REPORTS=0
dynamic = {status: skipped, reason: UNVEIL default}
surface = run_surface_analysis(sample, container=None)  # 既存 API
  失敗 → {status: skipped, reason: <例外の短い文>}
  成功 → 返却 JSON をそのまま（status を嘘の ok に上書きしない）
unpack = 既定 skipped。UPX はファイルが PE かつ env UNVEIL_UNPACK=1 のときだけ（U13 必須ではない。should）
static = U14 まで skipped（U13 では Ghidra を開始しない）
generate_report(surface, dynamic, static, html=False, out_dir=temp)
_paths を落とす
```

`run_surface_analysis` は `H:\CURSOR\MalCheck\mau\surface_runner.py`。コンテナ名 env が空ならローカル `scripts/remnux/analyze.py`。それが無い/失敗なら **skipped**。ホスト成功は lab だが、analyze.py が動くなら使ってよい。製品ラベル「lab-fallback」を JSON `surface.isolation` に付ける。

**削除:** `status: partial` + `unveil sidecar surface-minimal` を `ok` 相当として出すコード。

hashes が空なら sidecar が sha256 を `surface.hashes.sha256` に **補完**してよい（識別用）。それだけで status を ok にしない。

### 6.3 Coordinator

- `malcheck_sample` は今どおり probe を payload に載せる。
- `--ghidra-eligible` は U14 まで無視してよい（U13 は static しない）。

### 6.4 UI `Malware.tsx`

表層タブ:

- `phase1_surface.status` と `reason`
- status=ok のときだけ scanner キーを `<pre>` または表（巨大 JSON は 32KiB で切る）
- skipped のとき成功円を出さない

モック: `mockup_malware.html` の表層を「skipped: analyze.py missing」例と「ok: yara_hits」例のどちらか。stub hashes-only を完成例にしない。

### 6.5 TEST **U13 surface honest**

malcheck 合成 PE の report について:

```
surface = payload.report.phase1_surface
"surface-minimal" not in str(surface.get("reason"))
surface.status in ("ok", "skipped", "failed", "error")  # partial は hashes-only 禁止
if surface.status == "ok":
    keys = set(surface) - {"status","reason","hashes","file_name","isolation"}
    assert keys  # 何か scanner 由来がある
schema 2.1 と dynamic skipped は U06 のまま
```

### 6.6 DoD

- [ ] ソースに `surface-minimal` が残っていない
- [ ] U06 pass
- [ ] Docker 無しでも sidecar が fake Ghidra ok を出さない（U13 では static skipped）

---

## 7. U14 — Ghidra static

### 7.1 目的

eligible かつローカル image があるときだけ `mau.static_analyzer.run_static_analysis` を呼ぶ。無ければ skipped。**Rust から docker run しない**（ADR-U009）。`docker pull` しない。`images.get` 失敗は MalCheck が `StaticError` にする → sidecar は skipped。

`MAU_GHIDRA_NETWORK_NONE` 既定 1 を維持（`static_analyzer.py` 既存）。

### 7.2 投入条件（AND）

Coordinator が sidecar に渡す:

```
--ghidra-eligible true|false   # contracts::ghidra_eligible の結果
```

sidecar 側:

```
eligible = arg && is_analyzable_binary(surface.file_type, suffix)
image = env UNVEIL_GHIDRA_IMAGE or "ghidra-headless:latest"
if not eligible: static = skipped "not eligible"
elif not docker: static = skipped "docker unavailable"
else:
    try run_static_analysis(sample, image=image, timeout_sec=...)
    except: static = skipped or failed（ok にしない）
```

JS / Magika javascript は eligible false（U05）。

### 7.3 現コードの必修正

`malcheck_sidecar.py` の:

```python
if ghidra_ran: return _fail("refusing forged Ghidra success")
```

**U14 で削除。** 代わりに「eligible でないのに status=ok」なら fail。正当な ok は許可。

RESULTS_DIR は UNVEIL workspace temp。ホスト cwd に `results/` を残さない。

### 7.4 C_docker

`check_docker`: `docker info` 成功 **かつ** `docker image inspect {UNVEIL_GHIDRA_IMAGE}` 成功。image 無しは C_docker false。malware ジョブ開始は今どおり sidecar+isolation。static は skipped。診断 UI に「Ghidra image なし」を出す。

### 7.5 UI

静的タブ: `phase3_static.status`。ok のとき関数数や analysis 要約（あれば）。skipped は「未実施」＋ reason。空の疑似 C プレースホルダを成功色にしない。

### 7.6 TEST **U14 ghidra skipped-or-real**

```
malcheck app.js → phase3_static.status != "ok"（skipped/failed）
malcheck 合成 PE（Docker 無し環境）→ status != "ok"
payload に phase3_static.status=="ok" かつ ghidra_eligible false が無い
```

イメージがある lab での ok は Gate 必須にしない。

### 7.7 DoD

- [ ] ソースに `docker pull` が無い
- [ ] JS で static ok が無い
- [ ] U05 / U06 回帰

---

## 8. U15 — 横断 UI

### 8.1 目的

ctl の U08 を画面に載せる。生パスを UI に出さない。

### 8.2 Coordinator

```
usb_scan_roots: job_id → PathBuf  # U12 で追加済み想定

handoff_usb_file(job_id, relative_name) -> ModuleJob
  root = usb_scan_roots[job_id]
  path = root.join(relative_name)  # 正規化、root 外なら PermissionDenied
  bytes = read
  artifact = import_bytes(bytes, relative_name の basename)
  malcheck_artifact(artifact_id)
  同一 sha256（U08 と同じ）

handoff_to_deobfuscation(artifact_id) -> 現在の artifact を current にして UI が deobfuscation へ
```

Magika キャッシュ（ADR-U007）: `Inner.probe_by_sha256: HashMap<String, ContentTypeProbe>`。`probe_bytes` の前に sha256 ヒットなら再利用。

Dashboard: `overview()` の jobs を module kind 別に数える。`store.insert_job` の kind は既に `usb` / `malware` / `analyze`。`job_outcomes` に加えて `modules: Vec<NamedCount>` を **既存 DashboardOverview にフィールド追加**すると TS/Rust 両方必要。互換を壊したくなければ overview JSON に `module_jobs` を任意フィールドで足し、TS 型を拡張。

### 8.3 Tauri / UI

- `handoff_usb_cmd { job_id, name }`
- `Usb.tsx`: inventory 行に「マルウェア解析へ」
- `Malware.tsx`: 「難読化判定へ」（`setPage` は App から callback）
- `App.tsx`: `onHandoffDeobfuscation` で page=deobfuscation
- Markdown 出力先: clipboard / workspace / picker。既存 `markdown_export_cmd` の `dest` を Analyze/Malware/Usb から選ばせる（今は clipboard のみ）

モック: `mockup.html` に Job 表。`mockup_usb.html` に handoff ボタン。

### 8.4 TEST **U15 handoff ui**

GUI E2E は必須にしない。ヘッドレス:

```
unveil-ctl に handoff-named --job は重いので、既存 handoff に加え:

Coordinator 単体試験または ctl:
  usb-scan folder_mode
  新しい ctl: import-from-scan 相当が無くても
  handoff --root --sample が U08 のまま pass

追加: レポート JSON / module job JSON に Windows の `\Users\` 絶対パスが無い
  （payload.inventory[].name は相対）
```

必須アサーション:

```
malcheck / usb-scan の stdout JSON を文字列化して
  re.search(r"[A-Za-z]:\\\\Users\\\\", text) が無い
相対 name のみ
```

### 8.5 DoD

- [ ] USB 結果から malware へ Artifact 経由
- [ ] Markdown dest 3 種が UI から呼べる
- [ ] U08 pass

---

## 9. U16 — SBOM と 08

### 9.1 成果物

| ファイル | 内容 |
|---|---|
| `UNVEIL/sidecars/requirements.lock` | magika==x.y.z 等、sidecar が import するもの |
| `UNVEIL/third_party/magika/SHA256SUMS` | U11 で作成済みなら再利用 |
| `UNVEIL/docs/08-quality-validation.md` | 節を追加。AT-U11–U15 / ST-U01 の **定義のみ**。precision ％を Gate に書かない |

ST-U01: 「USB スキャン中に対象 root へ write しない」— 既存 U07 を参照する一文で足りる。

カタログ: UNVEIL の summary に「Magika 同梱」「Ghidra はローカル image 時のみ」を事実に合わせて更新。未実装を完成と書かない。

### 9.2 TEST **U16 sbom**

```
(REPO_ROOT / "sidecars/requirements.lock").is_file()
08 に "AT-U11" または "U11 magika bundled" の文字列
```

### 9.3 DoD

- [ ] 全プローブ pass
- [ ] 16 の状態行を「U11–U16 実装済み」に更新（この PHASE の最後）
- [ ] 開発日記 `ObsidianVault/90_DevLog/YYYY-MM-DD.md`
- [ ] `H:\CURSOR\カタログ.html` UNVEIL 行（事実のみ）

---

## 10. 画面モック（PHASE ごと）

実装コードより先に、その PHASE で触るモック HTML を **既存ファイル差分**する。新規 `mockup-template` 禁止。配色は今のサイバー CSS 変数のまま。

| PHASE | ファイル |
|---|---|
| U11 | `mockup_analyze.html` Probe 注意文 |
| U12 | `mockup_usb.html` ドライブ一覧 |
| U13–U14 | `mockup_malware.html` 表層/静的タブの正直な status |
| U15 | `mockup.html` Job 表、handoff、MD 出力先 |

---

## 11. API 差分一覧（実装者が足すもの）

### ctl

```
usb-drives
usb-scan --root --readonly --workers N  [--folder-mode]
malcheck --sample --ghidra-eligible true|false
```

folder-mode は env でも可。ctl フラグがあれば TEST が楽。

### Tauri commands

```
usb_drives_cmd
usb_scan_drive_cmd { token }
handoff_usb_cmd { job_id, name }
markdown_export_cmd  # 既存 dest を UI から使う
```

### 型

`DashboardOverview` に `module_jobs?: {id, count}[]`（任意）。

`PageId` は変更しない。

---

## 12. 失敗時の縮退（16 と同じ。実装者が勝手に消さない）

| PHASE | 失敗時 |
|---|---|
| U11 | Magika unavailable 維持。U12/U13 は ext+magic |
| U12 | フォルダモードのみ。列挙 disabled |
| U13 | surface skipped。stub に戻さない |
| U14 | static skipped。表層は残す |
| U15 | ボタン非表示。ctl handoff のみ |

---

## 13. 倫理・OPSEC

- 解析は lab。識別子は汎用。
- TEST は EICAR / 合成 MZ / 短い JS のみ。
- sidecar 出力の IoC をオンライン照会しない。
- 顧客名・実在 C2 をコードに書かない。

---

## 14. 別チャット1通目（実装者用・コピー）

実装開始チャットにこれを貼る。計画を再発明しない。

```text
UNVEIL DEPTH を docs/17-depth-implementation-spec.md どおり U11 から実装する。
正本は .cursor/rules。User Rules は空が正常。
1 PHASE ずつ。各 PHASE 後に H:\CURSOR\UNVEIL で python TEST.py。
骨格 U00–U08 を壊さない。docker pull 禁止。Magika 実行時ネット禁止。
Ghidra ok 偽装禁止。malcheck_sidecar の hashes-only surface-minimal を消すのは U13。
U12 では UNVEIL_USB_FOLDER_MODE を TEST の U07/U08 に足す。
関連: @H:\CURSOR\UNVEIL\docs\17-depth-implementation-spec.md
関連: @H:\CURSOR\UNVEIL\docs\16-depth-phase-plan.md
やりたいこと: U11 Magika 同梱から順に実装し TEST を緑にする
```

---

## 15. PHASE ごとの主な変更ファイル

計画を発明せず、この表以外を「ついで」に広く触らない。

| PHASE | 主なパス |
|---|---|
| U11 | `sidecars/magika_probe.py`、`third_party/magika/`、`crates/coordinator/src/modules.rs`（`check_magika`）、`TEST.py`、`docs/mockup_analyze.html`、`apps/desktop/src/views/Analyze.tsx` |
| U12 | `sidecars/usb_sidecar.py`、`crates/coordinator`（Inner / ctl / usb）、`apps/desktop/src-tauri/src/lib.rs`、`apps/desktop/src/views/Usb.tsx`、`TEST.py`（U07 に `UNVEIL_USB_FOLDER_MODE=1`）、`docs/mockup_usb.html`。UGD `pipeline._applies` は should |
| U13 | `sidecars/malcheck_sidecar.py`（`run_surface_analysis`。`surface-minimal` 削除）、`Malware.tsx`、`TEST.py`、`docs/mockup_malware.html` |
| U14 | 同 sidecar（`--ghidra-eligible`、`run_static_analysis`、forged-ok 拒否の削除）、`modules.rs` `check_docker`、`Malware.tsx`、`TEST.py` |
| U15 | Coordinator `usb_scan_roots` / `probe_by_sha256`、`App.tsx` / `Usb.tsx` / `Malware.tsx` / Dashboard、`lib.rs` Tauri、`TEST.py`、`docs/mockup.html` |
| U16 | `sidecars/requirements.lock`、`docs/08-quality-validation.md`、`docs/16-depth-phase-plan.md` 状態行、`H:\CURSOR\カタログ.html` UNVEIL 行 |

難読化 `crates/engine` と `unveil-ctl smoke` の AT 順は全 PHASE で触らない。
