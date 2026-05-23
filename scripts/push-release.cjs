#!/usr/bin/env node

const { execSync, spawnSync } = require('child_process')
const fs = require('fs')
const path = require('path')

function readVersion() {
  const pkgPath = path.join(process.cwd(), 'package.json')
  const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'))
  return pkg.version
}

function run(cmd, dryRun) {
  if (dryRun) {
    console.log(`[dry-run] ${cmd}`)
    return
  }
  console.log(`> ${cmd}`)
  execSync(cmd, { stdio: 'inherit' })
}

function runBumpVersion(newVersion, dryRun) {
  const scriptPath = path.join(process.cwd(), 'scripts', 'bump-version.cjs')
  const args = newVersion ? [scriptPath, newVersion] : [scriptPath]

  if (dryRun) {
    console.log(`[dry-run] node ${args.join(' ')}`)
    return
  }

  console.log(`> node ${args.join(' ')}`)
  const result = spawnSync('node', args, { stdio: 'inherit' })
  if (result.status !== 0) {
    console.error('bump-version.cjs failed')
    process.exit(1)
  }
}

function main() {
  const args = process.argv.slice(2)
  const dryRun = args.includes('--dry-run') || args.includes('-n')
  const pushTag = args.includes('--tag') || args.includes('-t')
  const pushMain = args.includes('--main') || args.includes('-m')
  const skipBump = args.includes('--skip-bump')
  const all = !pushTag && !pushMain

  // Extract version argument (first non-flag arg)
  const newVersion = args.find(a => !a.startsWith('-'))

  // Step 1: Run bump-version.cjs
  if (!skipBump) {
    console.log('=== Running bump-version ===')
    runBumpVersion(newVersion, dryRun)
    console.log()
  }

  const version = readVersion()
  const tag = `v${version}`

  console.log(`Version: ${version}`)
  console.log(`Tag: ${tag}`)
  console.log()

  // Step 2: Push to GitHub
  if (all || pushMain) {
    run('git push origin main', dryRun)
  }

  if (all || pushTag) {
    const tagExists = (() => {
      try {
        execSync(`git rev-parse ${tag}`, { stdio: 'pipe' })
        return true
      } catch {
        return false
      }
    })()

    if (tagExists) {
      console.log(`Tag ${tag} already exists, pushing...`)
      run(`git push origin ${tag}`, dryRun)
    } else {
      console.log(`Creating tag ${tag}...`)
      run(`git tag ${tag}`, dryRun)
      run(`git push origin ${tag}`, dryRun)
    }
  }

  console.log()
  console.log('Done! Check GitHub Actions: https://github.com/RyenLee/rust-verse/actions')
}

main()
