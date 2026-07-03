import type { RuntimeState } from "../lib/ipc";
import { localApiSummary } from "../lib/diagnosticsDisplay";

export function DiagnosticsLocalApiPanel({
  port,
  state,
}: {
  port: number | null;
  state: RuntimeState;
}) {
  const rows = localApiSummary(port, state);

  return (
    <section className="rounded-xl border border-brand-border bg-brand-card p-4">
      <h2 className="text-sm font-semibold">Local API status</h2>
      <p className="mt-0.5 text-xs text-brand-textMuted">
        The chat engine listens only on your computer. Nothing is exposed to the
        internet by default.
      </p>
      <dl className="mt-4 space-y-3">
        {rows.map((row) => (
          <div key={row.label}>
            <dt className="text-xs font-medium text-brand-textMuted">{row.label}</dt>
            <dd className="mt-0.5 text-sm leading-relaxed">{row.value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
