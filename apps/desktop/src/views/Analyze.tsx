import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  approveTransform,
  applyTransform,
  chooseFile,
  exportMarkdown,
  exportReport,
  formatIpcError,
  importFile,
  importLesson,
  lastProbe,
  loadCurrentArtifact,
  loadDetectors,
  planTransform,
  previewTransform,
  saveSession,
  startAnalysis,
  wikiGet,
  wikiSearch,
} from "../ipc";
import type {
  AnalysisResult,
  ContentTypeProbe,
  DetectorToggle,
  Finding,
  ImportedArtifact,
  SelectionPreview,
  TransformPlan,
  TransformPreview,
  WikiArticle,
  WikiHit,
} from "../types";

type Props = {
  isolationAvailable: boolean;
};

export default function Analyze({ isolationAvailable }: Props) {
  const [preview, setPreview] = useState<SelectionPreview | null>(null);
  const [artifact, setArtifact] = useState<ImportedArtifact | null>(null);
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [detectors, setDetectors] = useState<DetectorToggle[]>([]);
  const [status, setStatus] = useState("ファイルを選択してください");
  const [busy, setBusy] = useState(false);
  const [plan, setPlan] = useState<TransformPlan | null>(null);
  const [xform, setXform] = useState<TransformPreview | null>(null);
  const [wikiHits, setWikiHits] = useState<WikiHit[]>([]);
  const [article, setArticle] = useState<WikiArticle | null>(null);
  const [wikiQuery, setWikiQuery] = useState("");
  const [probe, setProbe] = useState<ContentTypeProbe | null>(null);

  useEffect(() => {
    void loadDetectors().then(setDetectors);
    void wikiSearch("").then(setWikiHits);
    void loadCurrentArtifact().then((current) => {
      if (current) {
        setArtifact(current);
        setStatus(`引き渡し試料: ${current.display_name}`);
      }
    });
    const unlisten = listen<SelectionPreview>("selection-ready", (event) => {
      setPreview(event.payload);
      setArtifact(null);
      setAnalysis(null);
      setStatus("ドロップを受け取りました。取込を実行してください。");
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  async function run<T>(fn: () => Promise<T>): Promise<T | undefined> {
    setBusy(true);
    try {
      return await fn();
    } catch (cause) {
      setStatus(formatIpcError(cause));
      return undefined;
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <div className="banner">
        取込 → 根拠 → WIKI → プレビュー → 承認 → 変換 → レポート。未検出は安全ではない。入力は実行しない。
      </div>
      <section className="drop">
        <div>
          <strong>ファイルを選択、またはウィンドウへドロップ</strong>
          <p className="muted" style={{ margin: "6px 0 0" }}>
            静的解析のみ。最大 50 MiB。変換は承認後の派生物のみ。
          </p>
        </div>
        <div className="actions">
          <button type="button" className="btn" disabled={busy} onClick={() => void run(async () => {
            const next = await chooseFile();
            setPreview(next);
            setArtifact(null);
            setAnalysis(null);
            setStatus("選択しました。取込を実行してください。");
          })}>
            ファイルを選択
          </button>
          <button type="button" className="btn-save" disabled={busy} onClick={() => void run(async () => {
            const next = await importLesson();
            setPreview(null);
            setArtifact(next);
            setAnalysis(null);
            setProbe(await lastProbe());
            setStatus("教材スナップショットを作成しました。");
          })}>
            教材から始める
          </button>
          <button
            type="button"
            className="btn-warn"
            disabled={busy || !artifact || !isolationAvailable}
            onClick={() => void run(async () => {
              if (!artifact) return;
              const result = await startAnalysis(artifact.artifact_id);
              setAnalysis(result);
              setProbe(await lastProbe());
              setStatus(`解析完了 assessment=${result.assessment} coverage=${result.coverage.status}`);
            })}
          >
            {isolationAvailable ? "静的解析を開始" : "解析を開始（隔離不足のため無効）"}
          </button>
          <button type="button" className="btn-save" disabled={busy} onClick={() => void run(async () => {
            await saveSession("research");
            setStatus("セッションを保存しました（明示保存）。");
          })}>
            セッション保存
          </button>
          <button type="button" className="btn" disabled={busy} onClick={() => void run(async () => {
            const report = await exportReport(false);
            await navigator.clipboard.writeText(report.json);
            setStatus(`レポートをコピーしました。原本同梱=${report.included_original}`);
          })}>
            レポートJSONコピー
          </button>
          <button
            type="button"
            className="btn-save"
            disabled={busy || !analysis}
            onClick={() => void run(async () => {
              const body = await exportMarkdown("deobfuscation", "clipboard");
              await navigator.clipboard.writeText(body);
              setStatus("Markdown をクリップボードへコピーしました。原本パスは含みません。");
            })}
          >
            結果をマークダウン形式で出力
          </button>
        </div>
      </section>

      <section className="card">
        <h2>取込確認</h2>
        <p className="sub">生パスは表示しない。無制限化はできない。</p>
        <table>
          <tbody>
            <tr>
              <td>選択中</td>
              <td>
                {preview ? (
                  <><code>{preview.display_name}</code> · {preview.byte_length} bytes</>
                ) : artifact ? (
                  <><code>{artifact.display_name}</code> · {artifact.byte_length} bytes</>
                ) : (
                  <span className="muted">未選択</span>
                )}
              </td>
              <td>
                {preview ? (
                  <button type="button" className="btn" disabled={busy} onClick={() => void run(async () => {
                    const next = await importFile(preview.token);
                    setArtifact(next);
                    setProbe(await lastProbe());
                    setStatus("スナップショットを作成しました。原本は変更していません。");
                  })}>
                    取込
                  </button>
                ) : null}
              </td>
            </tr>
            <tr>
              <td>SHA-256</td>
              <td>{artifact ? <code className="mono">{artifact.sha256}</code> : <span className="muted">取込後</span>}</td>
              <td>
                {artifact ? (
                  <button type="button" className="copy-btn" onClick={() => void navigator.clipboard.writeText(artifact.sha256)}>📋</button>
                ) : null}
              </td>
            </tr>
          </tbody>
        </table>
      </section>

      {probe ? (
        <section className="card">
          <h2>ContentType Probe</h2>
          <p className="sub">Magika score はタイプ信頼度。不一致は注意のみ。Ghidra 投入判定とは別。</p>
          <table>
            <tbody>
              <tr>
                <td>拡張子</td>
                <td><code>{probe.declared_extension || "—"}</code></td>
              </tr>
              <tr>
                <td>magic</td>
                <td><code>{probe.magic_hint}</code></td>
              </tr>
              <tr>
                <td>Magika</td>
                <td>
                  <code>{probe.magika.status}</code>
                  {probe.magika.output_label ? ` · ${probe.magika.output_label}` : null}
                  {probe.magika.score != null ? (
                    <span className="muted"> · タイプ信頼度 {probe.magika.score.toFixed(3)}（悪意ではない）</span>
                  ) : null}
                </td>
              </tr>
              <tr>
                <td>mismatch</td>
                <td>{probe.mismatch_flags.length ? probe.mismatch_flags.join(", ") : "なし"}</td>
              </tr>
            </tbody>
          </table>
        </section>
      ) : null}

      <section className="card">
        <h2>このジョブの検知器（ON/OFF）</h2>
        <p className="sub">OFF は MVP 対象外。製品名の特定はしない。</p>
        {detectors.map((d) => (
          <div className="setting-row" key={d.id}>
            <input className="toggle" type="checkbox" checked={d.enabled} disabled readOnly />
            <div>
              <strong>{d.id}</strong>
              <div className="muted">{d.summary}</div>
            </div>
          </div>
        ))}
      </section>

      {artifact ? (
        <div className="kpi-grid">
          <div className="kpi">
            <div className="val">{analysis?.assessment ?? "—"}</div>
            <div className="lbl">assessment（安全判定ではない）</div>
          </div>
          <div className="kpi">
            <div className="val">{analysis?.findings.length ?? "—"}</div>
            <div className="lbl">Finding</div>
          </div>
          <div className="kpi">
            <div className="val">{artifact.byte_length}</div>
            <div className="lbl">スナップショット bytes</div>
          </div>
          <div className="kpi">
            <div className="val">{analysis?.coverage.status ?? "unrun"}</div>
            <div className="lbl">coverage</div>
          </div>
        </div>
      ) : null}

      <div className="workspace">
        <section className="card">
          <h2>原本 / 根拠</h2>
          <div className="codeview">{analysis?.text_preview || artifact?.sha256 || "未取込"}</div>
          {analysis ? (
            <table>
              <thead>
                <tr>
                  <th>方式</th>
                  <th>識別</th>
                  <th>一致度</th>
                  <th>変換</th>
                </tr>
              </thead>
              <tbody>
                {analysis.findings.map((f) => (
                  <FindingRow
                    key={f.id}
                    finding={f}
                    busy={busy}
                    disabled={!isolationAvailable}
                    onPreview={() => void run(async () => {
                      const nextPlan = await planTransform(f.artifact_id, f.id);
                      setPlan(nextPlan);
                      const nextPrev = await previewTransform(nextPlan.plan_id);
                      setXform(nextPrev);
                      setStatus(`preview ${nextPrev.result} verdict=${nextPrev.validation.verdict}`);
                    })}
                    onWiki={() => void run(async () => {
                      const hits = await wikiSearch(f.technique_id);
                      setWikiHits(hits);
                      if (hits[0]) {
                        setArticle(await wikiGet(hits[0].article_id));
                      }
                    })}
                  />
                ))}
              </tbody>
            </table>
          ) : null}
          {xform ? (
            <div>
              <h2>プレビュー</h2>
              <p className="sub">未承認候補。検証={xform.validation.verdict}。意味等価ではない。</p>
              <div className="codeview">{xform.candidate_preview}</div>
              <div className="actions">
                <button type="button" className="btn-save" disabled={busy || !plan} onClick={() => void run(async () => {
                  if (!plan) return;
                  const approval = await approveTransform(plan.plan_id, plan.plan_hash);
                  await applyTransform(plan.plan_id, approval.approval_id, plan.plan_hash);
                  setStatus("派生物を確定しました。原本は不変です。");
                })}>
                  承認して適用
                </button>
              </div>
            </div>
          ) : null}
        </section>
        <aside>
          <section className="card">
            <h2>WIKI</h2>
            <div className="actions">
              <input
                value={wikiQuery}
                onChange={(e) => setWikiQuery(e.target.value)}
                placeholder="検索（base64 など）"
                aria-label="WIKI検索"
              />
              <button type="button" className="btn" disabled={busy} onClick={() => void run(async () => {
                setWikiHits(await wikiSearch(wikiQuery));
              })}>
                検索
              </button>
            </div>
            <ul className="legend">
              {wikiHits.map((h) => (
                <li key={h.article_id}>
                  <button type="button" className="copy-btn" onClick={() => void run(async () => {
                    setArticle(await wikiGet(h.article_id));
                  })}>
                    {h.title}
                  </button>
                  <span className="muted"> {h.engine_support}</span>
                </li>
              ))}
            </ul>
            {article ? (
              <div className="codeview" style={{ whiteSpace: "pre-wrap" }}>
                {article.title}
                {"\n"}
                engine_support: {article.engine_support}
                {"\n\n"}
                {article.body_markdown}
              </div>
            ) : (
              <p className="muted">記事を選ぶとオフライン本文を表示します。raw HTML は使いません。</p>
            )}
          </section>
        </aside>
      </div>
      <div className="status">{status}</div>
    </>
  );
}

function FindingRow({
  finding,
  busy,
  disabled,
  onPreview,
  onWiki,
}: {
  finding: Finding;
  busy: boolean;
  disabled: boolean;
  onPreview: () => void;
  onWiki: () => void;
}) {
  return (
    <tr>
      <td className="mono">{finding.technique_id}</td>
      <td>{finding.identification}</td>
      <td className="mono">{finding.score.value}</td>
      <td>
        <button type="button" className="btn" disabled={busy || disabled || finding.transformability === "unsupported"} onClick={onPreview}>
          プレビュー
        </button>{" "}
        <button type="button" className="copy-btn" onClick={onWiki}>
          WIKI
        </button>
      </td>
    </tr>
  );
}
