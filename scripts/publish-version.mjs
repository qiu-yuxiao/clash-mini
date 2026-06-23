// scripts/publish-version.mjs
import { spawn } from 'child_process'
import fs, { existsSync } from 'fs'
import path from 'path'

const rootDir = process.cwd()
const scriptPath = path.join(rootDir, 'scripts', 'release-version.mjs')

if (!existsSync(scriptPath)) {
  console.error('release-version.mjs not found!')
  process.exit(1)
}

const versionArg = process.argv[2]
if (!versionArg) {
  console.error('Usage: pnpm publish-version <version>')
  process.exit(1)
}

// 1. Call verify.py for pre-release verification
const runVerify = () =>
  new Promise((resolve, reject) => {
    const child = spawn('python', ['verify.py'], { stdio: 'inherit' })
    child.on('exit', (code) => {
      if (code === 0) resolve()
      else reject(new Error('verify.py failed'))
    })
  })

// 2. Call release-version.mjs
const runRelease = () =>
  new Promise((resolve, reject) => {
    const child = spawn('node', [scriptPath, versionArg], { stdio: 'inherit' })
    child.on('exit', (code) => {
      if (code === 0) resolve()
      else reject(new Error('release-version failed'))
    })
  })

// 3. Check if we need to tag
function isSemver(version) {
  return /^v?\d+\.\d+\.\d+(-[0-9A-Za-z-.]+)?$/.test(version)
}

async function run() {
  await runRelease()
  try {
    await runVerify()
  } catch (err) {
    console.error('[ERROR]: verify.py verification failed! Release tag will not be created.')
    console.error('[INFO]: Please fix the Changelog/Bug List alignment issues and run the command again.')
    process.exit(1)
  }

  let tag = null
  if (versionArg === 'alpha') {
    // Read package version from package.json
    const pkg = await import(path.join(rootDir, 'package.json'), {
      assert: { type: 'json' },
    })
    tag = `v${pkg.default.version}-alpha`
  } else if (isSemver(versionArg)) {
    // 1.2.3 or v1.2.3
    tag = versionArg.startsWith('v') ? versionArg : `v${versionArg}`
  }

  if (tag) {
    const { execSync } = await import('child_process')
    console.log('[INFO]: Committing version configurations and docs...')
    try {
      execSync('git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json updater/app-update.json bug_list.md Changelog.md Cargo.lock', { stdio: 'inherit' })
      execSync(`git commit -m "release: bump version to ${versionArg}" --no-verify`, { stdio: 'inherit' })
      console.log('[INFO]: Pushing dev branch to origin...')
      execSync('git push origin dev --no-verify', { stdio: 'inherit' })
    } catch (gitErr) {
      console.warn('[WARNING]: Git commit or push failed, but proceeding to tag anyway:', gitErr.message)
    }

    try {
      execSync(`git tag ${tag}`, { stdio: 'inherit' })
      execSync(`git push origin ${tag} --no-verify`, { stdio: 'inherit' })
      console.log(`[INFO]: Git tag ${tag} created and pushed.`)

      // Spawn background build monitor and log to release_monitor.log
      console.log('[INFO]: Spawning background build monitor...')
      const logFile = path.join(rootDir, 'release_monitor.log')
      const out = fs.openSync(logFile, 'a')
      const err = fs.openSync(logFile, 'a')
      const monitorProcess = spawn('node', ['scratch/monitor_build.mjs'], {
        detached: true,
        stdio: ['ignore', out, err]
      })
      monitorProcess.unref()
      console.log(`[INFO]: Build monitor spawned in background. Log file: ${logFile}`)
      console.log('[INFO]: It will automatically wait for CI and download the portable release.')
    } catch (err) {
      console.error(`[ERROR]: Failed to create or push git tag: ${tag}`, err.message)
      process.exit(1)
    }
  } else {
    console.log('[INFO]: No git tag created for this version.')
  }
}

run()
