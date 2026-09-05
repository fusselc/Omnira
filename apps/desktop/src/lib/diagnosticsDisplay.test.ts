import { describe, expect, it } from "vitest";
import {
  engineSummary,
  fallbackExplanation,
  prefersCpuFromEarlierLaunch,
  variantBadge,
  variantSelectionOrder,
  VULKAN_SKIPPED_PREFIX,
} from "./diagnosticsDisplay";

describe("fallbackExplanation", () => {
  it("explains a real Vulkan failure and keeps the engine detail", () => {
    const reason = "Vulkan unavailable: llama-server exited during startup: exit code: 1; stderr: ggml_vulkan: No devices found";
    const explanation = fallbackExplanation(reason);
    expect(explanation.title).toBe("Using CPU mode");
    expect(explanation.technicalDetail).toBe(reason);
  });

  it("distinguishes a remembered CPU preference from a failed GPU attempt", () => {
    const reason = `${VULKAN_SKIPPED_PREFIX} CPU was recorded as the working runtime on an earlier launch.`;
    const explanation = fallbackExplanation(reason);
    expect(explanation.title).toContain("remembered");
    expect(explanation.body).toContain("was not attempted");
    expect(explanation.technicalDetail).toBe(reason);
  });
});

describe("engine display", () => {
  it("names the engine while starting and running, and nothing when idle", () => {
    expect(engineSummary(null, "stopped")).toBe("None loaded");
    expect(engineSummary("llama.cpp", "starting")).toBe("llama.cpp (starting)");
    expect(engineSummary("llama.cpp", "ready")).toBe("llama.cpp");
  });

  it("labels the accelerator honestly", () => {
    expect(variantBadge("cuda", "GPU (CUDA)")).toEqual({
      label: "GPU acceleration (CUDA)",
      tone: "cuda",
    });
    expect(variantBadge("vulkan", "GPU (Vulkan)").label).toBe("GPU acceleration (Vulkan)");
    expect(variantBadge("cpu", "CPU").label).toBe("CPU mode");
    expect(variantBadge(null, null).label).toBe("Not running");
  });

  it("does not treat every non-CPU accelerator label as Vulkan", () => {
    expect(variantBadge(null, "GPU (CUDA)").tone).toBe("cuda");
    expect(variantBadge(null, "GPU (Vulkan)").tone).toBe("vulkan");
    expect(variantBadge(null, "CPU").tone).toBe("cpu");
    expect(variantBadge(null, "Unknown accelerator").tone).toBe("none");
  });

  it("only offers the GPU retry when CPU is the remembered preference", () => {
    expect(prefersCpuFromEarlierLaunch("cpu")).toBe(true);
    expect(prefersCpuFromEarlierLaunch("vulkan")).toBe(false);
    expect(prefersCpuFromEarlierLaunch("cuda")).toBe(false);
    expect(prefersCpuFromEarlierLaunch(null)).toBe(false);
  });
});

describe("variantSelectionOrder", () => {
  it("uses CUDA → Vulkan → CPU when a CUDA binary is present", () => {
    expect(variantSelectionOrder(null, true)).toEqual(["cuda", "vulkan", "cpu"]);
    expect(variantSelectionOrder("vulkan", true)).toEqual(["cuda", "vulkan", "cpu"]);
  });

  it("keeps Vulkan → CPU when no CUDA binary is present", () => {
    expect(variantSelectionOrder(null, false)).toEqual(["vulkan", "cpu"]);
    expect(variantSelectionOrder("cuda", false)).toEqual(["vulkan", "cpu"]);
  });

  it("starts on CPU when that is preferred, then retries higher-priority GPUs after clear", () => {
    expect(variantSelectionOrder("cpu", true)).toEqual(["cpu", "cuda", "vulkan"]);
    expect(variantSelectionOrder("cpu", false)).toEqual(["cpu", "vulkan"]);
    expect(variantSelectionOrder(null, true)[0]).toBe("cuda");
  });
});
