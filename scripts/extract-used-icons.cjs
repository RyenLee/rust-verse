#!/usr/bin/env node

const fs = require('fs')
const path = require('path')

/**
 * Extract used MDI icons from .vue and .ts files,
 * then create a minimal icons JSON containing only those icons.
 */

const SRC_DIR = path.join(process.cwd(), 'src')
const OUTPUT_FILE = path.join(SRC_DIR, 'assets', 'used-icons.json')
const FULL_ICONS_PATH = path.join(process.cwd(), 'node_modules', '@iconify-json', 'mdi', 'icons.json')

// Matches any string of the form mdi:icon-name (in Vue templates / JS strings)
const MDI_REGEX = /mdi:([a-z0-9]+(?:-[a-z0-9]+)*)/g

/**
 * Recursively walk a directory and collect .vue / .ts files.
 */
function walk(dir, extensions) {
  const results = []
  const entries = fs.readdirSync(dir, { withFileTypes: true })
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      results.push(...walk(fullPath, extensions))
    } else if (extensions.some(ext => entry.name.endsWith(ext))) {
      results.push(fullPath)
    }
  }
  return results
}

/**
 * Extract unique mdi icon names from a set of files.
 */
function extractIcons(files) {
  const icons = new Set()
  for (const file of files) {
    const content = fs.readFileSync(file, 'utf8')
    let match
    while ((match = MDI_REGEX.exec(content)) !== null) {
      icons.add(match[1])
    }
  }
  return [...icons].sort()
}

/**
 * Load the full MDI icons.json.
 */
function loadFullIcons() {
  if (!fs.existsSync(FULL_ICONS_PATH)) {
    console.error(`ERROR: Full icons file not found at ${FULL_ICONS_PATH}`)
    process.exit(1)
  }
  return JSON.parse(fs.readFileSync(FULL_ICONS_PATH, 'utf8'))
}

// ── Main ──────────────────────────────────────────────────────────────────────

console.log('Scanning .vue and .ts files in src/ ...')
const files = walk(SRC_DIR, ['.vue', '.ts'])
console.log(`Found ${files.length} .vue/.ts files.`)

const usedIcons = extractIcons(files)
console.log(`Found ${usedIcons.length} unique MDI icon references:`)
usedIcons.forEach(name => console.log(`  - ${name}`))

console.log(`\nLoading full MDI icons from ${FULL_ICONS_PATH} ...`)
const fullData = loadFullIcons()
console.log(`Full icons.json contains ${Object.keys(fullData.icons).length} icons.`)

// Build minimal output
const missing = []
const output = {
  prefix: fullData.prefix || 'mdi',
  icons: {},
  width: fullData.width || 24,
  height: fullData.height || 24,
}

for (const name of usedIcons) {
  if (fullData.icons[name]) {
    output.icons[name] = fullData.icons[name]
  } else {
    missing.push(name)
  }
}

if (missing.length > 0) {
  console.warn(`\nWARNING: ${missing.length} icon(s) not found in MDI collection:`)
  missing.forEach(name => console.warn(`  - ${name}`))
}

// Ensure output directory exists
const outDir = path.dirname(OUTPUT_FILE)
if (!fs.existsSync(outDir)) {
  fs.mkdirSync(outDir, { recursive: true })
}

fs.writeFileSync(OUTPUT_FILE, JSON.stringify(output, null, '\t') + '\n')
console.log(`\nWrote ${OUTPUT_FILE} with ${Object.keys(output.icons).length} icons.`)

// Size comparison
const originalSize = fs.statSync(FULL_ICONS_PATH).size
const newSize = fs.statSync(OUTPUT_FILE).size
console.log(`\nSize comparison:`)
console.log(`  Original icons.json : ${(originalSize / 1024).toFixed(1)} KB`)
console.log(`  Used-icons.json     : ${(newSize / 1024).toFixed(1)} KB`)
console.log(`  Reduction           : ${((1 - newSize / originalSize) * 100).toFixed(1)}%`)