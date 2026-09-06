import json
from pathlib import Path

root = Path(r"H:\CURSOR\UNVEIL\knowledge\pack-1.0.0")
articles = root / "articles"

SECTIONS = """
## 30秒概要と何ではないか
{summary}

## 歴史
{history}

## 原理
{principle}

## 無害な前後例
{example}

## 検知の根拠と誤検知
{detect}

## UNVEIL 現版の操作
{ops}

## 検証と不可逆情報
{limits}

## 学習問題
Q: {q}
A: {a}

## 出典・ライセンス・改訂
citations: {cites}
license: research notes; quoted works remain with original authors.
revision: 1 / reviewed_at: 2026-09-06
"""

items = [
    dict(id="wiki.fundamentals.bytes.ja", title="bytes・文字コード・Hex", cat="fundamentals", tech=[], aliases=["バイト", "hex"], diff="beginner", eng="wiki_only", cites=["S3"],
         summary="ファイルは bytes の列である。文字コードは表示属性であり、解析対象そのものではない。Hex は別表現。",
         history="符号化の文脈は MIME と RFC 4648 に整理されている。文字コードの発明年をここで断定しない。",
         principle="UTF-8 の strict 検査と BOM 付き UTF-16 だけを MVP の自動認識とする。不正 bytes を置換してから解析しない。",
         example="同じ見た目の文字でも UTF-8 と UTF-16LE では bytes が異なる。",
         detect="拡張子はヒント。マジックとパーサ結果を優先する。",
         ops="原本スナップショットを Hex / text で表示。HTML として描画しない。",
         limits="表示正規化と原本 bytes を混同しない。",
         q="Hex 表示は復号か？", a="いいえ。別の見た目の同じ bytes である。"),
    dict(id="wiki.fundamentals.boundaries.ja", title="難読化・圧縮・暗号の境界", cat="fundamentals", tech=[], aliases=["境界"], diff="beginner", eng="wiki_only", cites=["S1","S3"],
         summary="難読化は理解を妨げる変換。圧縮は容量。暗号は秘密性。UNVEIL はこれらを混ぜて判定しない。",
         history="Collberg らの分類は layout/data/control 等。暗号理論の難読化（VBB）とは別系統。",
         principle="エンコード成功は機密性を意味しない。gzip 成功は無害を意味しない。",
         example="Base64(Hello) は読める。AES 暗号文は鍵なしでは戻らない。",
         detect="高エントロピーは圧縮・画像・暗号に共通。単独では陽性にしない。",
         ops="assessment と identification と transformability を分けて表示する。",
         limits="未検出は安全の証明ではない。",
         q="デコード成功は悪意の証拠か？", a="いいえ。形式の成立と目的は別。"),
    dict(id="wiki.history.taxonomy.ja", title="難読化史と分類", cat="history", tech=[], aliases=["taxonomy","年表"], diff="beginner", eng="wiki_only", cites=["S1","S2"],
         summary="1997 年の分類報告と 2001 年の理論的限界は、製品の「全部戻せる」期待を戒める。",
         history="1997: Collberg et al. Technical Report 148。2001: Barak et al. CRYPTO。2010 改訂稿がある。規格化年と発明年を混同しない。",
         principle="potency / resilience / cost は評価軸。万能解除の存在証明ではない。",
         example="年表の項目は出典付きのものだけをタイムラインへ載せる。",
         detect="記事の存在をエンジン対応済みと読まない。engine_support バッジを見る。",
         ops="WIKI 検索と Finding の technique_id から関連記事へ。",
         limits="未確認の初出年は断定しない。",
         q="VBB 不可能性は全難読化が無意味という意味か？", a="いいえ。実務の難読化と理論モデルは別。"),
    dict(id="wiki.enc.base64.ja", title="Base64 — 表現変換と難読化の境界", cat="encoding", tech=["enc.base64","enc.base64url"], aliases=["ベース64","base64"], diff="beginner", eng="decode_supported", cites=["S3","S4"],
         summary="3 bytes を 4 文字へ写す符号化。見た目は変わるが秘密性はない。",
         history="MIME での利用（RFC 2045）から RFC 4648（2006）で共通仕様へ。2006 は発明年ではない。",
         principle="alphabet / padding / strict decode / 再 encode。URL-safe は別プロファイル。",
         example="Hello, UNVEIL! → SGVsbG8sIFVOVkVJTCE=",
         detect="16 文字未満の汎用一致は候補にしない。JWT や ID も成立し得る。両立する場合は confirmed を避ける。",
         ops="選択範囲をプレビュー→承認→別 Artifact へデコード。原本は書き換えない。",
         limits="中身が暗号文ならデコード後も読めない。round_trip は意図の復元ではない。",
         q="デコード成功は悪意の証拠か？", a="いいえ。"),
    dict(id="wiki.enc.base16.ja", title="Base16 とハッシュ表記", cat="encoding", tech=["enc.base16"], aliases=["hex","base16"], diff="beginner", eng="decode_supported", cites=["S3"],
         summary="偶数長の hex 字種。ハッシュ指紋とペイロードを取り違えない。",
         history="RFC 4648 が Base16 を定義する。",
         principle="decode / reencode。大文字小文字はプロファイル。",
         example="48656c6c6f → Hello",
         detect="32/40/64 桁はハッシュの可能性を併記する。",
         ops="選択範囲のデコード。ファイル全体を勝手にバイナリ置換しない。",
         limits="ハッシュ値のデコードは別の意味を持たない。",
         q="SHA-256 hex は隠されたメッセージか？", a="通常は指紋であり、デコード対象ではない。"),
    dict(id="wiki.enc.escapes.ja", title="percent / JS escape", cat="encoding", tech=["enc.percent","enc.js-escape"], aliases=["percent","\\x"], diff="beginner", eng="decode_supported", cites=["S3"],
         summary="%HH と JS 文字列内の \\x / \\u。文脈を外して当てない。",
         history="URL と言語リテラルの escape は別系統。発明年は断定しない。",
         principle="percent はバイト列。+ を空白へ変えない。JS escape はパーサ文脈が必要。",
         example="%48%65%6C%6C%6F / \"\\x48i\"",
         detect="短い %HH 一つでは候補にしない。JS 以外の本文に JS 規則を適用しない。",
         ops="範囲デコード。二重デコードは別承認。",
         limits="HTML 実体や他言語 escape は MVP 対象外。",
         q="+ は空白か？", a="UNVEIL の percent デコーダは + を空白にしない。"),
    dict(id="wiki.layout.minify.ja", title="minify・整形・名前の情報損失", cat="layout", tech=["layout.minify"], aliases=["minify","整形"], diff="beginner", eng="pretty_supported", cites=["S1"],
         summary="空白と改行を減らした配布物。整形はできる。失われた識別子は戻らない。",
         history="配布最適化は難読化分類の layout に近いが、意図は容量と配信であることが多い。",
         principle="行長と空白比。通常のバンドルと同じ見た目。",
         example="function f(){return 1} → 整形後も名前 f の由来は分からない。",
         detect="通常ビルド成果物でも陽性になり得る。難読化兆候とは別軸。",
         ops="pretty-print のみ。実行しない。",
         limits="コメントと元の変数名は不可逆。意味等価は証明しない。",
         q="整形成功は元ソース復元か？", a="いいえ。"),
    dict(id="wiki.data.literal-concat.ja", title="リテラル分割と連結", cat="data", tech=["data.literal-concat"], aliases=["連結"], diff="intermediate", eng="rewrite_supported", cites=["S1"],
         summary="\"He\" + \"llo\" のような純粋連結だけを畳み込む。",
         history="文字列隠蔽は古い手法。製品名は特定しない。",
         principle="隣接する文字列リテラルと + のみ。変数・呼出し・タグ付きテンプレートは除外。",
         example="const a = \"He\" + \"llo\"; → const a = \"Hello\";",
         detect="AST 相当の走査。副作用のある式は対象外。",
         ops="承認後に派生物を作る。directive prologue 位置は拒否方針。",
         limits="行番号や toString 観測は一致しないことがある。",
         q="x + \"a\" は畳み込むか？", a="しない。変数参照は対象外。"),
    dict(id="wiki.data.string-table.ja", title="文字列テーブルの原理", cat="data", tech=["data.string-table"], aliases=["string table"], diff="intermediate", eng="wiki_only", cites=["S1"],
         summary="配列や復元関数に文字列を置く手法。MVP は兆候案内のみ。自動復元は PHASE 07。",
         history="データ難読化の典型。製品の特定はしない。",
         principle="参照と復号関数に似た構造。一般解はない。",
         example="概念図のみ。実行しない。",
         detect="MVP は一般的構造の記事案内。confirmed にしない。",
         ops="WIKI のみ。エンジンは未対応バッジ。",
         limits="自動復元しない。",
         q="テーブルがあれば戻せるか？", a="静的に解決できる部分だけが対象。MVP では戻さない。"),
    dict(id="wiki.compression.gzip.ja", title="gzip 圧縮と展開制限", cat="compression", tech=["compression.gzip"], aliases=["gzip"], diff="beginner", eng="inflate_supported", cites=["S3"],
         summary="マジック 1f 8b。圧縮は難読化ではない。爆弾は上限で止める。",
         history="gzip は流通形式。登場年を難読化史に混ぜない。",
         principle="単一ストリーム、展開 50MiB、比 100 倍。ISIZE は信用しない。",
         example="gzip(Hello, UNVEIL!) を上限内で展開する。",
         detect="CRC 成功でも悪意は分からない。多 member は MVP 対象外。",
         ops="仮想出力。ヘッダのファイル名へ書き出さない。",
         limits="複合アーカイブは対象外。",
         q="展開できたファイルは安全か？", a="分からない。安全宣言をしない。"),
    dict(id="wiki.packers.overview.ja", title="実行ファイルパッカー概観", cat="packers", tech=[], aliases=["packer","UPX"], diff="intermediate", eng="wiki_only", cites=["S5"],
         summary="実行ファイルを包む圧縮/保護。MVP はマジックのみ。内部 unpack は PHASE 07。",
         history="UPX 等は圧縮用途でも使われる。登場年は公式一次資料で確認する。",
         principle="成功と実行可能性は別検証。静的 unpack は版限定。",
         example="図解のみ。検体を実行しない。",
         detect="MZ/ELF は unsupported coverage。陽性にしない。",
         ops="基本メタデータと記事案内。",
         limits="動的解析は別製品境界。",
         q="UPX なら必ず戻せるか？", a="版と形式に依存。MVP では戻さない。"),
    dict(id="wiki.control.flattening.ja", title="制御フロー平坦化", cat="control", tech=[], aliases=["flattening"], diff="advanced", eng="wiki_only", cites=["S1"],
         summary="ディスパッチャで制御を平坦化する。静的な一般解除は困難。",
         history="control obfuscation の代表。特定製品名で断定しない。",
         principle="実行条件の解析が必要。MVP は概念のみ。",
         example="無害な CFG 概念図。実行しない。",
         detect="エンジン未対応。",
         ops="WIKI のみ。",
         limits="意味の完全復元を約束しない。",
         q="平坦化は常に悪意か？", a="知財保護や研究用途もある。意図は別問題。"),
    dict(id="wiki.vm.overview.ja", title="仮想化と静的解析の限界", cat="vm", tech=["control.dynamic-eval"], aliases=["VM","eval"], diff="advanced", eng="detect_syntax_only", cites=["S2"],
         summary="独自 VM や動的生成は静的一般解除が難しい。eval 構文は実行しない。",
         history="実行モデルの難読化は段階的に複雑化した。年を雑に「近年」としない。",
         principle="eval( の存在は実行証明ではない。",
         example="記事の構文例のみ。入力を評価器へ渡さない。",
         detect="control.dynamic-eval は possible。変換は unsupported。",
         ops="検知して記事へ。評価しない。",
         limits="PHASE 07 でも動的解析は別審査。",
         q="eval があれば悪性か？", a="構文の存在だけで悪性とは言えない。"),
    dict(id="wiki.theory.limits.ja", title="VBB の限界と製品の限界", cat="theory", tech=[], aliases=["VBB","Barak"], diff="advanced", eng="wiki_only", cites=["S2"],
         summary="仮想ブラックボックス型の汎用難読化には理論的限界がある。製品が無意味、または全て解除可能という意味ではない。",
         history="CRYPTO 2001 初版と 2010-07-18 改訂稿を区別する。",
         principle="理論モデルと実務ツールの対応範囲は別。",
         example="誤解を見分ける：全部戻せる／全部無意味、の両極端を避ける。",
         detect="記事はエンジン精度の代わりにならない。",
         ops="学習問題と出典。",
         limits="証明全体を本製品が検証したとはしない。",
         q="理論的限界は UNVEIL が解けない理由の全部か？", a="一部。安全境界と対象外方式の方が実務上大きい。"),
]

