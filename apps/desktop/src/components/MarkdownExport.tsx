import { useState } from "react";
import { exportMarkdown } from "../ipc";

type Props = {
  moduleId: string;
  disabled?: boolean;
  onStatus: (message: string) => void;
};

export default function MarkdownExport({ moduleId, disabled, onStatus }: Props) {
  const [dest, setDest] = useState<"clipboard" | "workspace" | "picker">("clipboard");
  const [busy, setBusy] = useState(false);

  async function runExport() {
    setBusy(true);
    try {
      const result = await exportMarkdown(moduleId, dest);
      if (dest === "clipboard") {
        await navigator.clipboard.writeText(result);
        onStatus("Markdown をクリップボードへコピーしました。");
      } else {
        onStatus(`Markdown を保存しました: ${result}`);
      }
    } catch (cause) {
      onStatus(String(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="actions" style={{ flexWrap: "wrap", gap: 8 }}>
      <label className="muted">
        出力先{" "}
        <select
          value={dest}
          disabled={disabled || busy}
          onChange={(event) =>
            setDest(event.target.value as "clipboard" | "workspace" | "picker")
          }
        >
          <option value="clipboard">クリップボード</option>
          <option value="workspace">ワークスペース</option>
          <option value="picker">保存ダイアログ</option>
        </select>
      </label>
      <button type="button" className="btn-save" disabled={disabled || busy} onClick={() => void runExport()}>
        結果をマークダウン形式で出力
      </button>
    </div>
  );
}
