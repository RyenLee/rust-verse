#!/usr/bin/env node
/**
 * Build-time locale config generator (CommonJS).
 *
 * Scans `src/locales/` at build time, validates each locale code (BCP 47),
 * checks for required index.ts file, and writes the validated locale list
 * to `src-tauri/config.toml` under the `[locale]` section.
 *
 * Invalid locale entries in config.toml are automatically removed.
 *
 * Run automatically via `beforeBuildCommand` in tauri.conf.json.
 */

const fs = require('fs');
const path = require('path');

// BCP 47 locale code validator
function isValidLocaleCode(code) {
  if (!code || typeof code !== 'string') return false;
  const parts = code.split('-');
  if (parts.length === 0 || parts.length > 3) return false;

  // Language: 2-3 lowercase letters
  const lang = parts[0];
  if (lang.length < 2 || lang.length > 3 || !/^[a-z]{2,3}$/.test(lang)) {
    return false;
  }

  // Script (optional): 4 titlecase letters (e.g., 'Hans' in zh-Hans)
  if (parts.length >= 2) {
    const scriptOrRegion = parts[1];
    // Script: 4 titlecase letters
    const isScript = /^[A-Z][a-z]{3}$/.test(scriptOrRegion);
    // Region: 2 uppercase letters
    const isRegion = /^[A-Z]{2}$/.test(scriptOrRegion);
    if (!isScript && !isRegion) {
      return false;
    }
  }

  // Region (optional for 3-part): 2 uppercase letters or 3 digits
  if (parts.length === 3) {
    const region = parts[2];
    if (!(/^[A-Z]{2}$/.test(region) || /^\d{3}$/.test(region))) {
      return false;
    }
  }

  return true;
}

// Read metadata.json for display names
function readMetadata(localeDir) {
  const metaPath = path.join(localeDir, 'metadata.json');
  if (fs.existsSync(metaPath)) {
    try {
      return JSON.parse(fs.readFileSync(metaPath, 'utf-8'));
    } catch {
      // Ignore parse errors
    }
  }
  return null;
}

// Well-known locale display names
const WELL_KNOWN = {
  en: { name: 'English', english_name: 'English' },
  'zh-CN': { name: '简体中文', english_name: 'Chinese Simplified' },
  'zh-TW': { name: '繁體中文', english_name: 'Chinese Traditional' },
  'zh-Hans': { name: '简体中文', english_name: 'Chinese Simplified (Hans)' },
  'zh-Hant': { name: '繁體中文', english_name: 'Chinese Traditional (Hant)' },
  ja: { name: '日本語', english_name: 'Japanese' },
  ko: { name: '한국어', english_name: 'Korean' },
  fr: { name: 'Français', english_name: 'French' },
  de: { name: 'Deutsch', english_name: 'German' },
  es: { name: 'Español', english_name: 'Spanish' },
  'pt-BR': { name: 'Português (Brasil)', english_name: 'Portuguese (Brazil)' },
  'pt-PT': { name: 'Português', english_name: 'Portuguese' },
  ru: { name: 'Русский', english_name: 'Russian' },
  it: { name: 'Italiano', english_name: 'Italian' },
  ar: { name: 'العربية', english_name: 'Arabic' },
  hi: { name: 'हिन्दी', english_name: 'Hindi' },
  th: { name: 'ภาษาไทย', english_name: 'Thai' },
  vi: { name: 'Tiếng Việt', english_name: 'Vietnamese' },
  id: { name: 'Bahasa Indonesia', english_name: 'Indonesian' },
  ms: { name: 'Bahasa Melayu', english_name: 'Malay' },
  tr: { name: 'Türkçe', english_name: 'Turkish' },
  pl: { name: 'Polski', english_name: 'Polish' },
  nl: { name: 'Nederlands', english_name: 'Dutch' },
  uk: { name: 'Українська', english_name: 'Ukrainian' },
};

function getLocaleDisplayName(code) {
  if (WELL_KNOWN[code]) {
    return WELL_KNOWN[code];
  }
  return { name: code, english_name: code };
}

function scanLocales(localesSrcDir) {
  if (!fs.existsSync(localesSrcDir)) {
    console.warn(`[locale-config] Locales directory not found: ${localesSrcDir}`);
    return [];
  }

  const entries = fs.readdirSync(localesSrcDir, { withFileTypes: true });
  const locales = [];

  for (const entry of entries) {
    if (!entry.isDirectory()) continue;

    const code = entry.name;
    const localeDir = path.join(localesSrcDir, code);

    // Validate BCP 47 code
    if (!isValidLocaleCode(code)) {
      console.warn(`[locale-config] Skipping '${code}' - invalid BCP 47 locale code`);
      continue;
    }

    // Check for required index.ts
    const indexFile = path.join(localeDir, 'index.ts');
    if (!fs.existsSync(indexFile)) {
      console.warn(`[locale-config] Skipping '${code}' - missing index.ts`);
      continue;
    }

    // Get display names
    const meta = readMetadata(localeDir);
    const display = meta
      ? { name: meta.name || code, english_name: meta.english_name || code }
      : getLocaleDisplayName(code);

    locales.push({
      code,
      name: display.name,
      english_name: display.english_name,
    });
  }

  // Sort by code
  locales.sort((a, b) => a.code.localeCompare(b.code));

  return locales;
}

