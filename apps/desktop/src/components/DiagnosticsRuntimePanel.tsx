import { useEffect, useState } from "react";
import {
  ipc,
  toAppError,
  type AppError,
  type ModelEntry,
  type RuntimeStatus,
  type Settings,
} from "../lib/ipc";
import {
  engineSummary,
  fallbackExplanation,
  prefersCpuFromEarlierLaunch,
  resolveLoadedModel,
  runtimeStateSummary,
  variantBadge,
  variantBadgeClass,
} from "../lib/diagnosticsDisplay";

export function DiagnosticsRuntimePanel({
  runtime,
  models,
  onError,
}: {
  runtime: RuntimeStatus;
  models: ModelEntry[];
  onError?: (error: AppError) => void;
}) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [resetDone, setResetDone] = useState(false);

  useEffect(() => {
    void ipc.getSettings().then(setSettings).catch(() => setSettings(null));
  }, [runtime.state, runtime.variant]);

  const state = runtimeStateSummary(runtime.state);
  const badge = variantBadge(runtime.variant, runtime.accelerator_label);
  const fallback = runtime.fallback_reason
    ? fallbackExplanation(runtime.fallback_reason)
    : null;
  const loaded = resolveLoadedModel(runtime.model_id, models);
  const canRetryVulkan =
    settings != null && prefersCpuFromEarlierLaunch(settings.preferred_runtime_variant);

  const retryVulkan = async () => {
    try {
      const stored = await ipc.getSettings();
      const next = { ...stored, preferred_runtime_variant: null };
      await ipc.saveSettings(next);
      setSettings(next);
      setResetDone(true);
    } catch (e) {
      onError?.(toAppError(e));
    }
  };

  return (
    <section className="rounded-xl border border-brand-border bg-brand-card p-4">
      <h2 className="text-sm font-semibold">Local runtime</h2>
      <p className="mt-0.5 text-xs text-brand-textMuted">
        How Omnira is running your model on this computer. The main Chat screen
        only shows &ldquo;Running locally&rdquo; and the engine name.
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
          <p className="mt-2 select-text break-words font-mono text-[11px] text-zinc-600">
            {fallback.technicalDetail}
          </p>
        </div>
      )}

      {canRetryVulkan && !resetDone && (
        <div className="mt-3 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-brand-border px-4 py-3">
          <p className="text-xs text-brand-textMuted">
            Omnira remembers that CPU worked and starts with it. Retry GPU
            acceleration (Vulkan) the next time a model is loaded.
          </p>
          <button
            type="button"
            onClick={() => void retryVulkan()}
            className="shrink-0 rounded-lg border border-brand-border px-3 py-1.5 text-xs font-medium hover:bg-brand-hover"
          >
            Try GPU acceleration again
          </button>
        </div>
      )}
      {resetDone && (
        <p className="mt-3 text-xs text-accent-success">
          Vulkan will be tried first on the next model load. Reload the model
          from Models or Chat to apply.
        </p>
      )}

      <dl className="mt-4 grid gap-3 sm:grid-cols-2">
        <DetailRow
          label="Engine"
          value={engineSummary(runtime.engine_label, runtime.state)}
        />
        <DetailRow
          label="Mode"
          value={runtime.accelerator_label ?? "Not running"}
        />
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
        <p className="mt-2 select-text truncate text-xs text-zinc-600" title={loaded.path}>
          {loaded.path}
          {loaded.status === "missing" && (
            <span className="ml-2 text-accent-warning">(file missing)</span>
          )}
        </p>
      )}

      {runtime.model_id && (
        <p className="mt-2 select-text font-mono text-[11px] text-zinc-600">
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
      <dd className="mt-0.5 select-text text-sm">{value}</dd>
    </div>
  );
}