for it in items:
    tech = "[" + ", ".join(it["tech"]) + "]"
    aliases = "[" + ", ".join(it["aliases"]) + "]"
    cites = "[" + ", ".join(it["cites"]) + "]"
    body = SECTIONS.format(
        summary=it["summary"], history=it["history"], principle=it["principle"],
        example=it["example"], detect=it["detect"], ops=it["ops"], limits=it["limits"],
        q=it["q"], a=it["a"], cites=cites,
    )
    fm = f"""---
article_id: {it['id']}
title: {it['title']}
locale: ja-JP
revision: 1
category: {it['cat']}
technique_ids: {tech}
aliases: {aliases}
difficulty: {it['diff']}
engine_support: {it['eng']}
citations: {cites}
---
"""
    (articles / f"{it['id']}.md").write_text(fm + body, encoding="utf-8")

timeline = [
    {"year": "1996", "title": "MIME Content-Transfer-Encoding", "note": "Base64 の使用文脈。発明年ではない。", "citation": "S4"},
    {"year": "1997", "title": "Obfuscating Transformations taxonomy", "note": "layout/data/control 等の評価軸。", "citation": "S1"},
    {"year": "2001", "title": "On the (Im)possibility of Obfuscating Programs", "note": "VBB 型汎用難読化の限界。全て無意味という結論ではない。", "citation": "S2"},
    {"year": "2006", "title": "RFC 4648", "note": "Base16/32/64 の共通仕様。暗号との違い。", "citation": "S3"},
    {"year": "2010", "title": "S2 改訂稿", "note": "2001 初版と 2010-07-18 版を区別する。", "citation": "S2"},
]
glossary = [
    {"term": "難読化", "definition": "理解を困難にする表現変換。悪意そのものではない。"},
    {"term": "エンコード", "definition": "Base64 等の表現変換。秘密性を保証しない。"},
    {"term": "検知", "definition": "兆候を見つけること。安全判定ではない。"},
    {"term": "confirmed", "definition": "形式検査と整合性に合格した方式候補。意図の確定ではない。"},
    {"term": "round_trip", "definition": "逆変換で入力範囲に一致すること。作者意図の復元ではない。"},
    {"term": "fail-closed", "definition": "隔離できないときは解析を開始しない。"},
]
(root / "timeline.json").write_text(json.dumps(timeline, ensure_ascii=False, indent=2), encoding="utf-8")
(root / "glossary.json").write_text(json.dumps(glossary, ensure_ascii=False, indent=2), encoding="utf-8")
(root / "manifest.json").write_text(json.dumps({"pack": "1.0.0", "articles": len(items), "locales": ["ja-JP"]}, indent=2), encoding="utf-8")
(root / "lessons" / "hello-base64.md").write_text("# lesson\n\n`const token = \"c3BlY2ltZW4ubGFi\";`\n", encoding="utf-8")
print("wrote", len(items), "articles")
