# 14 — USBチェックモジュール（USB-GuardDuty 融合）

- 設計版: 1.0.0 / 作成日: 2026-09-08
- 状態: UPGRADE 設計案（U04）
- エンジン正本: `H:\CURSOR\USB-GuardDuty\backend\`
- UI: UNVEIL 内 `/usb`。UGD 単体 EXE / pywebview は **製品入口にしない**（エンジン源としてリポ維持。OQ-U01）

## 1. モジュールの位置づけ

| 項目 | 内容 |
|---|---|
| 目的 | リムーバブル USB 上のファイルを **読み取り専用** で静的検査 |
| 非目的 | BadUSB / ファームウェア検出、媒体消毒、ホスト侵害の保証、動的実行 |
| worker | `sidecar:usb`（Python multiprocessing pool） |
| 契約 | UGD scan result JSON（pipeline 出力 + findings[]） |

## 2. 検出パイプライン（UGD 継承）

正本: `USB-GuardDuty/backend/pipeline.py`

```text
ドライブ / フォルダ root（read-only walk）
  → inventory（symlink 除外）
  → per file: AnalyzerContext(path, temp workspace, online=false)
  → 拡張子フィルタ + 9 analyzers + YARA 横断
  → findings[] + overall label（clean | suspicious | malicious）
```

### 2.1 アナライザ一覧

| ID | ファイル | 対象 |
|---|---|---|
| magic_header | `analyzers/magic_header.py` | 全ファイル（偽装マジック） |
| hidden_fs | `analyzers/hidden_fs.py` | 隠し属性、二重拡張子、autorun.inf |
| pe_analyze | `analyzers/pe_analyze.py` | PE（エントロピー、構造） |
| office_vba | `analyzers/office_vba.py` | Office マクロ |
| image_stego | `analyzers/image_stego.py` | 画像内埋め込み |
| pdf_struct | `analyzers/pdf_struct.py` | PDF 構造 / JS |
| script_analyze | `analyzers/script_analyze.py` | ps1, bat, lnk 等 |
| archive_walk | `analyzers/archive_walk.py` | zip, 7z（深度・サイズ cap） |
| yara_scan | `analyzers/yara_scan.py` | 全ファイル横断 |

プラグイン契約: `analyzers/base.py`（`plugins/loader.py` は U04 で pipeline に配線）

### 2.2 Magika との関係

- 現行 UGD は **拡張子**で `APPLIES` を切る → 偽装拡張子で analyzer 漏れリスク
- UNVEIL では Broker の ContentTypeProbe を analyzer 選択の **追加入力**とする（[15](15-magika-absorption.md)）
- 拡張子 vs Magika 不一致は Finding ではなく **警告バナー**（自動 malicious にしない）

## 3. 読み取り専用契約（必須）

| 規則 | 実装 |
|---|---|
| 対象 USB へ書込禁止 | pipeline は read-only walk。extract は host temp のみ |
| シンボリックリンク | inventory 除外 |
| パストラバーサル | scan root 外へ出ない（UGD `_resolve_in_scan`） |
| 生セクタ | `partition_raw.py`、`GENERIC_READ`、**管理者権限時のみ** |
| ホスト書込 | `%APPDATA%\USB-GuardDuty\` または UNVEIL workspace のみ |
| 検体実行 | 禁止（[07](07-security.md) TH-01） |

**ST-U01:** スキャン中に対象ボリュームへ write API が呼ばれないことを試験（ファイル system filter または audit）。

## 4. プロセス分離

正本: `backend/worker_pool.py`

| 項目 | 値 |
|---|---|
| モデル | `multiprocessing` spawn Pool |
| タイムアウト | 30s / file |
| maxtasksperchild | 100 |
| YARA | worker initializer でロード（pickle 不可） |

**U04 で解消する既知ギャップ（UGD）:**

- `ScanManager._run` が `workers=1` 固定 → manifest / settings の `workers: 4` を渡す
- `scan_hidden`, `disabled_plugins` 等 config を pipeline に接続
- VirusTotal API key（Settings UI のみ）— **実装しない**（NFR-U01）

## 5. YARA と `--online`

| 項目 | UNVEIL 既定 |
|---|---|
| ルール更新 | **OFF**（`--online` 相当なし） |
| 同梱 | EICAR + ローカル rules |
| 更新 API | Coordinator が `--online` 明示時のみ sidecar に許可（将来 Settings） |
| ソース | signature-base / reversinglabs / yara-forge（UGD `rules_updater.py`） |

オンライン時も **試料本体は送信しない**。ルール ZIP のみ取得。

## 6. UI 構成（UGD 画面 → UNVEIL）

| UGD 画面 | UNVEIL `/usb` |
|---|---|
| Dashboard | ドライブ一覧、挿入検知、履歴 diff、スキャン開始 |
| ScanView | SSE 進捗、停止 |
| Results | ファイルツリー、severity  filter、codetext プレビュー |
| Rules | YARA ソース toggle、ローカル upload（更新は offline 既定） |
| Reports | JSON / HTML / PDF ダウンロード |
| Settings | スキャン toggle、workers — **VT キー UI は載せない** |

共通シェル装備（[06](06-ui-ux.md) / [10-mockup-html.mdc](.cursor/rules/10-mockup-html.mdc)）: テーマ、フォント ±、サイドバー折畳み、📋 コピー。

## 7. ドライブ列挙

正本: `backend/drives.py`

- `GetDriveTypeW == DRIVE_REMOVABLE`
- 固定ディスクは **既定スキャン不可**（lab フォルダモードは明示同意 + 禁止 root チェック）
- VID/PID、composite HID+storage 警告（BadUSB **疑い** — 確定ではない）

## 8. スコアと UNVEIL 軸

UGD `overall`: clean / suspicious / malicious は **Finding 集約ラベル**。

| UGD | UNVEIL 写像 |
|---|---|
| overall.malicious | assessment=indicators_present（USB ドメイン） |
| overall.clean | assessment=no_indicators（**安全証明ではない**） |
| findings[] | Finding + Evidence |
| coverage | 走査ファイル数 / スキップ / 上限到達 |

1 本の危険度％に畳まない（[10](10-unified-workbench.md) ALG-01）。

## 9. モジュール間 handoff

Results から「マルウェア解析へ」:

- 選択ファイルの Artifact を Broker がスナップショット化（USB 上のパスを UI に残さない）
- [13-malware-module.md](13-malware-module.md) の MalCheck intake へ投入
- AT-U08 / U08: 同一 SHA-256

## 10. 試験（U04 / U07）

| ID | 内容 |
|---|---|
| AT-U07 | リムーバブル限定、非リムーバブル拒否 |
| U07 usb readonly | スキャン中 write なし |
| UGD 回帰 | `tests/check_all.py` 22 PASS 相当を sidecar smoke に縮約 |
| fixture | EICAR + 合成ファイルのみ（`tests/conftest.py`） |

## 11. 関連

- [11-module-architecture.md](11-module-architecture.md)
- UGD `docs/design/01-architecture.md`, `05-threat-model.md`
- [15-magika-absorption.md](15-magika-absorption.md)
