# 04 — データモデル・状態・IPC契約

## 1. 基本規則

契約版は`schema_version: "1.0"`を提案する。本文は設計仕様であり、生成済み型定義/API実装ではない。実装時にJSON SchemaからRust/TypeScript型と互換性試験を生成する。

- ID: 推測困難なUUID（例の`art-demo`等は説明用）。時刻: UTC RFC3339。
- SHA-256: 小文字hex64文字。長さ/offset: 非負整数、現MVP上限内。
- byte range: `start`を含み`end`を含まない`[start,end)`。`0 <= start <= end <= byte_length`を検証する。
- 行/列: UIは1起点、バイト範囲は0起点。UTF-8 bytes、Unicode文字、UTF-16 code unitを混用しない。JSパーサの座標を元bytesへ変換して保存する。
- ワーカー出力はすべて非信頼。IDの所有Session、範囲、数、文字列長、enum、ハッシュをブローカーで検証する。

## 2. 主要エンティティ

| 型 | 必須属性/関係 | 重要な制約 |
|---|---|---|
| Session | id, mode, created_at, current_artifact_id, versions, settings | mode=ephemeral/persistent。保存は明示操作 |
| Artifact | id, session_id, sha256, byte_length, kind, blob_ref, created_by_step_id | 原本はstep=null。blob_refはブローカー内部のみ |
| Job | id, session_id, input_artifact_id, kind, status, limits, versions, last_seq | kind=import/analyze/preview/transform/export。importではinput_artifact_id=null可。終端後の再実行は新ID |
| Coverage | status, checked_ranges, checked_rules, skipped, inspected_bytes | rule別範囲を保持。重複バイトは合算しない |
| Feature | id, artifact_id, range, extractor_id, value, unit | 比率/entropy等に単位と窓を付ける |
| Finding | id, job_id, artifact_id, technique_id, category, range, identification, score, evidence_ids, transformability | scope付き。unknownを空文字で表現しない |
| Evidence | id, source_rule, range, observation, polarity, group, excerpt_ref | polarity=supports/contradicts/context。抜粋2 KiB以下 |
| AnalysisResult | job_id, coverage, assessment, finding_ids, limitations | completedでもinconclusiveがあり得る |
| TransformPlan | id, session_id, input_hash, range, adapters, parameters, limits, budget_ledger_id, preconditions, plan_hash, state | 承認はplan_hashに束縛 |
| Approval | id, plan_hash, policy_hash, approved_at, expires_at | 同じUIセッション内のみ、期限10分の暫定値 |
| TransformStep | id, plan_id, input_artifact_id, output_artifact_ids, adapter_version, parameters, result, mappings | 入出力関係に循環禁止 |
| Validation | step_id, checks, verdict, limitations, expected_hash | verdict=passed/failed/not_checked。教材以外expected_hash=null可 |
| Report | id, session_id, schema_version, versions, included_artifacts, redactions | 原本同梱/本文同梱は既定false |
| Article | article_id, technique_ids, locale, revision, title, citations, engine_support | 知識版とルール版を分ける |

SQLiteでは外部キーを有効化し、`(job_id,seq)`をイベント一意キー、`(session_id,sha256)`をblob参照の検索索引とする。削除時は参照関係を確認する。Artifact自体は上書きせず、選択状態や研究注記を別テーブルで更新する。

同じbytesに至る異なる経路では別Artifact IDを作り、blobだけを重複排除する。これによりcreated_by_step_idは一意で、経路の由来を失わない。Previewの検証はplan_idに属する未確定candidateとして保持し、Apply時にTransformStep/Validationを確定する。

範囲mappingは `source_range / output_range / mode`、modeはexact/approximate/unmapped。デコードやAST書換えで1対1対応が消えた場合、架空の正確な線を描かない。

## 3. ジョブ状態遷移

```text
queued ─→ running ─→ completed | partial | failed
   │          └─→ cancelling ─→ cancelled
   └─────────────────────────→ cancelled
```

