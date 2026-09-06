# 05 — WIKI・歴史・体系・学習コンテンツ

## 1. 学習設計

WIKIは「方式の一覧」だけではなく、**歴史→原理→見つけ方→戻し方→限界→追試**を一つの学習単位とする。難読化を悪意と同一視せず、配布容量削減・知財保護・研究・保守・セキュリティ検証の用途も説明する。

初学者には概要/用語/無害例を先に、熟練者には検知根拠/実装制約/出典をすぐ開ける構成を提供。記事はオフライン同梱、更新は版付きパック単位。Webから自動転載する仕組みは含めない。

## 2. 分類軸

| 主分類 | 下位例 | 可逆性/制限 | MVPとの関係 |
|---|---|---|---|
| 表現・符号化 | Base系、percent、escape | 規則既知なら可逆だが暗号ではない | 主要対象 |
| レイアウト | 空白除去、コメント除去、識別子変更 | 整形可能。失われた名前/コメントは戻らない | 整形支援 |
| データ | 文字列分割、テーブル、定数隠蔽 | 静的に解決できる部分だけ | リテラル連結のみ変換 |
| 制御フロー | opaque predicate、flattening | 実行条件・意味の解析が必要 | WIKIのみ |
| 包装 | 圧縮、実行ファイルpacker | 形式・版ごとに対応。圧縮だけで難読化ではない | gzipのみ展開 |
| 実行モデル | VM化、動的コード生成、自己書換え | 静的な一般解除は困難 | WIKIのみ |
| 解析妨害 | 環境依存、anti-debug等 | 環境で観測が変わる | 概念と限界のみ |
| 暗号との境界 | 暗号化文字列、鍵依存の復元 | 正しい鍵や条件が必要 | 学習のみ |

本分類はUNVEIL向けの実用分類。Collbergらの原分類（layout/data/control/preventive）をそのまま名乗らず、原分類との対応と製品上の追加軸を説明する。

補助軸: language、platform、era、reversibility、prerequisites、difficulty、engine_support、evidence_type。技法名と製品名を別エンティティとし、製品は複数技法を組み合わせ得る関係にする。

## 3. 歴史コンテンツの初期設計

以下は記事化のための年表シードであり、網羅的な難読化史ではない。「規格化年」「論文発表年」「製品登場年」「発明年」を区別する。未確認の初出年は断定しない。

| 年/年代 | 学ぶ事項 | 位置付け・注意 | 出典 |
|---|---|---|---|
| 1990年代以前からの背景 | 表現変換、低水準化、読みにくいコード | 背景章。特定の発明年は本文で裏付けを確認するまで掲載しない | S2のIntroduction/関連研究を出発点に調査 |
| 1996 | MIMEのContent-Transfer-Encoding | Base64の使用文脈。難読化技法の発明年ではない | S4 |
| 1997 | 難読化変換の分類と評価軸 | layout/data/control等を体系的に捉える | S1 |
| 2001 | Virtual Black Box型汎用難読化の限界 | すべての難読化が無意味、すべて解除可能という結論ではない | S2（会議初版） |
| 2006 | Base16/Base32/Base64の共通仕様 | 暗号との違い・canonical encoding・厳格検証 | S3 |
| 2010 | S2の改訂稿 | 2001初版と参照PDFの版を区別する練習 | S2（2010-07-18版） |
| 年代を固定しない発展章 | パッキング、文字列隠蔽、制御フロー、VM化 | 「近年発明」と誤記しない。製品/論文ごとに確認して年表へ追加 | S5、追加一次資料を調査 |

WIKI公開時の年表は日付付きの検証済み項目だけをタイムラインへ載せる。「年代を固定しない」行は概説章に置く。史料に当たっていない背景項目はreview_requiredとして公開ゲートから除外せず、確認完了まで公開を保留する。

## 4. 出典台帳（初期）

参照日: 2026-09-06。確認状態はこの設計作業でどこまで確認したかを示し、将来の教材レビュー完了を意味しない。

