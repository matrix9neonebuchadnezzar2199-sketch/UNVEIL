# 15 — Magika 吸収計画

- 設計版: 1.0.0 / 作成日: 2026-09-08
- 状態: UPGRADE 設計案（U03）
- 参照: [google/magika](https://github.com/google/magika)（Apache-2.0）
- 論文: [Magika: AI-Powered Content-Type Detection](https://arxiv.org/html/2409.13768)（3×512 bytes → ONNX、~200+ content types）

## 1. 吸収判断サマリ（ALG-13）

| 判定 | 内容 |
|---|---|
| **採用** | コンテンツタイプ Probe、scanner ルーティング、拡張子偽装の三面表示 |
| **採用（コード）** | 公式 Python または Rust バインディング + 同梱 ONNX モデル |
| **不採用** | 悪意スコア、DIE/capa/YARA/Ghidra 置換、VT/abuse.ch、モデル再学習 |

Magika は Gmail/Drive/Safe Browsing で **適切なスキャナへルーティング**するために使われている。UNVEIL でも同じ役割: **どの analyzer / Ghidra / 難読化 detector に回すか**。

## 2. Magika の動作（吸収するノウハウ）

出典: [How Magika Works](https://securityresearch.google/magika/core-concepts/how-it-works/)、[Prediction Modes](https://securityresearch.google/magika/core-concepts/prediction-modes/)

1. ファイル先頭・中央・末尾から固定サイズ chunk を読む（全文読込しない → 大ファイルでも一定時間）
2. 深層モデルが content type と confidence score を返す
3. **per-content-type threshold** で Accept / 汎用ラベルへ降格
4. 二ラベル: `dl`（モデル生）と `output`（ツール最終）。低信頼時は `txt` / `unknown` 等

### 2.1 PredictionMode（UNVEIL での使い分け）

| モード | UNVEIL 用途 |
|---|---|
| `high-confidence` | **ルーティング**（Ghidra 投入、PE analyzer 選択） |
| `medium-confidence` | UI ヒント（自動実行には使わない） |
| `best-guess` | アナリスト向け debug のみ（ルーティング禁止） |

### 2.2 UNVEIL 既存 Probe との役割分担

| 機構 | 役割 |
|---|---|
| UNVEIL 64KiB×3 分布（[03](03-analysis-pipeline.md)） | 難読化 **特徴**（エントロピー等） |
| libmagic（MalCheck surface） | 従来 file_type ヒント |
| Magika 3×512 | **コンテンツタイプ分類**（テキスト種別に強い） |

重複実装しない。Magika は **type**、entropy は **obfuscation feature**。

## 3. ContentTypeProbe 契約

Broker が Import 後に生成（[11](11-module-architecture.md) §5.2）。

```json
{
  "declared_extension": "txt",
  "magic_hint": "ASCII text",
  "magika": {
    "status": "ok",
    "dl_label": "pebin",
    "output_label": "pebin",
    "score": 0.997,
    "prediction_mode": "high-confidence",
    "mime_type": "application/x-dosexec",
    "is_text": false
  },
  "mismatch_flags": ["extension_vs_magika", "magic_vs_magika"]
}
```

| status | 意味 |
|---|---|
| ok | モデル実行成功 |
| unavailable | C_magika 不合格（fallback 禁止） |
| empty | 空ファイル |
| too_small | モデル未実行ヒューリスティック |

**UI 三面表示（FR-U04）:**

| ソース | 表示 |
|---|---|
| 宣言拡張子 | `.txt` |
| magic_hint | libmagic / file(1) |
| Magika output | `pebin` + score + mode |

不一致 → **注意バナー**（「自動的に malicious ではない」文言必須）。

## 4. ルーティング表（high-confidence 時）

`output_label` → 次工程（暫定。実装時 fixture で校正）

| Magika label（例） | マルウェア表層 | Ghidra static | USB analyzers | 難読化 engine |
|---|---|---|---|---|
| pebin / exe | PE scanners + capa | **可** | pe_analyze | PE magic only（MVP） |
| elf / macho | 部分 | 将来 | pe_analyze 相当 | unsupported |
| javascript | FLOSS/capa if avail | **不可** | script_analyze | JS detectors |
| python | format scanners | **不可** | script_analyze | text features |
| powershell | format scanners | **不可** | script_analyze | text features |
| pdf | pdfid | **不可** | pdf_struct | unsupported |
| zip / gzip | archive | **不可** | archive_walk | gzip detector |
| empty / unknown / generic txt | 限定表層 | **不可** | magic_header, yara | text probe |

**Gate U05:** javascript ラベルファイルを Ghidra に送らない（プローブ U05）。

低信頼（threshold 未満で generic 化）→ Ghidra **不可**、capa **慎重**（UI に degraded）。

## 5. 採用しないもの

| 項目 | 理由 |
|---|---|
| score → verdict | ALG-01 違反 |
| DIE 置換 | packer **製品**特定は capa/DIE 領域。Magika は type |
| capa / YARA 置換 | 能力レイヤが異なる |
| オンライン Magika API | NFR-U01 |
| カスタム再学習モデル | 再現性・SBOM 複雑化 |
| VT / abuse.ch 連携 | 試料送信 |

## 6. 実装方針（U03）

| 項目 | 提案 |
|---|---|
| 配置 | Coordinator 配下 `magika-probe` worker（sidecar または同一 Broker 内 Python 子プロセス） |
| 依存 | `pip install magika` 同梱 venv **または** Rust crate（版は SBOM 固定） |
| モデル | リポ同梱。起動時ネットワーク取得 **禁止**（OQ-U02） |
| ライセンス | Apache-2.0。NOTICE に Magika 記載（G-U07） |
| 失敗 | `C_magika=false` → libmagic へ **黙って**切替えない |

### 6.1 MalCheck 統合点

- `scripts/remnux/analyze.py` の `file_type` に Magika `output_label` を **追加フィールド** `content_type_magika` として sidecar が注入可能
- Ghidra gate: `is_analyzable_binary()` **AND** Magika pe 系 label

### 6.2 USB-GuardDuty 統合点

- `pipeline.APPLIES` を拡張子のみから **`applies(content_probe)`** へ
- 例: Magika `javascript` → script_analyze を拡張子 `.txt` でも実行候補に

### 6.3 難読化モジュール

- Probe 結果を Analyze 取込確認画面に表示
- JS / text 系 label で detector セットを絞る（過検知削減）

## 7. 禁止表示（UI 文案）

- ❌ 「Magika スコア 99% — 危険」
- ❌ 「AI がマルウェアと判定」
- ✅ 「コンテンツタイプ: Python source（信頼度 0.99, high-confidence）」
- ✅ 「拡張子 .txt と Magika 判定 pebin が不一致 — 偽装の可能性。追加検査を推奨（安全/危険の確定ではありません）」

## 8. 試験 fixture（U03 / U04 / U05）

| fixture | 期待 |
|---|---|
| `mislabel.txt` 中身 PE | mismatch_flags 3 件、注意表示、Ghidra は Magika pebin 時のみ |
| `hello.js` minified | Magika javascript、Ghidra 不可、JS detector 可 |
| empty file | status=empty、モデル未実行 |
| EICAR.txt | Magika text/plain 等、YARA EICAR hit（Magika は悪意と言わない） |

## 9. 関連 ADR

- **ADR-U002:** 公式 Magika バインディング同梱、再学習しない
- **ADR-U004:** Magika は DIE/capa/YARA/Ghidra と **相補**

## 10. 参考文献

- [Magika GitHub](https://github.com/google/magika)
- [How Magika Works](https://securityresearch.google/magika/core-concepts/how-it-works/)
- [Prediction Modes](https://securityresearch.google/magika/core-concepts/prediction-modes/)
- [Google Open Source Blog (2024)](https://opensource.googleblog.com/2024/02/magika-ai-powered-fast-and-efficient-file-type-identification.html)
