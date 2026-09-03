import { describe, expect, it } from "vitest";
import { applyTheme, LIGHT_THEME_CLASS, resolveTheme, type ThemeTarget } from "./theme";

function fakeRoot(): ThemeTarget & { classes: Set<string> } {
  const classes = new Set<string>();
  return {
    classes,
    style: { colorScheme: "" },
    classList: {
      toggle(token: string, force?: boolean) {
        const on = force ?? !classes.has(token);
        if (on) classes.add(token);
        else classes.delete(token);
        return on;
      },
    },
  };
}

describe("resolveTheme", () => {
  it("treats anything other than 'light' as dark", () => {
    expect(resolveTheme("light")).toBe("light");
    expect(resolveTheme("dark")).toBe("dark");
    expect(resolveTheme("")).toBe("dark");
    expect(resolveTheme(null)).toBe("dark");
    expect(resolveTheme(undefined)).toBe("dark");
    expect(resolveTheme("solarized")).toBe("dark");
  });
});

describe("applyTheme", () => {
  it("adds the light class and color-scheme when switching to light", () => {
    const root = fakeRoot();
    expect(applyTheme(root, "light")).toBe("light");
    expect(root.classes.has(LIGHT_THEME_CLASS)).toBe(true);
    expect(root.style.colorScheme).toBe("light");
  });

  it("removes the light class again when switching back to dark", () => {
    const root = fakeRoot();
    applyTheme(root, "light");
    expect(applyTheme(root, "dark")).toBe("dark");
    expect(root.classes.has(LIGHT_THEME_CLASS)).toBe(false);
    expect(root.style.colorScheme).toBe("dark");
  });

  it("is idempotent for repeated applications of the same theme", () => {
    const root = fakeRoot();
    applyTheme(root, "light");
    applyTheme(root, "light");
    expect(root.classes.has(LIGHT_THEME_CLASS)).toBe(true);
    applyTheme(root, "dark");
    applyTheme(root, "dark");
    expect(root.classes.has(LIGHT_THEME_CLASS)).toBe(false);
  });
});
