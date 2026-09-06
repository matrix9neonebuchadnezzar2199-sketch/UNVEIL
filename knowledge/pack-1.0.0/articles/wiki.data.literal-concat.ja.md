---
article_id: wiki.data.literal-concat.ja
title: リテラル分割と連結
locale: ja-JP
revision: 1
category: data
technique_ids: [data.literal-concat]
aliases: [連結]
difficulty: intermediate
engine_support: rewrite_supported
citations: [S1]
---

## 30秒概要と何ではないか
"He" + "llo" のような純粋連結だけを畳み込む。

## 歴史
文字列隠蔽は古い手法。製品名は特定しない。

## 原理
隣接する文字列リテラルと + のみ。変数・呼出し・タグ付きテンプレートは除外。

## 無害な前後例
const a = "He" + "llo"; → const a = "Hello";

## 検知の根拠と誤検知
AST 相当の走査。副作用のある式は対象外。

## UNVEIL 現版の操作
承認後に派生物を作る。directive prologue 位置は拒否方針。

## 検証と不可逆情報
行番号や toString 観測は一致しないことがある。

## 学習問題
Q: x + "a" は畳み込むか？
A: しない。変数参照は対象外。

## 出典・ライセンス・改訂
citations: [S1]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
