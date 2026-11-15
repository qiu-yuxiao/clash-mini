import { execSync } from 'child_process'
import { existsSync } from 'fs'
import fs from 'fs/promises'
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
            line.trim() ===
              '## \u539f\u59cb\u7248\u672c\u5386\u53f2 (Clash Verge History)'
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

  // 3. Try to read resolved/matched bugs from bug_list.md for the current version
  let resolvedBugsSection = ''
  const bugListPath = path.join(process.cwd(), 'bug_list.md')
  if (existsSync(bugListPath)) {
    try {
      const data = await fs.readFile(bugListPath, 'utf-8')
      const lines = data.split('\n')
      const bugs = []
      const seenBugs = new Set()

      const headingBugs = new Set()
      if (changelogSection) {
        for (const m of changelogSection.matchAll(/^###\s+[^\n]*$/gm)) {
          for (const b of m[0].matchAll(/\bBUG-\d+\b/g)) {
            headingBugs.add(b[0])
          }
        }
      }

      // 3a. Archived 3-column table: | **BUG-XXX** | vX.Y.Z | 状态 |
      for (const line of lines) {
        if (line.trim().startsWith('|') && line.includes('BUG-')) {
          const parts = line.split('|').map((p) => p.trim())
          if (parts.length >= 4) {
            const bugId = parts[1].replace(/\*/g, '').trim()
            if (!bugId.startsWith('BUG-')) continue
            const resolveVer = parts[2]
            const statusOrDesc = parts[3]

            const verNormalized = resolveVer
              .replace(/^v/, '')
              .replace(/-(full|all)$/, '')
            if (verNormalized === version && !seenBugs.has(bugId)) {
              seenBugs.add(bugId)
              bugs.push(`- **${bugId}**: ${statusOrDesc}`)
            }
          }
        }
      }

      // 3b. Pending cards fallback: ### BUG-XXX（...）
      const escapedVer = version.replace(/\./g, '\\.')
      const explicitVerTagRegex = new RegExp(`\\bv?${escapedVer}\\b`, 'i')
      let currentCard = null

      const flushCard = (card) => {
        if (!card || seenBugs.has(card.bugId)) return
        const isMatched =
          headingBugs.has(card.bugId) ||
          explicitVerTagRegex.test(card.heading) ||
          explicitVerTagRegex.test(card.statusLine)
        if (!isMatched) return
        seenBugs.add(card.bugId)
        const summary = card.category || card.heading
        bugs.push(`- **${card.bugId}**: ${summary}`)
      }

      for (const line of lines) {
        const cardMatch = line.match(/^###\s+(BUG-\d+)\b(.*)/)
        if (cardMatch) {
          flushCard(currentCard)
          currentCard = {
            bugId: cardMatch[1],
            heading: line.replace(/^###\s+/, '').trim(),
            statusLine: '',
            category: '',
          }
          continue
        }
        if (currentCard) {
          if (line.startsWith('---') || line.startsWith('## ')) {
            flushCard(currentCard)
            currentCard = null
            continue
          }
          if (line.includes('**当前状态**')) {
            currentCard.statusLine = line
          } else if (line.includes('**类别与级别**')) {
            currentCard.category = line
              .replace(/^-\s*\*\*类别与级别\*\*：\s*/, '')
              .trim()
          }
        }
      }
      flushCard(currentCard)

      if (bugs.length > 0) {
        resolvedBugsSection = `### 🐞 Fixed Bugs\n${bugs.join('\n')}`
        console.log(
          `Found ${bugs.length} matching bugs for version ${version} in bug_list.md`,
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
      gitCommitsSection = `### \uD83D\uDD28 Commit History\n${commits}`
    }
  } catch (err) {
    console.error('Error getting git commits:', err.message)
  }

  // Combine logs
  let finalLogs = `## ${tagName} Update Logs\n\n`

  if (changelogSection) {
    finalLogs += `### \uD83D\uDCDD Key Changes\n${changelogSection}\n\n`
  } else {
    finalLogs += `### \uD83D\uDCDD Key Changes\nThis version includes code optimizations and bug fixes.\n\n`
  }

  if (resolvedBugsSection) {
    finalLogs += `${resolvedBugsSection}\n\n`
  }

  if (gitCommitsSection) {
    finalLogs += `${gitCommitsSection}\n\n`
  }

  finalLogs = finalLogs.trim()

  console.log('Extracted update logs:\n' + finalLogs)

  const envFile = process.env.GITHUB_ENV
  if (envFile) {
    const delimiter = 'EOF_UPDATE_LOGS_' + Date.now()
    const envContent = `UPDATE_LOGS<<${delimiter}\n${finalLogs}\n${delimiter}\n`
    await fs.appendFile(envFile, envContent, 'utf-8')
    console.log('Successfully wrote UPDATE_LOGS to GITHUB_ENV')
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