| ID | 資料・URL | 使用箇所 | 現在の確認状態 |
|---|---|---|---|
| S1 | Collberg, Thomborson, Low, *A Taxonomy of Obfuscating Transformations*, Technical Report 148, 1997. [大学リポジトリPDF](https://researchspace.auckland.ac.nz/bitstreams/990cb7de-2b44-4a24-acee-82b0efa8e689/download) | 分類、potency/resilience/cost等 | 検索結果で書誌/所在を確認。本文の引用位置・分類詳細は記事公開前に再確認 |
| S2 | Barak et al., *On the (Im)possibility of Obfuscating Programs*, CRYPTO 2001初版、2010-07-18改訂稿。[著者公開PDF](https://www.boazbarak.org/Papers/obfuscate.pdf) | 理論的限界の注意 | 冒頭・Abstract・Introduction・初版注記を確認。証明全体を検証したとはしない |
| S3 | Josefsson, RFC 4648, 2006-10。[RFC Editor](https://www.rfc-editor.org/rfc/rfc4648) | Base系、§3/4/5/8/10/12 | 本文・テストベクトル・security considerations確認 |
| S4 | Freed/Borenstein, RFC 2045, 1996-11。[RFC Editor](https://www.rfc-editor.org/rfc/rfc2045) | MIME/表現変換の歴史 | S3の参考文献で書誌確認。記事作成時に対象節を直接確認 |
| S5 | UPX公式サイト。[UPX](https://upx.github.io/) | 圧縮用途、unpack、ライセンス | 公式Introduction/Overview確認。登場年・全対応版の根拠には使用しない |

出典ごとに著者、タイトル、発行元、初版/参照版、URL、参照日、節/頁、ライセンス、検証者を保存する。リンク先が消えた場合は引用済み書誌を保持し、再配布可能な範囲で代替所在を記録する。論文PDFの丸ごと同梱は権利確認なしに行わない。

## 5. 記事スキーマと必須テンプレート

```yaml
article_id: wiki.enc.base64.ja
technique_ids: [enc.base64]
locale: ja-JP
revision: 1
status: draft
category: encoding
title: Base64 — 表現変換と難読化の境界
aliases: [ベース64, base64]
difficulty: beginner
prerequisites: [wiki.fundamentals.bytes.ja]
engine_support: decode_supported
citations: [S3]
reviewed_at: null
```

必須本文:

1. 30秒概要と「何ではないか」。
2. 歴史的背景と初版/版の区別、出典。
3. 原理・用語・関連分類、必要なら独自の説明図。
4. 無害なbefore/afterと具体的な入力/出力の意味。
5. 検知の根拠と限界、誤検知となる正常例。
6. UNVEILの現版でできる操作と、前提/対象外。
7. 解除・可読化の検証方法と不可逆な情報。
8. 学習問題・答え・次の記事。
9. 出典・引用・ライセンス・改訂履歴・レビュー状態。

教材のsample_id、期待ハッシュ、生成版、方式の重ね順もメタデータに持つ。engine_supportは記事執筆者の自由入力でなく、対応マニフェストとの整合性チェックで導出する。

## 6. 初期記事バックログ

| article_id（ja省略） | 記事テーマ | 深度 | 教材 |
|---|---|---|---|
| wiki.fundamentals.bytes | bytes/文字コード/Hex | 入門 | 同じ文字と異なるbytes |
| wiki.fundamentals.boundaries | 難読化・圧縮・暗号の違い | 入門 | 表現変換と機密性 |
| wiki.history.taxonomy | 難読化史と分類 | 入門〜中級 | 年表の出典比較 |
| wiki.enc.base64 | Base64/Base64url | 入門 | Hello, UNVEIL! |
| wiki.enc.base16 | Base16とハッシュ表記 | 入門 | 正常なhexとの区別 |
| wiki.enc.escapes | percent/JS escape | 入門 | 日本語・境界・二重デコード |
| wiki.layout.minify | minify・整形・名前の情報損失 | 入門 | 自作の短いJS |
| wiki.data.literal-concat | リテラル分割・連結 | 中級 | 純粋式と副作用の対比 |
| wiki.data.string-table | 文字列テーブルの原理 | 中級 | 表だけの概念教材 |
| wiki.compression.gzip | 圧縮と展開制限 | 入門 | 小さな正常gzip |
| wiki.packers.overview | 実行ファイルパッカー | 中級 | 図解のみ、実行なし |
| wiki.control.flattening | 制御フロー平坦化 | 中級〜上級 | 無害な概念CFG |
| wiki.vm.overview | 仮想化と静的解析の限界 | 上級 | 抽象図のみ |
| wiki.theory.limits | VBBの限界と製品の限界 | 上級 | 誤解を見分ける問題 |

12本以上の公開をG04の最低条件とし、MVP対応方式に必要な記事は全件必須。記事数のために内容の薄いページを量産しない。上記は執筆計画であり、現在これらの記事が実装済みという意味ではない。

## 7. 完成イメージ用の記事内容案: Base64

**概要:** 3 bytesを4文字へ表現する符号化。見た目を変えるが秘密性はない。[S3 §4, §12]

**歴史:** MIME等での利用から共通仕様へ。2006はRFC 4648の公開年であり、Base64の発明年ではない。[S3 §1, S4]

**無害例:** `Hello, UNVEIL!` → `SGVsbG8sIFVOVkVJTCE=`。

**見つけ方:** 字種、長さ、paddingを確認し、strict decode後に再encodeする。短い英数字列や通常のデータ転送文字列でも成立するため、これだけで難読化意図を断定しない。

**UNVEILでの操作:** 選択範囲をプレビュー→別Artifactへデコード→round_tripを確認。URL-safe版・無paddingは別プロファイルで扱う。

**限界:** 中身が暗号文やバイナリなら、デコードできても読めるコードにはならない。ファイルの安全性も判定できない。

**学習問題:** 「デコード成功は悪意の証拠か？」答え: いいえ。方式の形式成立と目的・挙動は別。

## 8. 検索と解析連携

- 検索対象: タイトル、別名、用語、本文、分類、言語、年代、対応状態。全文索引は同梱時に構築する。
- 日本語は形態素依存を減らした正規化＋n-gram索引を初期案とし、英数字の完全一致/前方一致を加える。クエリは256文字、ページ20件。
- 順位: technique_id完全一致→タイトル/別名→見出し→本文。出典文だけがヒットした場合は該当位置を表示する。
- Findingから方式IDで記事へ。複数候補は複数記事。記事未整備の場合は上位分類記事と「専用記事未掲載」を表示し、空ページにしない。
- 「教材で試す」は専用セッションを開く。分析中の試料を差し替えない。学習進捗はローカルのみ保存。
- 誤字訂正/誤検知の研究メモは、個人メモと公式記事を分離。MVPにサーバー上の共同編集はない。

## 9. 編集・公開フロー

`draft → fact_review → technical_review → published → deprecated`。歴史・理論は出典レビュー、解除手順は解析担当、安全教材はセキュリティレビューを必須化する。公開パックはschema/内部リンク/引用/教材期待値/対応状態をCI検証し署名する。引用と本文を区別し、訂正理由を履歴に残す。

記事レンダラーはraw HTML/iframe/scriptを無効化。画像は同梱の権利確認済み資産のみ。外部リンクはクリック時にドメインを表示し、OSブラウザで開く前に確認する。入力ファイルから抽出したURLをWIKI出典として自動訪問しない。
