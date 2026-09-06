---
article_id: wiki.compression.gzip.ja
title: gzip 圧縮と展開制限
locale: ja-JP
revision: 1
category: compression
technique_ids: [compression.gzip]
aliases: [gzip]
difficulty: beginner
engine_support: inflate_supported
citations: [S3]
---

## 30秒概要と何ではないか
マジック 1f 8b。圧縮は難読化ではない。爆弾は上限で止める。

## 歴史
gzip は流通形式。登場年を難読化史に混ぜない。

## 原理
単一ストリーム、展開 50MiB、比 100 倍。ISIZE は信用しない。

## 無害な前後例
gzip(Hello, UNVEIL!) を上限内で展開する。

## 検知の根拠と誤検知
CRC 成功でも悪意は分からない。多 member は MVP 対象外。

## UNVEIL 現版の操作
仮想出力。ヘッダのファイル名へ書き出さない。

## 検証と不可逆情報
複合アーカイブは対象外。

## 学習問題
Q: 展開できたファイルは安全か？
A: 分からない。安全宣言をしない。

## 出典・ライセンス・改訂
citations: [S3]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
