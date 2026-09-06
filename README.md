# UNVEIL

[![status](https://img.shields.io/badge/status-mvp-blue)](docs/01-phase-plan.md)
[![phase](https://img.shields.io/badge/PHASE-06%20MVP-informational)](docs/01-phase-plan.md)
[![stack](https://img.shields.io/badge/stack-Tauri%202%20%7C%20Rust%20%7C%20React-62E5F2)](docs/02-architecture.md)
[![platform](https://img.shields.io/badge/isolation-Windows%20AppContainer%2BJob-62E5F2)](docs/07-security.md)
[![license](https://img.shields.io/badge/license-undecided-lightgrey)](docs/09-decisions-operations.md)
[![repo](https://img.shields.io/badge/github-UNVEIL-24292f)](https://github.com/matrix9neonebuchadnezzar2199-sketch/UNVEIL)

研究・検証用のローカル GUI。ファイルの難読化兆候を検知し、方式の特定と安全な解除・可読化を支援する。同じ画面で歴史と分類の WIKI を参照できる。

**入力は実行しない。** 隔離できない環境では解析を開始しない。未検出は安全の証明ではない。

## 読者別導線

| 読者 | 最初に読む |
|------|------------|
| 利用者・レビュー | [docs/00-product-requirements.md](docs/00-product-requirements.md) |
| 実装 | 本 README のインストール → [docs/02-architecture.md](docs/02-architecture.md) |
| 解析エンジン | [docs/03-analysis-pipeline.md](docs/03-analysis-pipeline.md) |
| セキュリティ | [docs/07-security.md](docs/07-security.md) |
| 設計入口 | [docs/README.md](docs/README.md) |

## 目次

- [現状](#現状)
- [動作環境](#動作環境)
- [インストール](#インストール)
- [使用例](#使用例)
- [終了](#終了)
- [トラブルシューティング](#トラブルシューティング)
- [セキュリティ境界](#セキュリティ境界)
- [ライセンス](#ライセンス)
- [GitHub Topics](#github-topics)

## 現状

PHASE 00–06 の MVP を実装済み。検証入口はリポジトリ直下の `python TEST.py`。PHASE 07（packer unpack 等）は別ロードマップのため未実装。

| 部品 | 状態 |
|------|------|
| 設計書 | `docs/00`–`09` |
| 隔離 | Windows AppContainer + Job Object。自己診断不合格なら解析開始不可 |
| 取込 | 選択トークン → スナップショット + SHA-256。原本非破壊 |
| 検知 | Base64/16/percent/JS escape、gzip、minify、リテラル連結、eval 構文 |
| 変換 | Plan → Preview → Approve → Apply。検証失敗は推奨出力にしない |
| WIKI | オフライン 14 記事 + 年表 + 用語 |
| GUI | Dashboard / Analyze（WIKI・変換・レポートを Analyze 内） |
| PHASE 07 | 未実装（PE/ELF はマジックのみ） |

提案スタック（ADR-002、未承認）: Tauri 2 + React/TypeScript + Rust coordinator/worker + SQLite。

## 動作環境

- OS: 開発は Windows。使用は隔離された検証環境の Windows（OQ-01）。解析は AppContainer+Job の自己診断が通る環境だけ
- Rust: stable（`rust-toolchain.toml`）
- Node.js: 22+
- WebView2（Windows）

## インストール

1. リポジトリを取得する。

```powershell
git clone https://github.com/matrix9neonebuchadnezzar2199-sketch/UNVEIL.git
cd UNVEIL
```

2. 全機能の導通確認をする。

```powershell
python TEST.py
```

3. デスクトップシェルの依存を入れる。

```powershell
cd apps/desktop
npm install
```

4. 開発起動する。

```powershell
npm run tauri dev
```

初回は Tauri のコンパイルで数分かかることがある。

## 使用例

### 最小例

```powershell
cd apps/desktop
npm run tauri dev
```

起動後、ヘッダの隔離バッジが OK なら Analyze で教材取込→静的解析ができる。自己診断が落ちているときは解析ボタンは無効（fail-closed）。

### ヘッドレス smoke

```powershell
cargo build -p unveil-analysis-worker -p unveil-coordinator --bins
$env:UNVEIL_WORKER = "$PWD\target\debug\unveil-worker.exe"
python TEST.py
# または
.\target\debug\unveil-ctl.exe diagnose
.\target\debug\unveil-ctl.exe smoke
```

ワーカーは `--isolated` なしでは終了コード 1。Coordinator 経由でのみ起動する。

### チェック一式

```powershell
.\scripts\check.ps1
```

## 終了

- 開発ウィンドウを閉じる、または端末で `Ctrl+C`
- ブラウザタブを閉じても解析ワーカーは残さない（Job Object の KILL_ON_JOB_CLOSE）
- 残留確認: `Get-Process unveil*` が空であること

## トラブルシューティング

| 症状 | 原因 | 対処 |
|------|------|------|
| `SANDBOX_UNAVAILABLE` | AppContainer/Job 自己診断失敗、または worker 未ビルド | `cargo build -p unveil-analysis-worker`。失敗時は解析しない |
| `unveil-worker` がすぐ終了 | `--isolated` なしで直接起動した | Coordinator / `unveil-ctl` 経由にする |
| `cargo test` が unveil をビルドしようとする | workspace に Tauri クレートが含まれる | `--exclude unveil` を使う（`scripts/check.ps1`） |
| `npm run tauri dev` が EACCES | Windows excludedportrange（1420 は 1339–1438 に含まれる） | 開発ポートは `14250`（`vite.config.ts` / `tauri.conf.json`） |
| アイコン警告 | `src-tauri/icons` 未生成 | `python tools/generate_icons.py` |

## セキュリティ境界

- 試料を実行しない。動的解析は MVP に含めない
- UI は許可した型付き IPC のみ。任意パス・シェル・外部 URL オープンは持たない
- 原本は書き換えない。成果物は派生物
- 実マルウェア検体を `tests/fixtures` や git に置かない

## ライセンス

未定（OQ-06）。コードと教材は別管理予定。配布前にリポジトリ所有者が決める。

## GitHub Topics

`static-analysis` `deobfuscation` `research` `tauri` `rust` `offline` `malware-research`
