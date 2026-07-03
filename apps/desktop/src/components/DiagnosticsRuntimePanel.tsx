import type { ModelEntry, RuntimeStatus } from "../lib/ipc";
import {
  fallbackExplanation,
  resolveLoadedModel,
  runtimeStateSummary,
  variantBadge,
  variantBadgeClass,
} from "../lib/diagnosticsDisplay";

export function DiagnosticsRuntimePanel({
  runtime,
  models,
}: {
  runtime: RuntimeStatus;
  models: ModelEntry[];
}) {
  const state = runtimeStateSummary(runtime.state);
  const badge = variantBadge(runtime.variant, runtime.accelerator_label);
  const fallback = runtime.fallback_reason
    ? fallbackExplanation(runtime.fallback_reason)
    : null;
  const loaded = resolveLoadedModel(runtime.model_id, models);

  return (
    <section className="rounded-xl border border-brand-border bg-brand-card p-4">
      <h2 className="text-sm font-semibold">Local runtime</h2>
      <p className="mt-0.5 text-xs text-brand-textMuted">
        How Omnira is running your model on this computer. The main Chat screen
        only shows &ldquo;Running locally&rdquo;.
      </p>

      <div className="mt-4 flex flex-wrap items-center gap-3">
        <span className="inline-flex items-center gap-2 text-sm">
          <span className={`h-2.5 w-2.5 rounded-full ${state.dotClass}`} />
          {state.label}
        </span>
        <span
          className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${variantBadgeClass(badge.tone)}`}
        >
          {badge.label}
        </span>
      </div>

      {fallback && (
        <div className="mt-4 rounded-lg border border-accent-warning/30 bg-accent-warning/10 px-4 py-3">
          <p className="text-sm font-medium text-zinc-100">{fallback.title}</p>
          <p className="mt-1 text-xs leading-relaxed text-brand-textMuted">
            {fallback.body}
          </p>
          <p className="mt-2 font-mono text-[11px] text-zinc-600">
            {fallback.technicalDetail}
          </p>
        </div>
      )}

      <dl className="mt-4 grid gap-3 sm:grid-cols-2">
        <DetailRow
          label="Context size"
          value={
            runtime.context_size
              ? `${runtime.context_size.toLocaleString()} tokens`
              : "Not available"
          }
        />
        <DetailRow
          label="Loaded model"
          value={loaded ? loaded.name : runtime.model_id ? "Unknown model" : "None"}
        />
      </dl>

      {loaded && (
        <p className="mt-2 truncate text-xs text-zinc-600" title={loaded.path}>
          {loaded.path}
          {loaded.status === "missing" && (
            <span className="ml-2 text-accent-warning">(file missing)</span>
          )}
        </p>
      )}

      {runtime.model_id && (
        <p className="mt-2 font-mono text-[11px] text-zinc-600">
          Model id: {runtime.model_id}
        </p>
      )}
    </section>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs text-brand-textMuted">{label}</dt>
      <dd className="mt-0.5 text-sm">{value}</dd>
    </div>
  );
}
