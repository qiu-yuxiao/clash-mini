import fs from 'fs'
import fsp from 'fs/promises'
import { createRequire } from 'module'
import path from 'path'

import { context, getOctokit } from '@actions/github'
import AdmZip from 'adm-zip'

const target = process.argv.slice(2)[0]
const ARCH_MAP = {
  'x86_64-pc-windows-msvc': 'x64',
  'aarch64-pc-windows-msvc': 'arm64',
}

const PROCESS_MAP = {
  x64: 'x64',
  arm64: 'arm64',
}
const arch = target ? ARCH_MAP[target] : PROCESS_MAP[process.arch]
/// Script for ci
/// 打包绿色版/便携版 (only Windows)
async function resolvePortable() {
  if (process.platform !== 'win32') return

  const releaseDir = target ? `./target/${target}/release` : `./target/release`
  const configDir = path.join(releaseDir, '.config')

  if (!fs.existsSync(releaseDir)) {
    throw new Error('could not found the release dir')
  }

  await fsp.mkdir(configDir, { recursive: true })
  if (!fs.existsSync(path.join(configDir, 'PORTABLE'))) {
    await fsp.writeFile(path.join(configDir, 'PORTABLE'), '')
  }

  // 清理可能遗留的本地私有配置数据，防止敏感信息泄漏
  const privateDirs = [
    'io.github.clash-mini.clash-mini',
    'io.github.clash-mini.clash-mini.dev',
  ]
  for (const dirName of privateDirs) {
    const dirPath = path.join(configDir, dirName)
    if (fs.existsSync(dirPath)) {
      await fsp.rm(dirPath, { recursive: true, force: true })
    }
  }

  const zip = new AdmZip()

  zip.addLocalFile(path.join(releaseDir, 'clash-mini.exe'))
  zip.addLocalFile(path.join(releaseDir, 'verge-mihomo.exe'))
  zip.addLocalFile(path.join(releaseDir, 'verge-mihomo-alpha.exe'))
  zip.addLocalFolder(path.join(releaseDir, 'resources'), 'resources')
  zip.addLocalFolder(configDir, '.config')
  if (fs.existsSync(path.join(process.cwd(), '用户必读.txt'))) {
    zip.addLocalFile(path.join(process.cwd(), '用户必读.txt'))
  }

  const require = createRequire(import.meta.url)
  const packageJson = require('../package.json')
  const { version } = packageJson
  const zipFile = `Clash.Mini_${version}_${arch}_portable.zip`
  zip.writeZip(zipFile)
  console.log('[INFO]: create portable zip successfully')

  // push release assets
  if (process.env.GITHUB_TOKEN === undefined) {
    throw new Error('GITHUB_TOKEN is required')
  }

  const options = { owner: context.repo.owner, repo: context.repo.repo }
  const github = getOctokit(process.env.GITHUB_TOKEN)
  const tag = process.env.TAG_NAME || `v${version}`
  console.log('[INFO]: upload to ', tag)

  const { data: releases } = await github.rest.repos.listReleases({
    ...options,
    per_page: 100,
  })
  const release = releases.find((r) => r.tag_name === tag)
  if (!release) {
    throw new Error(`Release not found for tag ${tag}`)
  }

  const assets = release.assets.filter((x) => {
    return x.name === zipFile
  })
  if (assets.length > 0) {
    const id = assets[0].id
    await github.rest.repos.deleteReleaseAsset({
      ...options,
      asset_id: id,
    })
  }

  console.log(release.name)

  await github.rest.repos.uploadReleaseAsset({
    ...options,
    release_id: release.id,
    name: zipFile,
    data: zip.toBuffer(),
  })
}

resolvePortable().catch(console.error)
