import { createI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'

/**
 * Build-time default locale injected by Vite define.
 * Set via DEFAULT_LOCALE env var: DEFAULT_LOCALE=en pnpm build
 */
declare const __DEFAULT_LOCALE__: string | undefined

/**
 * Locale info returned from the backend scan.
 * Mirrors the Rust struct `LocaleInfo`.
 */
export interface LocaleInfo {
  /** Locale code (e.g. "en", "zh-CN") */
  code: string
  /** Native display name (e.g. "English", "简体中文") */
  name: string
  /** English display name (e.g. "English", "Chinese Simplified") */
  english_name: string
}

const STORAGE_KEY = 'rustverse-locale'

/**
 * Use Vite's import.meta.glob to statically discover all locale modules at build time.
 * This ensures all locale files are included in the production bundle regardless of
 * whether the backend can find the locales directory at runtime.
 *
 * The glob pattern matches directories like: en/index.ts, zh-CN/index.ts, etc.
 * import.meta.glob with eager:false returns lazy import functions.
 */
const localeModules = import.meta.glob<{ default: Record<string, unknown> }>('./*/index.ts')

/** Build the list of available locales from the glob results (build-time known). */
function buildLocaleListFromGlob(): LocaleInfo[] {
  const wellKnown: Record<string, { name: string; english_name: string }> = {
    en: { name: 'English', english_name: 'English' },
    'zh-CN': { name: '简体中文', english_name: 'Chinese Simplified' },
    'zh-TW': { name: '繁體中文', english_name: 'Chinese Traditional' },
    ja: { name: '日本語', english_name: 'Japanese' },
    ko: { name: '한국어', english_name: 'Korean' },
    fr: { name: 'Français', english_name: 'French' },
    de: { name: 'Deutsch', english_name: 'German' },
    es: { name: 'Español', english_name: 'Spanish' },
    'pt-BR': { name: 'Português (Brasil)', english_name: 'Portuguese (Brazil)' },
    ru: { name: 'Русский', english_name: 'Russian' },
    it: { name: 'Italiano', english_name: 'Italian' },
    ar: { name: 'العربية', english_name: 'Arabic' },
  }

  const locales: LocaleInfo[] = []
  for (const path of Object.keys(localeModules)) {
    // Extract locale code from path like "./en/index.ts" → "en"
    const match = path.match(/^\.\/(.+?)\/index\.ts$/)
    if (!match) continue
    const code = match[1]
    const info = wellKnown[code] || { name: code, english_name: code }
    locales.push({ code, ...info })
  }
  return locales.sort((a, b) => a.code.localeCompare(b.code))
}

/** In-memory cache of discovered locales (build-time, never empty). */
let _availableLocales: LocaleInfo[] = buildLocaleListFromGlob()

/** Set of locales whose messages have been loaded into vue-i18n. */
const loadedLocales = new Set<string>()

/**
 * Discover available locales — SYNCHRONOUS.
 * Uses the build-time glob list only. No backend calls during startup.
 * This ensures the function NEVER blocks, even in production.
 *
 * Backend enhancement (metadata merge) is handled asynchronously after startup.
 */
export function discoverLocales(): LocaleInfo[] {
  return _availableLocales
}

/**
 * Async version: discover + optionally merge backend metadata.
 * Use this for settings UI refresh, not for startup.
 */
export async function discoverLocalesWithBackend(): Promise<LocaleInfo[]> {
  // Start with build-time list
  discoverLocales()

  // Optionally try to enhance with backend data (fire-and-forget, non-blocking)
  try {
    const backendLocales = await Promise.race([
      invoke<LocaleInfo[]>('list_available_locales', { forceRefresh: false }),
      new Promise<never>((_, reject) => setTimeout(() => reject(new Error('timeout')), 3000)),
    ])
    for (const bl of backendLocales) {
      const existing = _availableLocales.find(l => l.code === bl.code)
      if (existing && bl.name !== bl.code) existing.name = bl.name
    }
  } catch {
    // Backend scan failed — that's fine, we have the build-time list
  }

  return _availableLocales
}

/**
 * Get the currently cached list of available locales.
 * Call discoverLocales() first to populate the cache.
 */
export function getAvailableLocales(): LocaleInfo[] {
  return _availableLocales
}

/**
 * Detect initial locale by priority:
 * 1. localStorage (user explicit choice)
 * 2. Build-time default (__DEFAULT_LOCALE__ — installer language)
 * 3. navigator.language (system locale)
 * 4. Fallback: 'en'
 *
 * NOTE: Backend-persisted locale is checked in initLocale() BEFORE
 * this function, as its highest-priority input.
 */
function detectLocale(): string {
  // Priority 1: localStorage
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored && _availableLocales.some(l => l.code === stored)) {
    return stored
  }

  // Priority 2: build-time default (installer language)
  const buildDefault = typeof __DEFAULT_LOCALE__ === 'string' ? __DEFAULT_LOCALE__ : ''
  if (buildDefault && _availableLocales.some(l => l.code === buildDefault)) {
    return buildDefault
  }

  // Priority 3: navigator.language
  const browserLang = navigator.language
  const matched = _availableLocales.find(l => l.code === browserLang)
  if (matched) return matched.code
  const prefixMatched = _availableLocales.find(l => browserLang.startsWith(l.code.split('-')[0]))
  if (prefixMatched) return prefixMatched.code

  // Priority 4: hardcoded fallback
  return 'en'
}

