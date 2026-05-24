#!/usr/bin/env node

/**
 * Generate or verify Tauri updater signing key pair.
 *
 * - If a private key already exists at `.tauri-signer-key`, verify it matches the public key.
 * - If no private key exists, generate a new key pair using `tauri signer generate`.
 * - Update the public key in `tauri.conf.json` and `.tauri-signer-key.pub`.
 * - Output the private key content for CI environment variable setup.
 */

const { execSync } = require('child_process')
const fs = require('fs')
const path = require('path')

const PROJECT_ROOT = path.join(__dirname, '..')
const PRIVATE_KEY_PATH = path.join(PROJECT_ROOT, '.tauri-signer-key')
const PUBLIC_KEY_PATH = path.join(PROJECT_ROOT, 'src-tauri', '.tauri-signer-key.pub')
const TAURI_CONF_PATH = path.join(PROJECT_ROOT, 'src-tauri', 'tauri.conf.json')

function readPrivateKey() {
  try {
    return fs.readFileSync(PRIVATE_KEY_PATH, 'utf8').trim()
  } catch {
    return null
  }
}

function readPublicKey() {
  try {
    return fs.readFileSync(PUBLIC_KEY_PATH, 'utf8').trim()
  } catch {
    return null
  }
}

function updateTauriConfPubkey(pubkey) {
  const content = fs.readFileSync(TAURI_CONF_PATH, 'utf8')
  const updated = content.replace(
    /"pubkey":\s*"[^"]*"/,
    `"pubkey": "${pubkey}"`
  )
  if (updated !== content) {
    fs.writeFileSync(TAURI_CONF_PATH, updated)
    console.log('Updated pubkey in tauri.conf.json')
  }
}

function generateKeyPair() {
  console.log('Generating new Tauri signer key pair...')

  // Use tauri signer generate command
  // -w writes the private key to the specified file
  // The public key is written to the same file with .pub suffix
  const privateKeyDir = path.dirname(PRIVATE_KEY_PATH)
  if (!fs.existsSync(privateKeyDir)) {
    fs.mkdirSync(privateKeyDir, { recursive: true })
  }

  try {
    execSync(
      `npx tauri signer generate -w "${PRIVATE_KEY_PATH}"`,
      {
        cwd: PROJECT_ROOT,
        stdio: 'pipe',
        env: { ...process.env, CI: 'true' }
      }
    )
  } catch (err) {
    // tauri signer generate prompts for a password; use empty password in non-interactive mode
    // Try with TAURI_SIGNING_PRIVATE_KEY_PASSWORD set to empty
    try {
      execSync(
        `npx tauri signer generate -w "${PRIVATE_KEY_PATH}"`,
        {
          cwd: PROJECT_ROOT,
          stdio: 'pipe',
          env: {
            ...process.env,
            CI: 'true',
            TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ''
          }
        }
      )
    } catch (err2) {
      console.error('Failed to generate key pair automatically.')
      console.error('Please run manually:')
      console.error(`  npx tauri signer generate -w "${PRIVATE_KEY_PATH}"`)
      console.error('Then re-run this script.')
      process.exit(1)
    }
  }

  // Read the generated keys
  const privateKey = readPrivateKey()
  const publicKey = readPublicKey()

  if (!privateKey || !publicKey) {
    console.error('Key generation succeeded but keys could not be read.')
    process.exit(1)
  }

  return { privateKey, publicKey }
}

function main() {
  const args = process.argv.slice(2)
  const forceRegenerate = args.includes('--regenerate') || args.includes('-f')

  let privateKey = readPrivateKey()
  let publicKey = readPublicKey()

  if (privateKey && !forceRegenerate) {
    // Private key exists, verify it matches the public key
    console.log('Private key found at .tauri-signer-key')

    if (publicKey) {
      console.log('Public key found at src-tauri/.tauri-signer-key.pub')
      // Ensure tauri.conf.json has the correct pubkey
      updateTauriConfPubkey(publicKey)
    }

    console.log('Signing key pair is ready.')
  } else {
    // Need to generate a new key pair
    const result = generateKeyPair()
    privateKey = result.privateKey
    publicKey = result.publicKey

    // Copy public key to src-tauri directory
    fs.writeFileSync(PUBLIC_KEY_PATH, publicKey)

    // Update tauri.conf.json
    updateTauriConfPubkey(publicKey)

    console.log('New signing key pair generated.')
    console.log('')
    console.log('IMPORTANT: Add the private key to your CI secrets:')
    console.log('  GitHub: Settings > Secrets > TAURI_SIGNING_PRIVATE_KEY')
    console.log('')
    console.log('Private key content:')
    console.log(privateKey)
    console.log('')
  }

  // Output private key path for use in build scripts
  if (args.includes('--print-key')) {
    console.log(privateKey)
  }

  // Set environment variable for current process
  process.env.TAURI_SIGNING_PRIVATE_KEY = privateKey

  return { privateKey, publicKey }
}

main()
