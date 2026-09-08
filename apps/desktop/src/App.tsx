import { useEffect, useState } from "react";
import Analyze from "./views/Analyze";
import Dashboard from "./views/Dashboard";
import Malware from "./views/Malware";
import Usb from "./views/Usb";
import type { IsolationDiagnosis, ModuleJob, ModuleSpecView, PageId } from "./types";
import { diagnoseIsolation, formatIpcError, loadModules } from "./ipc";
import "./App.css";

function isPageId(id: string): id is PageId {
  return id === "dashboard" || id === "deobfuscation" || id === "malware" || id === "usb";
}

function App() {
  const [page, setPage] = useState<PageId>("dashboard");
  const [diagnosis, setDiagnosis] = useState<IsolationDiagnosis | null>(null);
  const [modules, setModules] = useState<ModuleSpecView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [malwareHandoffJob, setMalwareHandoffJob] = useState<ModuleJob | null>(null);
  const [analyzeKey, setAnalyzeKey] = useState(0);

  useEffect(() => {
    diagnoseIsolation()
      .then(setDiagnosis)
      .catch((cause) => setError(formatIpcError(cause)));
    loadModules()
      .then((payload) => setModules(payload.modules ?? []))
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

  const active = modules.find((item) => item.id === page);
  const isolationOk = Boolean(diagnosis?.available);

  return (
    <>
      <header>
        <h1>UNVEIL</h1>
        <span className="pill">{active?.label_ja ?? page}</span>
        <span className="pill">STATIC</span>
        {diagnosis && !diagnosis.available ? (
          <span className="pill warn">隔離未実証 · 解析開始不可</span>
        ) : (
          <span className="pill">隔離 OK</span>
        )}
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
          <h3>モジュール</h3>
          {modules.map((item) => {
            const id = isPageId(item.id) ? item.id : "dashboard";
            const disabled = item.disabled;
            return (
              <button
                key={item.id}
                type="button"
                className={`${page === id ? "side-link active" : "side-link"}${disabled ? " disabled" : ""}`}
                disabled={disabled}
                title={disabled ? "準備中" : item.label_ja}
                onClick={() => {
                  if (!disabled && isPageId(item.id)) {
                    setPage(item.id);
                  }
                }}
              >
                {item.label_ja}
                {item.status === "planned" ? <span className="side-badge">準備中</span> : null}
                {item.status === "mvp" ? <span className="side-badge mvp">MVP</span> : null}
              </button>
            );
          })}
          <div className="side-meta">
            v0.2.0 · schema 1.0
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
            <Dashboard onOpenAnalyze={() => setPage("deobfuscation")} />
          ) : null}
          {page === "deobfuscation" ? (
            <Analyze key={analyzeKey} isolationAvailable={isolationOk} />
          ) : null}
          {page === "malware" ? (
            <Malware
              isolationAvailable={isolationOk}
              initialJob={malwareHandoffJob}
              onHandoffDeobfuscation={() => {
                setAnalyzeKey((value) => value + 1);
                setPage("deobfuscation");
              }}
            />
          ) : null}
          {page === "usb" ? (
            <Usb
              isolationAvailable={isolationOk}
              onHandoffMalware={(job) => {
                setMalwareHandoffJob(job);
                setPage("malware");
              }}
            />
          ) : null}
        </main>
      </div>
    </>
  );
}

export default App;
