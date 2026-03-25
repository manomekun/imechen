import { createSignal, createEffect, onCleanup } from "solid-js";

export type ThemeMode = "dark" | "light" | "system";

const STORAGE_KEY = "imechen-theme";

function getStoredTheme(): ThemeMode {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "dark" || stored === "light" || stored === "system") {
    return stored;
  }
  return "system";
}

function getSystemTheme(): "dark" | "light" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function resolveTheme(mode: ThemeMode): "dark" | "light" {
  return mode === "system" ? getSystemTheme() : mode;
}

const [themeMode, setThemeMode] = createSignal<ThemeMode>(getStoredTheme());
const [resolvedTheme, setResolvedTheme] = createSignal<"dark" | "light">(
  resolveTheme(getStoredTheme()),
);

export function useTheme() {
  createEffect(() => {
    const mode = themeMode();
    localStorage.setItem(STORAGE_KEY, mode);
    const resolved = resolveTheme(mode);
    setResolvedTheme(resolved);
    document.documentElement.setAttribute("data-theme", resolved);
  });

  // Watch OS theme changes
  const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  const handleChange = () => {
    if (themeMode() === "system") {
      const resolved = getSystemTheme();
      setResolvedTheme(resolved);
      document.documentElement.setAttribute("data-theme", resolved);
    }
  };
  mediaQuery.addEventListener("change", handleChange);
  onCleanup(() => mediaQuery.removeEventListener("change", handleChange));

  return { themeMode, setThemeMode, resolvedTheme };
}
