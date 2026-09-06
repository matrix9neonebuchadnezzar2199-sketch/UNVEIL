---
article_id: wiki.fundamentals.bytes.ja
title: bytes・文字コード・Hex
locale: ja-JP
revision: 1
category: fundamentals
technique_ids: []
aliases: [バイト, hex]
difficulty: beginner
engine_support: wiki_only
citations: [S3]
---

## 30秒概要と何ではないか
ファイルは bytes の列である。文字コードは表示属性であり、解析対象そのものではない。Hex は別表現。

## 歴史
符号化の文脈は MIME と RFC 4648 に整理されている。文字コードの発明年をここで断定しない。

## 原理
UTF-8 の strict 検査と BOM 付き UTF-16 だけを MVP の自動認識とする。不正 bytes を置換してから解析しない。

## 無害な前後例
同じ見た目の文字でも UTF-8 と UTF-16LE では bytes が異なる。

## 検知の根拠と誤検知
拡張子はヒント。マジックとパーサ結果を優先する。

## UNVEIL 現版の操作
原本スナップショットを Hex / text で表示。HTML として描画しない。

## 検証と不可逆情報
表示正規化と原本 bytes を混同しない。

## 学習問題
Q: Hex 表示は復号か？
A: いいえ。別の見た目の同じ bytes である。

## 出典・ライセンス・改訂
citations: [S3]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