const i18n = createI18n({
  legacy: false,
  locale: 'en', // Will be updated in initLocale()
  fallbackLocale: 'en',
  messages: {},
})

/**
 * Dynamically load a locale's messages and set it as active.
 * Uses the build-time glob imports (always available in production).
 * Skips loading if already loaded.
 */
export async function loadLocaleAsync(locale: string): Promise<void> {
  if (loadedLocales.has(locale)) {
    i18n.global.locale.value = locale
    return
  }

  // Use the glob-based module loader
  const modulePath = `./${locale}/index.ts`
  const loader = localeModules[modulePath]
  if (!loader) {
    console.warn(`[i18n] No locale module found for: ${locale} (tried ${modulePath})`)
    return
  }

  try {
    const mod = await loader()
    const messages = mod.default
    i18n.global.setLocaleMessage(locale, messages)
    loadedLocales.add(locale)
    i18n.global.locale.value = locale
  } catch (e) {
    console.error(`[i18n] Failed to load locale "${locale}":`, e)
    if (locale !== 'en') {
      await loadLocaleAsync('en')
    }
  }
}

/**
 * Set the active locale with full persistence:
 * - Update vue-i18n runtime
 * - Save to localStorage
 * - Persist to backend ($APPDATA/locale.json) — fire-and-forget
 * - Update document.lang attribute
 */
export async function setLocale(locale: string): Promise<void> {
  await loadLocaleAsync(locale)
  localStorage.setItem(STORAGE_KEY, locale)
  document.documentElement.setAttribute('lang', locale)

  // Persist to backend (non-blocking, ignore errors)
  invoke('set_locale', { locale }).catch(() => {
    /* Backend persistence is optional */
  })
}

/**
 * Get the current active locale code.
 */
export function getLocale(): string {
  return i18n.global.locale.value as string
}

/**
 * Initialize locale on app startup.
 *
 * This function is designed to NEVER block or hang, even if the Tauri
 * backend is not yet ready. All backend-dependent operations are deferred
 * via timeouts.
 *
 * Priority order:
 * 1. Backend-persisted locale (user's explicit choice from previous sessions)
 * 2. localStorage (browser storage)
 * 3. Build-time DEFAULT_LOCALE (installer language, set via Vite define)
 * 4. navigator.language (system locale auto-detect)
 * 5. 'en' hardcoded fallback
 */
export async function initLocale(): Promise<void> {
  // Step 1: Discover available locales (instant — already initialized as module var)
  discoverLocales()

  // Step 2: Eagerly load the fallback locale
  await loadLocaleAsync('en')

  // Step 3: Try backend-persisted locale (timeout-safe, non-blocking)
  let locale: string

  try {
    const persisted = await Promise.race([
      invoke<string>('get_locale'),
      new Promise<never>((_, reject) => setTimeout(() => reject(new Error('timeout')), 2000)),
    ])
    if (persisted && persisted !== 'en' && _availableLocales.some(l => l.code === persisted)) {
      // Backend had a non-default locale — user explicitly chose it previously
      locale = persisted
    } else {
      locale = detectLocale()
    }
  } catch {
    // Backend not ready or timeout — fall through to detectLocale()
    locale = detectLocale()
  }

  // Step 4: Load and apply the detected locale
  await loadLocaleAsync(locale)
  document.documentElement.setAttribute('lang', locale)
}

export default i18n
