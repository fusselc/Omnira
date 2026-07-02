import { open } from "@tauri-apps/plugin-dialog";
import { isMockMode } from "./mock";

/** GGUF file picker; returns a canned path in browser mock mode. */
export async function pickGgufFile(): Promise<string | null> {
  if (isMockMode) {
    return "C:\\Models\\example-model-q4_k_m.gguf";
  }
  const selected = await open({
    multiple: false,
    title: "Choose a GGUF model file",
    filters: [{ name: "GGUF models", extensions: ["gguf"] }],
  });
  return typeof selected === "string" ? selected : null;
}