function updateConfigToml(configPath, locales) {
  if (!fs.existsSync(configPath)) {
    console.error(`[locale-config] config.toml not found at: ${configPath}`);
    process.exit(1);
  }

  let configContent = fs.readFileSync(configPath, 'utf-8');
  const lines = configContent.split('\n');
  const newLines = [];
  let inLocaleSection = false;
  let localeSectionIndex = -1;
  let i = 0;

  // Find [locale] section
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim() === '[locale]') {
      inLocaleSection = true;
      localeSectionIndex = newLines.length;
      newLines.push(line);
      i++;
      continue;
    }

    if (inLocaleSection) {
      // Check if we hit another section
      if (line.trim().startsWith('[') && !line.trim().startsWith('[locale.')) {
        inLocaleSection = false;
        // Insert new locale section content before this new section
        newLines.push(`codes = ${JSON.stringify(locales.map(l => l.code))}`);
        newLines.push('');
        if (locales.length > 0) {
          newLines.push('# Locale metadata (auto-generated, do not edit manually)');
          for (const l of locales) {
            newLines.push(`[locale.meta."${l.code}"]`);
            newLines.push(`  name = ${JSON.stringify(l.name)}`);
            newLines.push(`  english_name = ${JSON.stringify(l.english_name)}`);
          }
        }
        newLines.push('');
        // Don't i++ here, let the loop process this line as part of the new section
        continue;
      }

      // Skip old locale section content (codes, meta entries)
      if (line.trim().startsWith('codes') || line.trim().startsWith('# Locale metadata') || line.trim().startsWith('[locale.meta')) {
        i++;
        continue;
      }

      // Empty line or other content, keep it but check if we're done with locale section
      if (line.trim() === '' || line.startsWith('#') || line.startsWith(' ')) {
        // Check if next non-empty line is a new section
        let nextNonEmpty = i + 1;
        while (nextNonEmpty < lines.length && lines[nextNonEmpty].trim() === '') nextNonEmpty++;
        if (nextNonEmpty < lines.length && lines[nextNonEmpty].trim().startsWith('[') && !lines[nextNonEmpty].trim().startsWith('[locale.')) {
          inLocaleSection = false;
          newLines.push(`codes = ${JSON.stringify(locales.map(l => l.code))}`);
          newLines.push('');
          if (locales.length > 0) {
            newLines.push('# Locale metadata (auto-generated, do not edit manually)');
            for (const l of locales) {
              newLines.push(`[locale.meta."${l.code}"]`);
              newLines.push(`  name = ${JSON.stringify(l.name)}`);
              newLines.push(`  english_name = ${JSON.stringify(l.english_name)}`);
            }
          }
          newLines.push('');
        }
        i++;
        continue;
      }

      i++;
      continue;
    }

    newLines.push(line);
    i++;
  }

  // If no [locale] section found, append it
  if (localeSectionIndex === -1) {
    newLines.push('');
    newLines.push('[locale]');
    newLines.push(`codes = ${JSON.stringify(locales.map(l => l.code))}`);
    newLines.push('');
    if (locales.length > 0) {
      newLines.push('# Locale metadata (auto-generated, do not edit manually)');
      for (const l of locales) {
        newLines.push(`[locale.meta."${l.code}"]`);
        newLines.push(`  name = ${JSON.stringify(l.name)}`);
        newLines.push(`  english_name = ${JSON.stringify(l.english_name)}`);
      }
    }
  }

  configContent = newLines.join('\n');
  fs.writeFileSync(configPath, configContent, 'utf-8');
  console.log(`[locale-config] Updated ${configPath} with ${locales.length} locales`);
}

function main() {
  // Resolve paths relative to project root
  const projectRoot = path.resolve(__dirname, '..');
  const localesSrcDir = path.join(projectRoot, 'src', 'locales');
  const configPath = path.join(projectRoot, 'src-tauri', 'config.toml');

  console.log('[locale-config] Scanning locales from:', localesSrcDir);
  const locales = scanLocales(localesSrcDir);

  if (locales.length === 0) {
    console.warn('[locale-config] No valid locales found!');
    process.exit(1);
  }

  console.log(`[locale-config] Found ${locales.length} valid locales:`);
  for (const l of locales) {
    console.log(`  - ${l.code}: ${l.name} (${l.english_name})`);
  }

  updateConfigToml(configPath, locales);
  console.log('[locale-config] Done!');
}

main();
