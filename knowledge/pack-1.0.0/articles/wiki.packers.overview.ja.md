---
article_id: wiki.packers.overview.ja
title: 実行ファイルパッカー概観
locale: ja-JP
revision: 1
category: packers
technique_ids: []
aliases: [packer, UPX]
difficulty: intermediate
engine_support: wiki_only
citations: [S5]
---

## 30秒概要と何ではないか
実行ファイルを包む圧縮/保護。MVP はマジックのみ。内部 unpack は PHASE 07。

## 歴史
UPX 等は圧縮用途でも使われる。登場年は公式一次資料で確認する。

## 原理
成功と実行可能性は別検証。静的 unpack は版限定。

## 無害な前後例
図解のみ。検体を実行しない。

## 検知の根拠と誤検知
MZ/ELF は unsupported coverage。陽性にしない。

## UNVEIL 現版の操作
基本メタデータと記事案内。

## 検証と不可逆情報
動的解析は別製品境界。

## 学習問題
Q: UPX なら必ず戻せるか？
A: 版と形式に依存。MVP では戻さない。

## 出典・ライセンス・改訂
citations: [S5]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
