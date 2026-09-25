/**
 * Main-UI status pill: visible copy is "Running locally" plus the engine
 * name. Accelerator names belong in Advanced Diagnostics only
 * (docs/design-principles.md).
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { StatusPill } from "./StatusPill";
import type { RuntimeStatus } from "../lib/ipc";

const acceleratorNames = /Vulkan|CUDA|GPU|CPU mode/i;

function pill(status: RuntimeStatus): string {
  return renderToStaticMarkup(<StatusPill status={status} />);
}

function base(overrides: Partial<RuntimeStatus>): RuntimeStatus {
  return {
    state: "stopped",
    engine_label: null,
    variant: null,
    accelerator_label: null,
    fallback_reason: null,
    model_id: null,
    port: null,
    context_size: null,
    last_error: null,
    ...overrides,
  };
}

describe("StatusPill", () => {
  it("never names an accelerator in the tooltip or label, including when ready on Vulkan", () => {
    const html = pill(
      base({
        state: "ready",
        engine_label: "llama.cpp",
        variant: "vulkan",
        accelerator_label: "GPU (Vulkan)",
      }),
    );
    expect(html).toContain("Running locally");
    expect(html).toContain("llama.cpp");
    expect(html).toContain("title=\"Engine: llama.cpp\"");
    expect(html).not.toMatch(acceleratorNames);
  });

  it("never names CUDA, GPU, or CPU mode across runtime states", () => {
    const states: RuntimeStatus[] = [
      base({
        state: "ready",
        engine_label: "llama.cpp",
        variant: "cuda",
        accelerator_label: "GPU (CUDA)",
      }),
      base({
        state: "ready",
        engine_label: "llama.cpp",
        variant: "cpu",
        accelerator_label: "CPU",
      }),
      base({
        state: "starting",
        engine_label: "llama.cpp",
        variant: "vulkan",
        accelerator_label: "GPU (Vulkan)",
      }),
      base({
        state: "error",
        engine_label: "llama.cpp",
        accelerator_label: "GPU (Vulkan)",
      }),
    ];
    for (const status of states) {
      expect(pill(status)).not.toMatch(acceleratorNames);
    }
  });
});
