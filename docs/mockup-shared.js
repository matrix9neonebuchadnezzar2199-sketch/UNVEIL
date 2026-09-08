(function () {
  const themeBtn = document.getElementById("theme-toggle");
  if (themeBtn) {
    function applyTheme(theme) {
      document.documentElement.classList.toggle("light", theme === "light");
      themeBtn.textContent = theme === "light" ? "☀" : "🌙";
    }
    themeBtn.onclick = () =>
      applyTheme(document.documentElement.classList.contains("light") ? "dark" : "light");
  }

  let fontSize = 14;
  const plus = document.getElementById("font-plus");
  const minus = document.getElementById("font-minus");
  if (plus) {
    plus.onclick = () => {
      fontSize = Math.min(20, fontSize + 1);
      document.documentElement.style.fontSize = fontSize + "px";
    };
  }
  if (minus) {
    minus.onclick = () => {
      fontSize = Math.max(11, fontSize - 1);
      document.documentElement.style.fontSize = fontSize + "px";
    };
  }

  const collapse = document.getElementById("collapse-btn");
  const balloon = document.getElementById("sidebar-balloon");
  if (collapse) collapse.onclick = () => document.body.classList.add("sidebar-collapsed");
  if (balloon) balloon.onclick = () => document.body.classList.remove("sidebar-collapsed");

  const sidebar = document.getElementById("sidebar");
  const resizer = document.getElementById("sidebar-resizer");
  if (sidebar && resizer) {
    let dragging = false;
    resizer.addEventListener("mousedown", (e) => {
      if (e.target.id === "collapse-btn") return;
      dragging = true;
      resizer.classList.add("dragging");
      e.preventDefault();
    });
    document.addEventListener("mousemove", (e) => {
      if (!dragging) return;
      sidebar.style.width = Math.min(420, Math.max(160, e.clientX)) + "px";
    });
    document.addEventListener("mouseup", () => {
      dragging = false;
      resizer.classList.remove("dragging");
    });
  }

  document.querySelectorAll(".copy-btn").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const value = btn.getAttribute("data-copy") || "";
      try {
        await navigator.clipboard.writeText(value);
        btn.textContent = "copied";
        setTimeout(() => {
          btn.textContent = "📋";
        }, 1200);
      } catch {
        btn.textContent = "fail";
      }
    });
  });

  initMarkdownExport();
})();

function defaultWorkspacePath() {
  return "%LOCALAPPDATA%\\UNVEIL\\reports";
}

function suggestedMdName() {
  const moduleId = document.body.getAttribute("data-md-module") || "unveil";
  const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
  return `${moduleId}-${stamp}.md`;
}

function sampleMarkdown() {
  const moduleId = document.body.getAttribute("data-md-module") || "unveil";
  const title = {
    dashboard: "UNVEIL Dashboard 概要",
    malware: "マルウェア解析レポート",
    deobfuscation: "難読化判定レポート",
    usb: "USBチェックレポート",
  }[moduleId] || "UNVEIL レポート";
  return [
    `# ${title}`,
    "",
    "- 生成: モック（実解析結果ではない）",
    "- 未検出 / clean は安全の証明ではない",
    "- Magika score はタイプ信頼度であり悪意確率ではない",
    "",
    "## ジョブ",
    "",
    "| 項目 | 値 |",
    "|---|---|",
    `| モジュール | ${moduleId} |`,
    "| job.status | completed |",
    "",
    "## 注意",
    "",
    "この Markdown はモックのダミー出力です。実装では Broker がエスケープ済み本文のみを書きます。",
    "",
  ].join("\n");
}

function showToast(text) {
  let toast = document.getElementById("md-toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.id = "md-toast";
    toast.className = "toast";
    document.body.appendChild(toast);
  }
  toast.textContent = text;
  toast.classList.add("show");
  setTimeout(() => toast.classList.remove("show"), 2800);
}

function setJobComplete(complete) {
  document.body.setAttribute("data-job-complete", complete ? "true" : "false");
  const btn = document.getElementById("md-export-btn");
  if (!btn) return;
  btn.disabled = !complete;
  btn.title = complete
    ? "解析結果を Markdown で出力する"
    : "解析が完了するまで出力できません";
}