- Job.stageは `import/probe/features/detect/plan/preview/transform/validate/export`。状態と工程を分ける。
- `partial`: 使える確定結果を保持したまま予算/未対応工程等で予定の一部を終えられない。`coverage.partial`と必ず同義ではなく、部分範囲を計画どおり調査したcompletedもあり得る。
- `cancelled`: 利用者による停止。既に確定済みの結果をpartial_resultsとして参照できるが、Job.statusをpartialへ上書きしない。
- `failed`: 入力不正、ワーカー異常、保存失敗等。unsupportedは検査範囲の値であり例外コードと混ぜない。
- 予算到達で有用な確定結果がない場合failed＋LIMIT_REACHED、有用な結果がある場合partial＋LIMIT_REACHED。
- 中止と完了が競合したらDBで最終状態をcompare-and-set。一度確定した終端状態が勝つ。遅れた完了イベントは破棄する。
- 再起動時に残ったrunning/cancellingはfailed＋INTERRUPTEDへ変更し、過去結果を保った新Jobで再実行する。

TransformPlanは `draft → previewing → awaiting_approval → approved → applying → applied`。どの非終端状態からもexpired/rejectedへ遷移可能。入力・版・制限変更時にinvalidatedとし、旧承認を拒否する。preview/applyの失敗はfailed、中止はcancelledとしてPlanを終端にし、再試行は新Planとする。適用中のexpiresは予約済み処理を取り消さず、Apply受理時に承認期限を検証する。ジョブは承認待ちでワーカーを占有しない。

## 4. IPCの共通エンベロープ

```json
{
  "schema_version": "1.0",
  "request_id": "req-demo",
  "command": "analysis.start",
  "params": {
    "session_id": "session-demo",
    "artifact_id": "art-demo",
    "profile": "static-safe-v1"
  }
}
```

応答は`{request_id, ok, data}`または`{request_id, ok:false, error:{code,message_key,retryable,details}}`。同期応答は受理結果だけ。長時間操作はjob_idを返しイベントで通知する。UI表示用文言はmessage_keyでローカライズし、ワーカーの生エラーにHTMLとして依存しない。

## 5. コマンド一覧

| コマンド | 入力 | 出力/責務 |
|---|---|---|
| session.create | mode | session_id。永続化は確認必須 |
| session.list / session.get | filter/cursor またはsession_id | セッション一覧/読取専用スナップショット |
| file.choose | 許可済みダイアログ設定 | opaque selection_token |
| file.import | session_id, selection_token | 取込job_id、完了後artifact_id |
| analysis.start | session_id, artifact_id, profile | job_id。版/制限をサーバー側確定 |
| job.cancel | job_id | accepted/current_status。冪等 |
| job.get | job_id | 最新状態/結果/last_seq |
| job.events | job_id, after_seq | 連番順イベント、継続cursor |
| artifact.read | artifact_id, start, max_bytes, view | エスケープ済み表示/bytes。サーバーでサイズ制限 |
| transform.plan | artifact_id, finding_idまたは明示範囲, adapter_id, params | plan_id, plan_hash, 前提・上限・警告 |
| transform.preview | plan_id | job_id、未確定候補と検証 |
| transform.approve | plan_id, plan_hash, acknowledged_warnings | approval_id、期限 |
| transform.apply | plan_id, approval_id, idempotency_key | job_id/既存job_id |
| comparison.get | left_artifact_id, right_artifact_id, cursor | 差分ページ、mapping精度 |
| wiki.search | query, filters, locale, cursor | 記事ID・見出し・安全な抜粋 |
| wiki.get | article_id, revision | 記事内容・出典・対応状態 |
| session.save | session_id, retention_choice | 保存結果 |
| session.delete | session_id, confirmation_token | 削除範囲/失敗理由 |
| export.choose | format | ネイティブ保存ダイアログでdestination_tokenを発行 |
| report.list | session_id, cursor | 書出し履歴/保存先のローカル情報 |
| report.export | session_id, format, redactions, destination_token | job_id。原本/既存入力への上書き拒否 |

