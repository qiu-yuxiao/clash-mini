import fs from 'fs/promises'
import { existsSync } from 'fs'
import { execSync } from 'child_process'
import path from 'path'

async function run() {
  // 1. Get the tag name from GITHUB_REF_NAME (or fallback for local testing)
  const tagName =
    process.env.GITHUB_REF_NAME || process.env.TAG_NAME || process.argv[2]
  if (!tagName) {
    console.error(
      'Error: GITHUB_REF_NAME or TAG_NAME environment variable is required.',
    )
    process.exit(1)
  }

  // Extract clean version (e.g. v1.1.7-full -> 1.1.7, v1.1.7 -> 1.1.7)
  const version = tagName.replace(/^v/, '').replace(/-(full|all)$/, '')
  console.log(
    `Generating update logs for Tag: ${tagName}, Normalized Version: ${version}`,
  )

  let changelogSection = ''

  // 2. Try to read from Changelog.md for the specific version
  const changelogPath = path.join(process.cwd(), 'Changelog.md')
  if (existsSync(changelogPath)) {
    try {
      const data = await fs.readFile(changelogPath, 'utf-8')
      const lines = data.split('\n')

      let isCapturing = false
      const captured = []

      const titleRegex = new RegExp(
        `^##\\s+v?${version.replace(/\./g, '\\.')}\\b`,
        'i',
      )
      const nextTitleRegex = /^##\s+v?\d+/i

      for (const line of lines) {
        if (titleRegex.test(line)) {
          isCapturing = true
          continue
        }
        if (isCapturing) {
          if (
            nextTitleRegex.test(line) ||
            line.trim() === '## 原始版本历史 (Clash Verge History)'
          ) {
            break
          }
          captured.push(line)
        }
      }

      if (captured.length > 0) {
        changelogSection = captured.join('\n').trim()
        console.log(
          `Found matching changelog section in Changelog.md for version ${version}`,
        )
      } else {
        console.log(
          `No matching changelog section found in Changelog.md for version ${version}`,
        )
      }
    } catch (err) {
      console.error('Error reading Changelog.md:', err)
    }
  }

  // 3. Try to read resolved bugs from bug_list.md matching the current version
  let resolvedBugsSection = ''
  const bugListPath = path.join(process.cwd(), 'bug_list.md')
  if (existsSync(bugListPath)) {
    try {
      const data = await fs.readFile(bugListPath, 'utf-8')
      const lines = data.split('\n')
      const bugs = []

      for (const line of lines) {
        if (line.trim().startsWith('|') && line.includes('BUG-')) {
          const parts = line.split('|').map((p) => p.trim())
          if (parts.length >= 5) {
            const bugId = parts[1].replace(/^\*\*|\*\*$/g, '') // strip ** if present
            const desc = parts[2]
            const resolveVer = parts[3] // e.g. v1.1.6 or 1.1.7

            const verNormalized = resolveVer
              .replace(/^v/, '')
              .replace(/-(full|all)$/, '')
            if (verNormalized === version) {
              bugs.push(`- **${bugId}**: ${desc}`)
            }
          }
        }
      }

      if (bugs.length > 0) {
        resolvedBugsSection = `### 🐞 已解决的缺陷 (Fixed Bugs)\n${bugs.join('\n')}`
        console.log(
          `Found ${bugs.length} resolved bugs for version ${version} in bug_list.md`,
        )
      }
    } catch (err) {
      console.error('Error reading bug_list.md:', err)
    }
  }

  // 4. Try to get git commits since last release tag
  let gitCommitsSection = ''
  try {
    const tagsOutput = execSync('git tag --sort=-v:refname')
      .toString()
      .trim()
      .split('\n')
    // Find the previous release tag (ignoring current tag, alpha, rc, deploytest)
    const prevTag = tagsOutput.find((t) => {
      const isReleaseTag =
        /^v\d+\.\d+\.\d+$/i.test(t) || /^v\d+\.\d+\.\d+-(full|all)$/i.test(t)
      return isReleaseTag && t !== tagName
    })

    let commits = ''
    if (prevTag) {
      console.log(`Found previous release tag: ${prevTag}`)
      commits = execSync(`git log ${prevTag}..HEAD --pretty=format:"- %s (%h)"`)
        .toString()
        .trim()
    } else {
      console.log('No previous release tag found, getting last 15 commits')
      commits = execSync(`git log -n 15 --pretty=format:"- %s (%h)"`)
        .toString()
        .trim()
    }

    if (commits) {
      gitCommitsSection = `### 🔨 提交历史 (Commit History)\n${commits}`
    }
  } catch (err) {
    console.error('Error getting git commits:', err.message)
  }

  // Combine logs
  let finalLogs = `## ${tagName} 更新日志\n\n`

  if (changelogSection) {
    finalLogs += `### 📝 主要更新 (Key Changes)\n${changelogSection}\n\n`
  } else {
    finalLogs += `### 📝 主要更新 (Key Changes)\n此版本包含代码优化和缺陷修复。\n\n`
  }

  if (resolvedBugsSection) {
    finalLogs += `${resolvedBugsSection}\n\n`
  }

  if (gitCommitsSection) {
    finalLogs += `${gitCommitsSection}\n\n`
  }

  finalLogs = finalLogs.trim()

  console.log('Extracted update logs:\n' + finalLogs)

  // Write to GITHUB_ENV if available
  const githubEnv = process.env.GITHUB_ENV
  if (githubEnv) {
    await fs.appendFile(githubEnv, `UPDATE_LOGS<<EOF\n${finalLogs}\nEOF\n`)
    console.log('Successfully wrote UPDATE_LOGS to GITHUB_ENV')
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
