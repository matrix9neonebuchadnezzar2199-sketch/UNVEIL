---
article_id: wiki.vm.overview.ja
title: 仮想化と静的解析の限界
locale: ja-JP
revision: 1
category: vm
technique_ids: [control.dynamic-eval]
aliases: [VM, eval]
difficulty: advanced
engine_support: detect_syntax_only
citations: [S2]
---

## 30秒概要と何ではないか
独自 VM や動的生成は静的一般解除が難しい。eval 構文は実行しない。

## 歴史
実行モデルの難読化は段階的に複雑化した。年を雑に「近年」としない。

## 原理
eval( の存在は実行証明ではない。

## 無害な前後例
記事の構文例のみ。入力を評価器へ渡さない。

## 検知の根拠と誤検知
control.dynamic-eval は possible。変換は unsupported。

## UNVEIL 現版の操作
検知して記事へ。評価しない。

## 検証と不可逆情報
PHASE 07 でも動的解析は別審査。

## 学習問題
Q: eval があれば悪性か？
A: 構文の存在だけで悪性とは言えない。

## 出典・ライセンス・改訂
citations: [S2]
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
