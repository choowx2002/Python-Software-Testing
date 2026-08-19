import { createI18n } from "vue-i18n";
import en from "./locales/en";
import zh from "./locales/zh";

const savedLocale = localStorage.getItem("testmate-locale");

const i18n = createI18n({
  legacy: false,
  locale: savedLocale === "zh" ? "zh" : "en",
  fallbackLocale: "en",
  messages: { en, zh },
});

export default i18n;

export function setLocale(locale: "en" | "zh") {
  i18n.global.locale.value = locale;
  localStorage.setItem("testmate-locale", locale);
}
