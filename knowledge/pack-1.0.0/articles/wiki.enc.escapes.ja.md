---
article_id: wiki.enc.escapes.ja
title: percent / JS escape
locale: ja-JP
revision: 1
category: encoding
technique_ids: [enc.percent, enc.js-escape]
aliases: [percent, \x]
difficulty: beginner
engine_support: decode_supported
citations: [S3]
---

## 30秒概要と何ではないか
%HH と JS 文字列内の \x / \u。文脈を外して当てない。

## 歴史
URL と言語リテラルの escape は別系統。発明年は断定しない。

## 原理
percent はバイト列。+ を空白へ変えない。JS escape はパーサ文脈が必要。

## 無害な前後例
%48%65%6C%6C%6F / "\x48i"

## 検知の根拠と誤検知
短い %HH 一つでは候補にしない。JS 以外の本文に JS 規則を適用しない。

## UNVEIL 現版の操作
範囲デコード。二重デコードは別承認。

## 検証と不可逆情報
HTML 実体や他言語 escape は MVP 対象外。

## 学習問題
Q: + は空白か？
A: UNVEIL の percent デコーダは + を空白にしない。

## 出典・ライセンス・改訂
citations: [S3]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
