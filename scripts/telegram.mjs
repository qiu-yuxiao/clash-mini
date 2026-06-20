import { readFileSync } from 'fs'

import axios from 'axios'

import { log_error, log_info, log_success } from './utils.mjs'

const CHAT_ID_RELEASE = '@clash_verge_re' // Official release channel
const CHAT_ID_TEST = '@vergetest' // Test channel

async function sendTelegramNotification() {
  if (!process.env.TELEGRAM_BOT_TOKEN) {
    throw new Error('TELEGRAM_BOT_TOKEN is required')
  }

  const version =
    process.env.VERSION ||
    (() => {
      const pkg = readFileSync('package.json', 'utf-8')
      return JSON.parse(pkg).version
    })()

  const downloadUrl =
    process.env.DOWNLOAD_URL ||
    `https://github.com/qiu-yuxiao/clash-mini/releases/download/v${version}`

  const isAutobuild =
    process.env.BUILD_TYPE === 'autobuild' || version.includes('autobuild')
  const chatId = isAutobuild ? CHAT_ID_TEST : CHAT_ID_RELEASE
  const buildType = isAutobuild ? '\u6eda\u52a8\u66f4\u65b0\u7248' : '\u6b63\u5f0f\u7248'

  log_info(`Preparing Telegram notification for ${buildType} ${version}`)
  log_info(`Target channel: ${chatId}`)
  log_info(`Download URL: ${downloadUrl}`)

  // Read release notes and download URL
  let releaseContent = ''
  try {
    releaseContent = readFileSync('release.txt', 'utf-8')
    log_info('[OK]: Successfully read release.txt file')
  } catch (error) {
    log_error('[WARN]: Failed to read release.txt, using default release notes', error)
    releaseContent = '\u66f4\u591a\u65b0\u529f\u80fd\u73b0\u5df2\u652f\u6301\uff0c\u8be6\u7ec6\u66f4\u65b0\u65e5\u5fd7\u8bf7\u67e5\u770b\u53d1\u5e03\u9875\u9762\u3002'
  }

  // Convert Markdown to HTML
  function convertMarkdownToTelegramHTML(content) {
    // Strip stray HTML tags and markdown bold from heading text
    const cleanHeading = (text) =>
      text
        .replace(/<\/?[^>]+>/g, '')
        .replace(/\*\*/g, '')
        .trim()
    return content
      .split('\n')
      .map((line) => {
        if (line.trim().length === 0) {
          return ''
        } else if (line.startsWith('## ')) {
          return `<b>${cleanHeading(line.replace('## ', ''))}</b>`
        } else if (line.startsWith('### ')) {
          return `<b>${cleanHeading(line.replace('### ', ''))}</b>`
        } else if (line.startsWith('#### ')) {
          return `<b>${cleanHeading(line.replace('#### ', ''))}</b>`
        } else {
          let processedLine = line.replace(
            /\[([^\]]+)\]\(([^)]+)\)/g,
            (match, text, url) => {
              const encodedUrl = encodeURI(url)
              return `<a href="${encodedUrl}">${text}</a>`
            },
          )
          processedLine = processedLine.replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
          return processedLine
        }
      })
      .join('\n')
  }

  function normalizeDetailsTags(content) {
    return content
      .replace(
        /<summary>\s*<strong>\s*(.*?)\s*<\/strong>\s*<\/summary>/g,
        '\n<b>$1</b>\n',
      )
      .replace(/<summary>\s*(.*?)\s*<\/summary>/g, '\n<b>$1</b>\n')
      .replace(/<\/?details>/g, '')
      .replace(/<\/?strong>/g, (m) => (m === '</strong>' ? '</b>' : '<b>'))
      .replace(/<br\s*\/?>/g, '\n')
  }

  // Strip HTML tags not supported by Telegram and escape stray angle brackets
  function sanitizeTelegramHTML(content) {
    // Telegram supports: b, strong, i, em, u, ins, s, strike, del,
    // a, code, pre, blockquote, tg-spoiler, tg-emoji
    const allowedTags =
      /^\/?(b|strong|i|em|u|ins|s|strike|del|a|code|pre|blockquote|tg-spoiler|tg-emoji)(\s|>|$)/i
    return content.replace(/<\/?[^>]*>/g, (tag) => {
      const inner = tag.replace(/^<\/?/, '').replace(/>$/, '')
      if (allowedTags.test(inner) || allowedTags.test(tag.slice(1))) {
        return tag
      }
      // Escape unsupported tags so they display as text
      return tag.replace(/</g, '&lt;').replace(/>/g, '&gt;')
    })
  }

  releaseContent = normalizeDetailsTags(releaseContent)
  const formattedContent = sanitizeTelegramHTML(
    convertMarkdownToTelegramHTML(releaseContent),
  )

  const releaseTitle = isAutobuild ? '\u6eda\u52a8\u66f4\u65b0\u7248\u53d1\u5e03' : '\u6b63\u5f0f\u53d1\u5e03'
  const encodedVersion = encodeURIComponent(version)
  const releaseTag = isAutobuild ? 'autobuild' : `v${version}`
  const content = `<b>🎉 <a href="https://github.com/qiu-yuxiao/clash-mini/releases/tag/${releaseTag}">Clash Mini v${version}</a> ${releaseTitle}</b>\n\n${formattedContent}`

  // Send to Telegram
  try {
    await axios.post(
      `https://api.telegram.org/bot${process.env.TELEGRAM_BOT_TOKEN}/sendMessage`,
      {
        chat_id: chatId,
        text: content,
        link_preview_options: {
          is_disabled: false,
          url: `https://github.com/qiu-yuxiao/clash-mini/releases/tag/v${encodedVersion}`,
          prefer_large_media: true,
        },
        parse_mode: 'HTML',
      },
    )
    log_success(`Telegram notification sent successfully to ${chatId}`)
  } catch (error) {
    log_error(
      `Telegram notification failed to send to ${chatId}:`,
      error.response?.data || error.message,
      error,
    )
    process.exit(1)
  }
}

// Execute function
sendTelegramNotification().catch((error) => {
  log_error('Script execution failed:', error)
  process.exit(1)
})