function downloadMarkdown(text, filename) {
  const blob = new Blob([text], { type: "text/markdown;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

async function saveMarkdownToPicker(text, filename) {
  if (window.showSaveFilePicker) {
    const handle = await window.showSaveFilePicker({
      suggestedName: filename,
      types: [{ description: "Markdown", accept: { "text/markdown": [".md"] } }],
    });
    const writable = await handle.createWritable();
    await writable.write(text);
    await writable.close();
    return handle.name || filename;
  }
  downloadMarkdown(text, filename);
  return filename;
}

function initMarkdownExport() {
  const btn = document.getElementById("md-export-btn");
  if (!btn) return;

  const complete = document.body.getAttribute("data-job-complete") === "true";
  setJobComplete(complete);

  if (!document.getElementById("md-export-modal")) {
    const wrap = document.createElement("div");
    wrap.id = "md-export-modal";
    wrap.className = "modal-backdrop";
    wrap.innerHTML = `
      <div class="modal" role="dialog" aria-labelledby="md-export-title">
        <h2 id="md-export-title">結果を Markdown で出力</h2>
        <p class="sub">出力先を選んでから書き出します。原本パスは含めません。</p>
        <label class="dest-option">
          <input type="radio" name="md-dest" value="workspace" checked>
          <div>
            <strong>ワークスペース既定</strong>
            <div class="muted">UNVEIL 管理領域の reports フォルダ</div>
            <div class="path-row">
              <input type="text" id="md-workspace-path" readonly>
              <button type="button" class="copy-btn" id="md-copy-workspace" data-copy="">📋</button>
            </div>
          </div>
        </label>
        <label class="dest-option">
          <input type="radio" name="md-dest" value="picker">
          <div>
            <strong>場所を選ぶ</strong>
            <div class="muted">保存ダイアログでフォルダとファイル名を指定</div>
          </div>
        </label>
        <label class="dest-option">
          <input type="radio" name="md-dest" value="clipboard">
          <div>
            <strong>クリップボード</strong>
            <div class="muted">ファイルは作らず本文だけコピー</div>
          </div>
        </label>
        <div class="actions" style="margin-top:14px;justify-content:flex-end;">
          <button type="button" class="btn" id="md-export-cancel">閉じる</button>
          <button type="button" class="btn-save" id="md-export-confirm">出力する</button>
        </div>
      </div>`;
    document.body.appendChild(wrap);
  }

  const modal = document.getElementById("md-export-modal");
  const wsInput = document.getElementById("md-workspace-path");
  const wsCopy = document.getElementById("md-copy-workspace");
  const defaultPath = `${defaultWorkspacePath()}\\${suggestedMdName()}`;
  if (wsInput) wsInput.value = defaultPath;
  if (wsCopy) {
    wsCopy.setAttribute("data-copy", defaultPath);
    wsCopy.addEventListener("click", async (event) => {
      event.preventDefault();
      try {
        await navigator.clipboard.writeText(defaultPath);
        wsCopy.textContent = "copied";
        setTimeout(() => {
          wsCopy.textContent = "📋";
        }, 1200);
      } catch {
        wsCopy.textContent = "fail";
      }
    });
  }

  btn.addEventListener("click", () => {
    if (btn.disabled) return;
    modal.classList.add("open");
  });
  document.getElementById("md-export-cancel").addEventListener("click", () => {
    modal.classList.remove("open");
  });
  modal.addEventListener("click", (event) => {
    if (event.target === modal) modal.classList.remove("open");
  });

  document.getElementById("md-export-confirm").addEventListener("click", async () => {
    const dest = document.querySelector('input[name="md-dest"]:checked')?.value || "workspace";
    const text = sampleMarkdown();
    const filename = suggestedMdName();
    try {
      if (dest === "clipboard") {
        await navigator.clipboard.writeText(text);
        showToast("Markdown をクリップボードにコピーしました");
      } else if (dest === "picker") {
        const saved = await saveMarkdownToPicker(text, filename);
        showToast(`保存しました: ${saved}`);
      } else {
        downloadMarkdown(text, filename);
        showToast(`ワークスペース相当として保存ダイアログを開きました: ${filename}`);
      }
      modal.classList.remove("open");
    } catch (err) {
      if (err && err.name === "AbortError") return;
      showToast("出力に失敗しました");
    }
  });

  document.querySelectorAll("[data-complete-job]").forEach((el) => {
    el.addEventListener("click", () => {
      setJobComplete(true);
      const fill = document.getElementById("usb-progress-fill");
      const label = document.getElementById("usb-progress-label");
      if (fill) fill.style.width = "100%";
      if (label) label.textContent = "1260 / 1260 files · completed";
    });
  });
}
