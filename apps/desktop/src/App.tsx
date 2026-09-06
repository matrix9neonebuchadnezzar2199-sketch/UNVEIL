import { useEffect, useState } from "react";
import Analyze from "./views/Analyze";
import Dashboard from "./views/Dashboard";
import type { IsolationDiagnosis, PageId } from "./types";
import { diagnoseIsolation, formatIpcError } from "./ipc";
import "./App.css";

function App() {
  const [page, setPage] = useState<PageId>("dashboard");
  const [diagnosis, setDiagnosis] = useState<IsolationDiagnosis | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    diagnoseIsolation()
      .then(setDiagnosis)
      .catch((cause) => setError(formatIpcError(cause)));
  }, []);

  useEffect(() => {
    const themeBtn = document.getElementById("theme-toggle");
    const plus = document.getElementById("font-plus");
    const minus = document.getElementById("font-minus");
    const collapse = document.getElementById("collapse-btn");
    const balloon = document.getElementById("sidebar-balloon");
    const sidebar = document.getElementById("sidebar");
    const resizer = document.getElementById("sidebar-resizer");
    if (!themeBtn || !plus || !minus || !collapse || !balloon || !sidebar || !resizer) {
      return;
    }
    let fontSize = 14;
    let dragging = false;
    const onTheme = () => {
      const light = document.documentElement.classList.toggle("light");
      themeBtn.textContent = light ? "☀" : "🌙";
    };
    const onPlus = () => {
      fontSize = Math.min(20, fontSize + 1);
      document.documentElement.style.fontSize = `${fontSize}px`;
    };
    const onMinus = () => {
      fontSize = Math.max(11, fontSize - 1);
      document.documentElement.style.fontSize = `${fontSize}px`;
    };
    const onCollapse = () => document.body.classList.add("sidebar-collapsed");
    const onBalloon = () => document.body.classList.remove("sidebar-collapsed");
    const onDown = (event: MouseEvent) => {
      if ((event.target as HTMLElement).id === "collapse-btn") {
        return;
      }
      dragging = true;
      resizer.classList.add("dragging");
      event.preventDefault();
    };
    const onMove = (event: MouseEvent) => {
      if (!dragging) {
        return;
      }
      sidebar.style.width = `${Math.min(420, Math.max(160, event.clientX))}px`;
    };
    const onUp = () => {
      dragging = false;
      resizer.classList.remove("dragging");
    };
    themeBtn.addEventListener("click", onTheme);
    plus.addEventListener("click", onPlus);
    minus.addEventListener("click", onMinus);
    collapse.addEventListener("click", onCollapse);
    balloon.addEventListener("click", onBalloon);
    resizer.addEventListener("mousedown", onDown);
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => {
      themeBtn.removeEventListener("click", onTheme);
      plus.removeEventListener("click", onPlus);
      minus.removeEventListener("click", onMinus);
      collapse.removeEventListener("click", onCollapse);
      balloon.removeEventListener("click", onBalloon);
      resizer.removeEventListener("mousedown", onDown);
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
  }, []);

  return (
    <>
      <header>
        <h1>UNVEIL</h1>
        <span className="pill">{page === "dashboard" ? "概要" : "Analyze"}</span>
        <span className="pill">STATIC</span>
        {diagnosis && !diagnosis.available ? (
          <span className="pill warn">隔離未実証 · 解析開始不可</span>
        ) : null}
        <span className="spacer" />
        <span className="hdr-group">
          <span className="lbl">文字</span>
          <button type="button" className="hdr-btn" id="font-minus" title="フォントサイズを小さく">
            −
          </button>
          <button type="button" className="hdr-btn" id="font-plus" title="フォントサイズを大きく">
            ＋
          </button>
        </span>
        <span className="hdr-group">
          <button type="button" className="hdr-btn" id="theme-toggle" title="ライト／ダーク切替">
            🌙
          </button>
        </span>
      </header>
      <div id="sidebar-balloon" title="メニューを開く">
        💬 メニュー
      </div>
      <div id="layout">
        <nav id="sidebar">
          <h3>画面</h3>
          <button
            type="button"
            className={page === "dashboard" ? "side-link active" : "side-link"}
            onClick={() => setPage("dashboard")}
          >
            Dashboard
          </button>
          <button
            type="button"
            className={page === "analyze" ? "side-link active" : "side-link"}
            onClick={() => setPage("analyze")}
          >
            Analyze
          </button>
          <div className="side-meta">
            v0.1.0 · schema 1.0
            <br />
            隔離: {diagnosis?.error_code ?? (diagnosis?.available ? "OK" : "…")}
          </div>
        </nav>
        <div id="sidebar-resizer" title="ドラッグで幅調整">
          <button type="button" id="collapse-btn" title="サイドバーを折りたたむ">
            ▼
          </button>
        </div>
        <main>
          {error ? <p className="err">{error}</p> : null}
          {page === "dashboard" ? (
            <Dashboard onOpenAnalyze={() => setPage("analyze")} />
          ) : (
            <Analyze isolationAvailable={Boolean(diagnosis?.available)} />
          )}
        </main>
      </div>
    </>
  );
}

export default App;
