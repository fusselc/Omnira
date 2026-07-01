import { useCallback, useEffect, useState } from "react";
import { RefreshCw, Download } from "lucide-react";
import {
  ipc,
  toAppError,
  type AppError,
  type DiagnosticsSnapshot,
} from "../lib/ipc";
import { ErrorBanner } from "../components/ErrorBanner";

/**
 * Advanced Diagnostics: the only screen that names the accelerator, ports,
 * process state, and technical detail (docs/design-principles.md).
 */
export function Diagnostics() {
  const [snap, setSnap] = useState<DiagnosticsSnapshot | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [includePaths, setIncludePaths] = useState(false);
  const [exportedTo, setExportedTo] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setSnap(await ipc.diagnosticsSnapshot());
  }, []);

  useEffect(() => {
    void refresh();
    const t = setInterval(() => void refresh(), 5000);
    return () => clearInterval(t);
  }, [refresh]);

  const exportReport = async () => {
    setError(null);
    setExportedTo(null);
    try {
      setExportedTo(await ipc.diagnosticsExport(includePaths));
    } catch (e) {
      setError(toAppError(e));
    }
  };

  if (!snap) return null;
  const rt = snap.runtime;

  return (
    <div className="mx-auto flex h-full max-w-4xl flex-col gap-4 overflow-y-auto px-8 py-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold">Advanced Diagnostics</h1>
          <p className="mt-1 text-sm text-brand-textMuted">
            Technical details about the local runtime. Omnira v{snap.app_version}.
          </p>
        </div>
        <button
          onClick={() => void refresh()}
          className="flex items-center gap-2 rounded-lg border border-brand-border px-3 py-1.5 text-xs hover:bg-brand-hover"
        >
          <RefreshCw size={13} />
          Refresh
        </button>
      </header>

      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}

      <section className="grid grid-cols-2 gap-3">
        <Stat label="Runtime state" value={rt.state} />
        <Stat
          label="Accelerator"
          value={rt.accelerator_label ?? "Not running"}
        />
        <Stat
          label="Local API binding"
          value={rt.port ? `127.0.0.1:${rt.port} (loopback only, key required)` : "Not bound"}
        />
        <Stat
          label="Context size"
          value={rt.context_size ? `${rt.context_size} tokens` : "n/a"}
        />
        {rt.fallback_reason && (
          <Stat label="Fallback reason" value={rt.fallback_reason} wide />
        )}
        {rt.model_id && <Stat label="Loaded model id" value={rt.model_id} wide />}
      </section>

      {snap.recent_errors.length > 0 && (
        <section className="rounded-xl border border-brand-border bg-brand-card p-4">
          <h2 className="mb-2 text-sm font-semibold">Recent runtime errors</h2>
          <ul className="space-y-2">
            {snap.recent_errors.map((e, i) => (
              <li key={i} className="text-xs">
                <span className="font-mono text-accent-danger">{e.code}</span>{" "}
                <span className="text-brand-textMuted">{e.message}</span>
                {e.detail && (
                  <p className="mt-0.5 font-mono text-[11px] text-zinc-600">{e.detail}</p>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="flex min-h-0 flex-1 flex-col rounded-xl border border-brand-border bg-brand-card p-4">
        <h2 className="mb-2 text-sm font-semibold">Recent local logs</h2>
        <p className="mb-2 text-xs text-zinc-600">
          Logs never contain your prompts or responses.
        </p>
        <pre className="min-h-32 flex-1 overflow-auto rounded-lg bg-brand-deep p-3 font-mono text-[11px] leading-relaxed text-zinc-400">
          {snap.recent_log_lines.length
            ? snap.recent_log_lines.join("\n")
            : "No log entries yet."}
        </pre>
      </section>

      <section className="flex items-center justify-between rounded-xl border border-brand-border bg-brand-card p-4">
        <div>
          <h2 className="text-sm font-semibold">Export diagnostics</h2>
          <p className="mt-0.5 text-xs text-brand-textMuted">
            Saves a report you can attach to a bug report. Redacted by default.
          </p>
          <label className="mt-2 flex items-center gap-2 text-xs text-brand-textMuted">
            <input
              type="checkbox"
              checked={includePaths}
              onChange={(e) => setIncludePaths(e.target.checked)}
              className="accent-accent-primary"
            />
            Include full file paths (opt-in)
          </label>
          {exportedTo && (
            <p className="mt-2 font-mono text-[11px] text-accent-success">
              Saved: {exportedTo}
            </p>
          )}
        </div>
        <button
          onClick={() => void exportReport()}
          className="flex items-center gap-2 rounded-lg bg-accent-primary px-4 py-2 text-sm font-medium text-white hover:bg-accent-primary/90"
        >
          <Download size={15} />
          Export
        </button>
      </section>
    </div>
  );
}

function Stat({ label, value, wide }: { label: string; value: string; wide?: boolean }) {
  return (
    <div
      className={`rounded-xl border border-brand-border bg-brand-card px-4 py-3 ${
        wide ? "col-span-2" : ""
      }`}
    >
      <dt className="text-xs text-brand-textMuted">{label}</dt>
      <dd className="mt-1 break-words font-mono text-sm">{value}</dd>
    </div>
  );
}
