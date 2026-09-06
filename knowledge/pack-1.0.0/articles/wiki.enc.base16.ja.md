---
article_id: wiki.enc.base16.ja
title: Base16 とハッシュ表記
locale: ja-JP
revision: 1
category: encoding
technique_ids: [enc.base16]
aliases: [hex, base16]
difficulty: beginner
engine_support: decode_supported
citations: [S3]
---

## 30秒概要と何ではないか
偶数長の hex 字種。ハッシュ指紋とペイロードを取り違えない。

## 歴史
RFC 4648 が Base16 を定義する。

## 原理
decode / reencode。大文字小文字はプロファイル。

## 無害な前後例
48656c6c6f → Hello

## 検知の根拠と誤検知
32/40/64 桁はハッシュの可能性を併記する。

## UNVEIL 現版の操作
選択範囲のデコード。ファイル全体を勝手にバイナリ置換しない。

## 検証と不可逆情報
ハッシュ値のデコードは別の意味を持たない。

## 学習問題
Q: SHA-256 hex は隠されたメッセージか？
A: 通常は指紋であり、デコード対象ではない。

## 出典・ライセンス・改訂
citations: [S3]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
