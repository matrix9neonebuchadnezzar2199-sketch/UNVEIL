import { useEffect, useState } from "react";
import {
  diagnoseModule,
  formatIpcError,
  handoffUsbFile,
  usbCancel,
  usbDrives,
  usbScan,
} from "../ipc";
import MarkdownExport from "../components/MarkdownExport";
import type { ModuleDiagnosis, ModuleJob, UsbDrive, UsbScanPayload } from "../types";

type Props = {
  isolationAvailable: boolean;
  onHandoffMalware?: (job: ModuleJob) => void;
};

function payloadOf(job: ModuleJob | null): UsbScanPayload {
  if (!job || typeof job.payload !== "object" || job.payload === null) {
    return {};
  }
  return job.payload as UsbScanPayload;
}

function shortSha256(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }
  return value.length > 16 ? `${value.slice(0, 8)}…${value.slice(-8)}` : value;
}

export default function Usb({ isolationAvailable, onHandoffMalware }: Props) {
  const [diagnosis, setDiagnosis] = useState<ModuleDiagnosis | null>(null);
  const [drives, setDrives] = useState<UsbDrive[]>([]);
  const [selectedToken, setSelectedToken] = useState<string | null>(null);
  const [labFolderMode, setLabFolderMode] = useState(false);
  const [job, setJob] = useState<ModuleJob | null>(null);
  const [status, setStatus] = useState("リムーバブルドライブを選ぶか、lab フォルダモードを有効にしてください");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    diagnoseModule("usb")
      .then(setDiagnosis)
      .catch((cause) => setStatus(formatIpcError(cause)));
    usbDrives()
      .then((payload) => setDrives(payload.drives ?? []))
      .catch(() => setDrives([]));
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

  const ready = Boolean(diagnosis?.available && isolationAvailable);
  const scanPayload = payloadOf(job);
  const inventory = scanPayload.inventory ?? [];
  const complete = job?.status === "completed";
  const cancelled = job?.status === "cancelled";

  return (
    <>
      <div className="banner">
        USBチェックは読み取り専用・静的のみ。overall は安全証明ではない。対象へは書き込まない。
      </div>
      <section className="card">
        <h2>リムーバブルドライブ</h2>
        {drives.length ? (
          <table>
            <tbody>
              {drives.map((drive) => (
                <tr key={drive.token}>
                  <td>
                    <label>
                      <input
                        type="radio"
                        name="usb-drive"
                        checked={selectedToken === drive.token}
                        onChange={() => setSelectedToken(drive.token)}
                      />
                      {" "}
                      <code>{drive.letter}</code>
                      {drive.composite_suspect ? " · composite 疑い" : null}
                    </label>
                  </td>
                  <td className="muted">
                    {drive.vid && drive.pid ? `VID_${drive.vid} PID_${drive.pid}` : "—"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <p className="muted">接続中のリムーバブルドライブはありません（空配列でも正常）。</p>
        )}
        <div className="setting-row" style={{ marginTop: 12 }}>
          <input
            className="toggle"
            type="checkbox"
            checked={labFolderMode}
            onChange={(event) => setLabFolderMode(event.target.checked)}
          />
          <div>
            <strong>lab フォルダモード</strong>
            <div className="muted">任意ディレクトリをスキャン（非リムーバブル既定拒否の例外）</div>
          </div>
        </div>
      </section>
      <section className="drop">
        <div>
          <strong>USB / フォルダ静的スキャン</strong>
          <p className="muted" style={{ margin: "6px 0 0" }}>USB-GuardDuty エンジン · workers 既定 4 · --online 既定 OFF</p>
        </div>
        <div className="actions">
          <button
            type="button"
            className="btn-warn"
            disabled={busy || !ready || (!selectedToken && !labFolderMode)}
            onClick={() => void run(async () => {
              const next = await usbScan({
                driveToken: labFolderMode ? undefined : selectedToken ?? undefined,
                folderMode: labFolderMode,
                workers: 4,
              });
              if (next) {
                setJob(next);
                const body = payloadOf(next);
                setStatus(`scanned status=${next.status} overall=${body.overall ?? "—"}`);
              }
            })}
          >
            {ready ? "スキャン開始" : "スキャン開始（能力不足のため無効）"}
          </button>
          <button
            type="button"
            className="btn-stop"
            disabled={!busy}
            onClick={() => void run(async () => {
              await usbCancel();
              setStatus("スキャンを停止しました");
            })}
          >
            停止
          </button>
          <MarkdownExport
            moduleId="usb"
            disabled={!complete || busy}
            onStatus={setStatus}
          />
        </div>
      </section>
      <section className="card">
        <h2>diagnose</h2>
        <ul className="legend">
          {(diagnosis?.checks ?? []).map((check) => (
            <li key={check.id}>
              <span className={check.available ? "ok" : "err"}>{check.available ? "OK" : "NG"}</span>
              {" "}
              <code>{check.id}</code> — {check.detail}
            </li>
          ))}
        </ul>
      </section>
      {job ? (
        <section className="card">
          <h2>結果</h2>
          <p className="sub">
            overall: <code>{scanPayload.overall ?? "—"}</code> · findings: {scanPayload.findings_count ?? 0}
            · {scanPayload.notice ?? "安全証明ではない"}
          </p>
          {cancelled ? <p className="warn">status=cancelled</p> : null}
          <p className="mono">{job.sha256 ?? "—"} {job.sha256 ? <button type="button" className="copy-btn" onClick={() => void navigator.clipboard.writeText(job.sha256 ?? "")}>📋</button> : null}</p>
          {inventory.length ? (
            <table>
              <thead>
                <tr>
                  <th>name</th>
                  <th>sha256</th>
                  <th>size</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {inventory.map((row) => (
                  <tr key={row.name}>
                    <td><code>{row.name}</code></td>
                    <td className="mono">{shortSha256(row.sha256)}</td>
                    <td>{row.size ?? "—"}</td>
                    <td>
                      {onHandoffMalware && job?.job_id ? (
                        <button
                          type="button"
                          className="btn"
                          disabled={busy || !ready}
                          onClick={() => void run(async () => {
                            const next = await handoffUsbFile(job.job_id, row.name);
                            if (next) {
                              onHandoffMalware(next);
                              setStatus(`handoff → malware · artifact=${next.input_artifact_id ?? "—"}`);
                            }
                          })}
                        >
                          マルウェア解析へ
                        </button>
                      ) : null}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : null}
        </section>
      ) : null}
      <div className="status">{status}</div>
    </>
  );
}
