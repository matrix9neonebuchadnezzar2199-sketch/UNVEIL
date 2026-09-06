---
article_id: wiki.enc.base64.ja
title: Base64 — 表現変換と難読化の境界
locale: ja-JP
revision: 1
category: encoding
technique_ids: [enc.base64, enc.base64url]
aliases: [ベース64, base64]
difficulty: beginner
engine_support: decode_supported
citations: [S3, S4]
---

## 30秒概要と何ではないか
3 bytes を 4 文字へ写す符号化。見た目は変わるが秘密性はない。

## 歴史
MIME での利用（RFC 2045）から RFC 4648（2006）で共通仕様へ。2006 は発明年ではない。

## 原理
alphabet / padding / strict decode / 再 encode。URL-safe は別プロファイル。

## 無害な前後例
Hello, UNVEIL! → SGVsbG8sIFVOVkVJTCE=

## 検知の根拠と誤検知
16 文字未満の汎用一致は候補にしない。JWT や ID も成立し得る。両立する場合は confirmed を避ける。

## UNVEIL 現版の操作
選択範囲をプレビュー→承認→別 Artifact へデコード。原本は書き換えない。

## 検証と不可逆情報
中身が暗号文ならデコード後も読めない。round_trip は意図の復元ではない。

## 学習問題
Q: デコード成功は悪意の証拠か？
A: いいえ。

## 出典・ライセンス・改訂
citations: [S3, S4]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
