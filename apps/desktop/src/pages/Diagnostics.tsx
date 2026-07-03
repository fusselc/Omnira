import { useCallback, useEffect, useMemo, useState } from "react";
import { RefreshCw, Download } from "lucide-react";
import {
  ipc,
  toAppError,
  type AppError,
  type DiagnosticsSnapshot,
  type ModelEntry,
} from "../lib/ipc";
import { ErrorBanner } from "../components/ErrorBanner";
import { DiagnosticsRuntimePanel } from "../components/DiagnosticsRuntimePanel";
import { DiagnosticsLocalApiPanel } from "../components/DiagnosticsLocalApiPanel";

/**
 * Advanced Diagnostics: the only screen that names the accelerator, ports,
 * process state, and technical detail (docs/design-principles.md).
 */
export function Diagnostics() {
  const [snap, setSnap] = useState<DiagnosticsSnapshot | null>(null);
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [error, setError] = useState<AppError | null>(null);
  const [includePaths, setIncludePaths] = useState(false);
  const [exportedTo, setExportedTo] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const snapshot = await ipc.diagnosticsSnapshot();
    setSnap(snapshot);
    if (snapshot.runtime.model_id) {
      setModels(await ipc.listModels());
    } else {
      setModels([]);
    }
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

  const displayErrors = useMemo(() => {
    if (!snap) return [];
    const errors = [...snap.recent_errors];
    const last = snap.runtime.last_error;
    if (
      snap.runtime.state === "error" &&
      last &&
      !errors.some((e) => e.code === last.code && e.message === last.message)
    ) {
      errors.unshift(last);
    }
    return errors;
  }, [snap]);

  if (!snap) return null;

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

      <DiagnosticsRuntimePanel runtime={snap.runtime} models={models} />
      <DiagnosticsLocalApiPanel port={snap.runtime.port} state={snap.runtime.state} />

      <section className="rounded-xl border border-brand-border bg-brand-card p-4">
        <h2 className="text-sm font-semibold">Support information</h2>
        <p className="mt-0.5 text-xs text-brand-textMuted">
          Paths and version details for troubleshooting. Everything stays on this
          computer unless you export a report yourself.
        </p>
        <dl className="mt-4 space-y-2 text-sm">
          <PathRow label="App version" value={snap.app_version} mono />
          <PathRow label="Omnira data" value={snap.data_dir} mono />
          <PathRow label="Settings" value={snap.config_path} mono />
          <PathRow label="Conversations database" value={snap.db_path} mono />
          <PathRow label="Logs" value={snap.log_dir} mono />
        </dl>
      </section>

      {displayErrors.length > 0 && (
        <section className="rounded-xl border border-brand-border bg-brand-card p-4">
          <h2 className="mb-2 text-sm font-semibold">Recent runtime errors</h2>
          <ul className="space-y-2">
            {displayErrors.map((e, i) => (
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

function PathRow({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <dt className="text-xs text-brand-textMuted">{label}</dt>
      <dd className={`mt-0.5 break-all ${mono ? "font-mono text-xs text-zinc-500" : ""}`}>
        {value}
      </dd>
    </div>
  );
}
