import { defineStore } from "pinia";
import { i18n } from "@/i18n";

export type AppTheme = "dark" | "light";
export type AppLocale = "zh" | "en";

const THEME_STORAGE_KEY = "zeroclawx.theme";
const LOCALE_STORAGE_KEY = "zeroclawx.locale";

function readStoredTheme(): AppTheme {
  if (typeof window === "undefined") {
    return "dark";
  }

  return window.localStorage.getItem(THEME_STORAGE_KEY) === "light" ? "light" : "dark";
}

function readStoredLocale(): AppLocale {
  if (typeof window === "undefined") {
    return "zh";
  }

  return window.localStorage.getItem(LOCALE_STORAGE_KEY) === "en" ? "en" : "zh";
}

function persistPreference(key: string, value: string) {
  if (typeof window === "undefined") {
    return;
  }

  window.localStorage.setItem(key, value);
}

export const useAppStore = defineStore("app", {
  state: () => ({
    theme: readStoredTheme() as AppTheme,
    locale: readStoredLocale() as AppLocale
  }),
  actions: {
    applyTheme(theme: AppTheme) {
      this.theme = theme;
      document.documentElement.dataset.theme = theme;
      persistPreference(THEME_STORAGE_KEY, theme);
    },
    toggleTheme() {
      this.applyTheme(this.theme === "dark" ? "light" : "dark");
    },
    setLocale(locale: AppLocale) {
      this.locale = locale;
      i18n.global.locale.value = locale;
      document.documentElement.lang = locale;
      persistPreference(LOCALE_STORAGE_KEY, locale);
    }
  }
});
