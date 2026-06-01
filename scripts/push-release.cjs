#!/usr/bin/env node

const { execSync, spawnSync } = require('child_process')
const fs = require('fs')
const path = require('path')

function readVersion() {
  const pkgPath = path.join(process.cwd(), 'package.json')
  const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'))
  return pkg.version
}

function readChangesForVersion(version) {
  const changesPath = path.join(process.cwd(), 'CHANGES.md')
  try {
    const content = fs.readFileSync(changesPath, 'utf8')
    const lines = content.split('\n')
    const result = []
    let inVersion = false
    let bracketCount = 0

    for (const line of lines) {
      if (line.match(/^## \[.*\]/)) {
        if (inVersion) break
        if (line.includes(`[${version}]`)) {
          inVersion = true
          continue
        }
      }
      if (inVersion) {
        result.push(line)
      }
    }

    const changes = result.join('\n').trim()
    return changes || `Release v${version}`
  } catch {
    return `Release v${version}`
  }
}

function run(cmd, dryRun) {
  if (dryRun) {
    console.log(`[dry-run] ${cmd}`)
    return
  }
  console.log(`> ${cmd}`)
  execSync(cmd, { stdio: 'inherit' })
}

function runAllowFail(cmd, dryRun) {
  try {
    run(cmd, dryRun)
  } catch {
    // non-fatal
  }
}

function hasStagedChanges() {
  try {
    execSync('git diff --cached --quiet', { stdio: 'pipe' })
    return false
  } catch {
    return true
  }
}

function hasUnstagedChanges() {
  try {
    execSync('git diff --quiet', { stdio: 'pipe' })
    return false
  } catch {
    return true
  }
}

function getStashCount() {
  try {
    const out = execSync('git stash list', { stdio: 'pipe' }).toString().trim()
    return out ? out.split('\n').length : 0
  } catch {
    return 0
  }
}

function getModifiedFiles() {
  try {
    const out = execSync('git diff --name-only', { stdio: 'pipe' }).toString().trim()
    return out ? out.split('\n').filter(Boolean) : []
  } catch {
    return []
  }
}

function restoreModifiedFiles(files, dryRun) {
  if (files.length === 0) return
  for (const file of files) {
    run(`git checkout -- "${file}"`, dryRun)
  }
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

  const newVersion = args.find(a => !a.startsWith('-'))

  const beforeStash = getStashCount()
  const hadUnstaged = hasUnstagedChanges()

  try {
    if (hadUnstaged) {
      console.log('=== Stashing unstaged changes ===')
      run('git stash push -m "wip: pre-release stash"', dryRun)
    }
    console.log('=== Pulling latest code ===')
    run('git pull --rebase origin main', dryRun)
    if (hadUnstaged) {
      console.log('=== Restoring stashed changes ===')
      // After pull, CRLF/LF normalization may cause files to appear modified,
      // which blocks git stash pop. Discard those phantom changes first.
      const phantomFiles = getModifiedFiles()
      if (phantomFiles.length > 0) {
        console.log('Discarding CRLF/LF phantom changes before stash pop...')
        restoreModifiedFiles(phantomFiles, dryRun)
      }
      runAllowFail('git stash pop', dryRun)
    }
    console.log()

    // Discard CRLF/LF line-ending noise that may block commits
    console.log('=== Normalizing line endings ===')
    runAllowFail('git add --renormalize .', dryRun)
    console.log()

    if (!skipBump) {
      console.log('=== Running bump-version ===')
      runBumpVersion(newVersion, dryRun)
      console.log()
    }

    const version = readVersion()
    const tag = `v${version}`
    const changes = readChangesForVersion(version)

    console.log(`Version: ${version}`)
    console.log(`Tag: ${tag}`)
    console.log(`Changes:\n${changes.split('\n').slice(0, 5).join('\n')}${changes.split('\n').length > 5 ? '\n...' : ''}`)
    console.log()

    if (!skipBump) {
      console.log('=== Committing version bump ===')
      run(`git add -A`, dryRun)
      if (hasStagedChanges()) {
        run(`git commit -m "chore: release v${version}"`, dryRun)
      } else {
        console.log('No changes to commit (version already up to date)')
      }
      console.log()
    }

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
        const tagMessage = `v${version}\n\n${changes}`
        const tempFile = path.join(process.cwd(), '.tag-msg.tmp')
        if (!dryRun) {
          fs.writeFileSync(tempFile, tagMessage)
        }
        run(`git tag ${tag} -F ${tempFile}`, dryRun)
        if (!dryRun) {
          fs.unlinkSync(tempFile)
        }
        run(`git push origin ${tag}`, dryRun)
      }
    }

    console.log()
    console.log('Done! Check GitHub Actions: https://github.com/RyenLee/rust-verse/actions')
  } catch (err) {
    console.error()
    console.error('========================================')
    console.error('              ERROR OCCURRED             ')
    console.error('========================================')
    console.error(err.message)
    console.error()

    // 恢复用户的修改
    if (hadUnstaged) {
      console.log('=== Attempting to restore your changes ===')
      try {
        runAllowFail('git stash pop', false)
        console.log('✓ Your changes have been restored')
      } catch (e) {
        console.error('✗ Failed to restore changes automatically')
        console.error('  Please run: git stash pop')
      }
    }

    process.exit(1)
  }
}

main()
