/**
 * Theme application. Settings stores the theme as a free-form string; the UI
 * only knows "dark" and "light". The class is set on the root element so the
 * CSS variables in index.css (and every `.light-theme ...` override) apply to
 * the whole document, and `color-scheme` follows so native controls and
 * scrollbars match.
 */
export type Theme = "dark" | "light";

export const LIGHT_THEME_CLASS = "light-theme";

export function resolveTheme(value: string | null | undefined): Theme {
  return value === "light" ? "light" : "dark";
}

/** The subset of `HTMLElement` the theme needs; keeps this testable without a DOM. */
export interface ThemeTarget {
  classList: { toggle(token: string, force?: boolean): boolean };
  style: { colorScheme: string };
}

export function applyTheme(
  target: ThemeTarget,
  value: string | null | undefined,
): Theme {
  const theme = resolveTheme(value);
  target.classList.toggle(LIGHT_THEME_CLASS, theme === "light");
  target.style.colorScheme = theme;
  return theme;
}
