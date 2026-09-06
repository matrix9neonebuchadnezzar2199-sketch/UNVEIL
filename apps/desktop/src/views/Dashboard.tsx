import { useEffect, useMemo, useState } from "react";
import { formatIpcError, loadOverview } from "../ipc";
import type { DashboardOverview } from "../types";

const PALETTE = ["#00d4ff", "#8b5cf6", "#10b981", "#e3892b"];

type Props = {
  onOpenAnalyze: () => void;
};

function pieBackground(overview: DashboardOverview): string {
  const total = overview.categories.reduce((sum, item) => sum + item.count, 0);
  if (total === 0) {
    return "conic-gradient(rgba(100,116,139,0.45) 0 100%)";
  }
  let start = 0;
  const stops: string[] = [];
  overview.categories.forEach((item, index) => {
    const next = start + (item.count / total) * 100;
    stops.push(`${PALETTE[index % PALETTE.length]} ${start}% ${next}%`);
    start = next;
  });
  return `conic-gradient(${stops.join(", ")})`;
}

export default function Dashboard({ onOpenAnalyze }: Props) {
  const [overview, setOverview] = useState<DashboardOverview | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadOverview()
      .then(setOverview)
      .catch((cause) => setError(formatIpcError(cause)));
  }, []);

  const maxTechnique = useMemo(() => {
    if (!overview) {
      return 1;
    }
    return Math.max(1, ...overview.techniques.map((item) => item.count));
  }, [overview]);

  if (!overview) {
    return <p className="muted">{error ?? "概要を読み込み中…"}</p>;
  }

  return (
    <>
      <div className="banner">
        Dashboard は保存研究の概要のみ。ファイル選択は Analyze。件数は実データ（未保存なら 0）。
        <button type="button" className="btn" style={{ marginLeft: 12 }} onClick={onOpenAnalyze}>
          Analyze を開く
        </button>
      </div>
      <div className="kpi-grid">
        <div className="kpi">
          <div className="val">{overview.saved_sessions}</div>
          <div className="lbl">保存セッション</div>
        </div>
        <div className="kpi">
          <div className="val">{overview.jobs_total}</div>
          <div className="lbl">解析ジョブ（累計）</div>
        </div>
        <div className="kpi">
          <div className="val">{overview.findings_total}</div>
          <div className="lbl">Finding（ルール一致）</div>
        </div>
        <div className="kpi">
          <div className={`val ${overview.isolation_passed === 0 ? "err" : "ok"}`}>
            {overview.isolation_passed}
          </div>
          <div className="lbl">隔離合格</div>
        </div>
      </div>
      <div className="charts">
        <section className="card">
          <h2>Finding 区分</h2>
          <p className="sub">実データが無いときは空円。安全判定ではない。</p>
          <div className="pie-wrap">
            <div
              className="pie"
              style={{ background: pieBackground(overview) }}
              role="img"
              aria-label="Finding 区分"
            />
            <ul className="legend">
              {overview.categories.length === 0 ? (
                <li className="muted">まだ Finding はありません</li>
              ) : (
                overview.categories.map((item, index) => (
                  <li key={item.id}>
                    <span className="swatch" style={{ background: PALETTE[index % PALETTE.length] }} />
                    {item.id} · {item.count}
                  </li>
                ))
              )}
            </ul>
          </div>
        </section>
        <section className="card">
          <h2>方式別 Finding 数</h2>
          <p className="sub">ルール一致の件数。</p>
          {overview.techniques.length === 0 ? (
            <p className="muted">解析が完了すると棒グラフが入ります。</p>
          ) : (
            overview.techniques.map((item) => (
              <div className="bar-row" key={item.id}>
                <span className="mono">{item.id}</span>
                <div className="bar-track">
                  <div className="bar" style={{ width: `${(item.count / maxTechnique) * 100}%` }} />
                </div>
                <span className="mono">{item.count}</span>
              </div>
            ))
          )}
        </section>
      </div>
      <section className="card">
        <h2>保存済みセッション</h2>
        {overview.sessions.length === 0 ? (
          <p className="muted">明示保存した研究だけがここに出ます。</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>名前</th>
                <th>Finding</th>
                <th>状態</th>
              </tr>
            </thead>
            <tbody>
              {overview.sessions.map((row) => (
                <tr key={row.name}>
                  <td>{row.name}</td>
                  <td className="mono">{row.findings}</td>
                  <td>{row.status}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </>
  );
}
