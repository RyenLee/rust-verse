#!/usr/bin/env node

const fs = require('fs').promises
const fsSync = require('fs')
const path = require('path')

/**
 * Read the version from src-tauri/config.toml [app] section.
 * This is the canonical version source.
 */
async function readVersionFromConfigToml() {
  const configTomlPath = path.join(process.cwd(), 'src-tauri', 'config.toml')
  let content
  try {
    content = await fs.readFile(configTomlPath, 'utf8')
  } catch (error) {
    if (error.code === 'ENOENT') {
      console.error('Error: src-tauri/config.toml not found')
      process.exit(1)
    }
    throw error
  }

  // Parse TOML manually for the [app] section
  // Supports: [app], [app.key], multi-line tables
  const lines = content.split('\n')
  let inAppSection = false

  for (const line of lines) {
    const trimmed = line.trim()

    // Detect [app] section start
    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
      const section = trimmed.slice(1, -1).split('.')[0]
      inAppSection = section === 'app'
      continue
    }

    if (inAppSection) {
      // Exit on next section
      if (trimmed.startsWith('[')) break

      const match = trimmed.match(/^version\s*=\s*"([^"]*)"/)
      if (match) {
        return match[1]
      }
    }
  }

  console.error('Error: Could not find app.version in src-tauri/config.toml')
  process.exit(1)
}

async function bumpVersion() {
  const version = process.argv[2]

  if (version) {
    // If a version argument is provided, update config.toml [app].version first
    const configTomlPath = path.join(process.cwd(), 'src-tauri', 'config.toml')
    let configContent = await fs.readFile(configTomlPath, 'utf8')

    // Replace version in [app] section
    configContent = configContent.replace(
      /^(version\s*=\s*)"[^"]*"/m,
      `$1"${version}"`
    )
    await fs.writeFile(configTomlPath, configContent)
    console.log(`Updated config.toml [app].version to ${version}`)
  }

  // Read the canonical version from config.toml
  const canonicalVersion = await readVersionFromConfigToml()
  console.log(`Canonical version from config.toml: ${canonicalVersion}`)

  // Function to update file if it exists
  async function updateFile(filename, searchPattern, replacement) {
    try {
      const filePath = path.join(process.cwd(), filename)
      const fileContent = await fs.readFile(filePath, 'utf8')

      const updatedContent = fileContent.replace(searchPattern, replacement)

      await fs.writeFile(filePath, updatedContent)
      console.log(`Updated ${filename} version to ${canonicalVersion}`)
    } catch (error) {
      if (error.code === 'ENOENT') {
        console.log(`Warning: ${filename} not found`)
      } else {
        console.error(`Error updating ${filename}:`, error.message)
      }
    }
  }

  // Update package.json
  await updateFile(
    'package.json',
    /"version":\s*"[^"]*"/,
    `"version": "${canonicalVersion}"`
  )

  // Update src-tauri/tauri.conf.json
  await updateFile(
    'src-tauri/tauri.conf.json',
    /"version":\s*"[^"]*"/,
    `"version": "${canonicalVersion}"`
  )

  // Update src-tauri/Cargo.toml
  await updateFile(
    'src-tauri/Cargo.toml',
    /^version\s*=\s*"[^"]*"/m,
    `version = "${canonicalVersion}"`
  )

  console.log(`\nAll files synced to version ${canonicalVersion}`)
}

bumpVersion().catch(console.error)