MVPには任意シェル実行、任意URL取得、自由なパス読み書きのIPCを設けない。`idempotency_key`はsession/command/payload hashと組で保存し、同一キー異なる内容は拒否する。別SessionのID、期限切れ選択/保存トークンは拒否する。

## 6. イベントとバックプレッシャー

```json
{
  "schema_version": "1.0",
  "job_id": "job-demo",
  "seq": 12,
  "type": "stage.progress",
  "stage": "detect",
  "progress": {"completed_units": 4, "total_units": 9},
  "payload": {"artifact_id": "art-demo"}
}
```

順序はjob内だけ保証する。UIは`seq`で重複排除し、欠番時は再取得する。進捗の分母不明ならtotal_units=null、偽のパーセンテージを出さない。イベントは秒10件以下に集約し、Finding本文は参照IDだけ通知。再接続時はsnapshot＋そのlast_seq以降を取得し、取りこぼしと二重表示を避ける。

## 7. Findingの説明例（抜粋、完全なschemaではない）

```json
{
  "id": "finding-demo",
  "artifact_id": "art-demo",
  "technique_id": "enc.base64",
  "category": "encoding",
  "range": {"start": 0, "end": 20},
  "identification": "probable",
  "score": {"value": 80, "kind": "rule_match", "rule_version": "1.0.0"},
  "evidence_ids": ["strict-alphabet-demo", "round-trip-demo"],
  "transformability": "supported",
  "limitations": ["Base64url is also possible.", "Encoding does not establish obfuscation intent."],
  "article_id": "wiki.enc.base64.ja"
}
```

方式の証拠とファイル全体のassessmentは分離している。UI/レポートはこのFindingを「80%悪性」と言い換えてはならない。

## 8. エラーと回復

| code | 分類 | 利用者への案内 |
|---|---|---|
| INPUT_NOT_REGULAR / INPUT_CHANGED | 取込 | 通常ファイルを再選択 |
| INPUT_TOO_LARGE | 制限 | 最大サイズを明示、無制限化しない |
| INVALID_ENCODING / PARSE_FAILED | 内容 | 原本を保持、Hex/汎用表示へ |
| ADAPTER_UNSUPPORTED | 対応外操作 | 対応表/WIKIへ、別操作を選ぶ |
| LIMIT_REACHED | 資源 | 到達した上限、部分結果、狭い範囲で再試行 |
| SANDBOX_UNAVAILABLE | 安全 | 解析禁止、隔離設定案内、WIKIは利用可 |
| WORKER_CRASHED / INTERRUPTED | 実行基盤 | 確定結果を保持、手動再試行 |
| PLAN_STALE / APPROVAL_EXPIRED | 競合 | 再プレビュー/再承認 |
| VALIDATION_FAILED | 変換 | 推奨出力にしない、根拠を表示 |
| STORAGE_FULL / PERMISSION_DENIED | 保存 | 保存先/容量を確認、原本は不変 |
| SCHEMA_MISMATCH | 互換 | 読み取り専用/適合版案内 |

## 9. レポートと移行

レポートは入力ハッシュ、サイズ、取得日時、解析範囲、方式候補、根拠、限界、変換DAG、検証、設定、全エンジン/ルール/パーサ/WIKI版、停止理由を含む。個人パス、生の抜粋、原本・派生物は既定で除外。ハッシュ自体も機密情報になり得るためマスキング対象にする。

完全再現モードは原本と派生物を含む明示同意付きパッケージ。マスキング済みレポートは再現に不足する項目を列挙し、「完全再現可能」と表示しない。共有用HTMLはscript/外部資産なしで、本文をtextとしてエスケープする。

schemaはminorで後方互換の任意フィールド追加、majorで破壊変更。未知majorは適合版を案内して書き換えない。DB移行前に利用者へバックアップ領域・必要容量を通知し、移行失敗時は旧DBを保持する。
