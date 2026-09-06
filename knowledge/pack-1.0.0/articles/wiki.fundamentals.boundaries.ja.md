---
article_id: wiki.fundamentals.boundaries.ja
title: 難読化・圧縮・暗号の境界
locale: ja-JP
revision: 1
category: fundamentals
technique_ids: []
aliases: [境界]
difficulty: beginner
engine_support: wiki_only
citations: [S1, S3]
---

## 30秒概要と何ではないか
難読化は理解を妨げる変換。圧縮は容量。暗号は秘密性。UNVEIL はこれらを混ぜて判定しない。

## 歴史
Collberg らの分類は layout/data/control 等。暗号理論の難読化（VBB）とは別系統。

## 原理
エンコード成功は機密性を意味しない。gzip 成功は無害を意味しない。

## 無害な前後例
Base64(Hello) は読める。AES 暗号文は鍵なしでは戻らない。

## 検知の根拠と誤検知
高エントロピーは圧縮・画像・暗号に共通。単独では陽性にしない。

## UNVEIL 現版の操作
assessment と identification と transformability を分けて表示する。

## 検証と不可逆情報
未検出は安全の証明ではない。

## 学習問題
Q: デコード成功は悪意の証拠か？
A: いいえ。形式の成立と目的は別。

## 出典・ライセンス・改訂
citations: [S1, S3]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
