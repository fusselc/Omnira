/**
 * Friendly display helpers for Advanced Diagnostics (UI-only).
 * Maps existing RuntimeStatus / ModelEntry fields to scannable copy.
 */
import type {
  ModelEntry,
  RuntimeState,
  RuntimeVariant,
} from "./ipc";

export interface RuntimeStateSummary {
  label: string;
  dotClass: string;
}

export interface VariantBadge {
  label: string;
  tone: "cuda" | "vulkan" | "cpu" | "none";
}

export interface FallbackExplanation {
  title: string;
  body: string;
  technicalDetail: string;
}

export interface LocalApiRow {
  label: string;
  value: string;
}

export interface ResolvedLoadedModel {
  name: string;
  path: string;
  status: ModelEntry["status"];
}

export function runtimeStateSummary(state: RuntimeState): RuntimeStateSummary {
  switch (state) {
    case "ready":
      return { label: "Engine ready", dotClass: "bg-accent-success" };
    case "starting":
      return { label: "Starting local engine…", dotClass: "bg-accent-warning animate-pulse" };
    case "error":
      return { label: "Engine problem", dotClass: "bg-accent-danger" };
    case "stopped":
    default:
      return { label: "No model running", dotClass: "bg-zinc-600" };
  }
}

export function variantBadge(
  variant: RuntimeVariant | null,
  acceleratorLabel: string | null,
): VariantBadge {
  if (variant === "cuda") {
    return { label: "NVIDIA GPU (CUDA)", tone: "cuda" };
  }
  if (variant === "vulkan") {
    return { label: "GPU acceleration (Vulkan)", tone: "vulkan" };
  }
  if (variant === "cpu") {
    return { label: "CPU mode", tone: "cpu" };
  }
  if (acceleratorLabel) {
    const lower = acceleratorLabel.toLowerCase();
    if (lower.includes("cpu")) {
      return { label: acceleratorLabel, tone: "cpu" };
    }
    if (lower.includes("cuda")) {
      return { label: acceleratorLabel, tone: "cuda" };
    }
    return { label: acceleratorLabel, tone: "vulkan" };
  }
  return { label: "Not running", tone: "none" };
}

export function fallbackExplanation(fallbackReason: string): FallbackExplanation {
  const lower = fallbackReason.toLowerCase();
  const fromCuda = lower.includes("cuda");
  return {
    title: fromCuda ? "Using a fallback accelerator" : "Using CPU mode",
    body: fromCuda
      ? "Omnira tried NVIDIA CUDA first. It switched to another local accelerator so chat can still run on this computer."
      : "Omnira tried GPU acceleration (Vulkan) first. It switched to CPU so chat can still run locally on this computer.",
    technicalDetail: fallbackReason,
  };
}

export function localApiSummary(
  port: number | null,
  state: RuntimeState,
): LocalApiRow[] {
  const active = state === "ready" || state === "starting";
  const binding =
    port != null && active
      ? `127.0.0.1:${port}`
      : "Not bound (start a model to open the local API)";

  return [
    { label: "Binding", value: binding },
    {
      label: "Network access",
      value: "Loopback only — other devices on your network cannot reach this engine",
    },
    {
      label: "Authentication",
      value: "Session key required for each chat request (never stored on disk or shown here)",
    },
  ];
}

export function resolveLoadedModel(
  modelId: string | null,
  models: ModelEntry[],
): ResolvedLoadedModel | null {
  if (!modelId) return null;
  const match = models.find((m) => m.id === modelId);
  if (!match) return null;
  return { name: match.name, path: match.path, status: match.status };
}

export function variantBadgeClass(tone: VariantBadge["tone"]): string {
  switch (tone) {
    case "cuda":
      return "bg-accent-success/15 text-accent-success";
    case "vulkan":
      return "bg-accent-primary/15 text-accent-primary";
    case "cpu":
      return "bg-accent-warning/15 text-accent-warning";
    default:
      return "bg-brand-hover text-brand-textMuted";
  }
}
