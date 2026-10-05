import { execSync } from 'child_process'
import { createHash } from 'crypto'
import fs from 'fs'
import fsp from 'fs/promises'
import path from 'path'
import zlib from 'zlib'

import AdmZip from 'adm-zip'
import { HttpsProxyAgent } from 'https-proxy-agent'
import fetch from 'node-fetch'
import { extract } from 'tar'

import { log_debug, log_error, log_info, log_success } from './utils.mjs'

/**
 * Prebuild script with optimization features:
 * 1. Skip downloading mihomo core if it already exists (unless --force is used)
 * 2. Cache version information for 1 hour to avoid repeated version checks
 * 3. Use file hash to detect changes and skip unnecessary chmod/copy operations
 * 4. Use --force or -f flag to force re-download and update all resources
 *
 */

const cwd = process.cwd()
const TEMP_DIR = path.join(cwd, 'node_modules/.verge')
const FORCE = process.argv.includes('--force') || process.argv.includes('-f')
const VERSION_CACHE_FILE = path.join(TEMP_DIR, '.version_cache.json')
const HASH_CACHE_FILE = path.join(TEMP_DIR, '.hash_cache.json')

// Windows x64 only — the project no longer targets other platforms/architectures.
const PLATFORM_MAP = {
  'x86_64-pc-windows-msvc': 'win32',
}
const ARCH_MAP = {
  'x86_64-pc-windows-msvc': 'x64',
}

const arg1 = process.argv.slice(2)[0]
const arg2 = process.argv.slice(2)[1]
const target = arg1 === '--force' || arg1 === '-f' ? arg2 : arg1
if (target && !PLATFORM_MAP[target]) {
  throw new Error(
    `unsupported target "${target}" — only x86_64-pc-windows-msvc is supported`,
  )
}
const { platform, arch } = target
  ? { platform: PLATFORM_MAP[target], arch: ARCH_MAP[target] }
  : process

const SIDECAR_HOST = target
  ? target
  : execSync('rustc -vV')
      .toString()
      .match(/(?<=host: ).+(?=\s*)/g)[0]

const RESOURCES_DIR = path.join(cwd, 'src-tauri', 'resources')
const SIDECAR_DIR = path.join(cwd, 'src-tauri', 'sidecar')
// Windows ships the service binaries as resources next to the app.
const SERVICE_DIR = RESOURCES_DIR

// =======================
// Version Cache
// =======================
async function loadVersionCache() {
  try {
    if (fs.existsSync(VERSION_CACHE_FILE)) {
      const data = await fsp.readFile(VERSION_CACHE_FILE, 'utf-8')
      return JSON.parse(data)
    }
  } catch (err) {
    log_debug('Failed to load version cache:', err.message)
  }
  return {}
}
async function saveVersionCache(cache) {
  try {
    await fsp.mkdir(TEMP_DIR, { recursive: true })
    await fsp.writeFile(VERSION_CACHE_FILE, JSON.stringify(cache, null, 2))
    log_debug('Version cache saved')
  } catch (err) {
    log_debug('Failed to save version cache:', err.message)
  }
}
async function getCachedVersion(key) {
  const cache = await loadVersionCache()
  const cached = cache[key]
  if (cached && Date.now() - cached.timestamp < 3600000) {
    log_info(`Using cached version for ${key}: ${cached.version}`)
    return cached.version
  }
  return null
}
async function setCachedVersion(key, version) {
  const cache = await loadVersionCache()
  cache[key] = { version, timestamp: Date.now() }
  await saveVersionCache(cache)
}

// =======================
// Hash Cache & File Hash
// =======================
async function calculateFileHash(filePath) {
  try {
    const fileBuffer = await fsp.readFile(filePath)
    const hashSum = createHash('sha256')
    hashSum.update(fileBuffer)
    return hashSum.digest('hex')
  } catch (ignoreErr) {
    return null
  }
}
async function loadHashCache() {
  try {
    if (fs.existsSync(HASH_CACHE_FILE)) {
      const data = await fsp.readFile(HASH_CACHE_FILE, 'utf-8')
      return JSON.parse(data)
    }
  } catch (err) {
    log_debug('Failed to load hash cache:', err.message)
  }
  return {}
}
async function saveHashCache(cache) {
  try {
    await fsp.mkdir(TEMP_DIR, { recursive: true })
    await fsp.writeFile(HASH_CACHE_FILE, JSON.stringify(cache, null, 2))
    log_debug('Hash cache saved')
  } catch (err) {
    log_debug('Failed to save hash cache:', err.message)
  }
}
async function hasFileChanged(filePath, targetPath) {
  if (FORCE) return true
  if (!fs.existsSync(targetPath)) return true
  const hashCache = await loadHashCache()
  const sourceHash = await calculateFileHash(filePath)
  const targetHash = await calculateFileHash(targetPath)
  if (!sourceHash || !targetHash) return true
  const cacheKey = targetPath
  const cachedHash = hashCache[cacheKey]
  if (cachedHash === sourceHash && sourceHash === targetHash) {
    return false
  }
  return true
}
async function updateHashCache(targetPath) {
  const hashCache = await loadHashCache()
  const hash = await calculateFileHash(targetPath)
  if (hash) {
    hashCache[targetPath] = hash
    await saveHashCache(hashCache)
  }
}

// =======================
// Meta maps
// =======================
const META_VERSION_URL =
  'https://github.com/MetaCubeX/mihomo/releases/latest/download/version.txt'
const META_URL_PREFIX = `https://github.com/MetaCubeX/mihomo/releases/download`
let META_VERSION

const META_MAP = {
  'win32-x64': 'mihomo-windows-amd64-v2',
}

async function getLatestReleaseVersion() {
  if (!FORCE) {
    const cached = await getCachedVersion('META_VERSION')
    if (cached) {
      META_VERSION = cached
      return
    }
  }
  const options = {}
  const httpProxy =
    process.env.HTTP_PROXY ||
    process.env.http_proxy ||
    process.env.HTTPS_PROXY ||
    process.env.https_proxy
  if (httpProxy) options.agent = new HttpsProxyAgent(httpProxy)

  try {
    const response = await fetch(META_VERSION_URL, {
      ...options,
      method: 'GET',
    })
    if (!response.ok)
      throw new Error(`Failed to fetch ${META_VERSION_URL}: ${response.status}`)
    META_VERSION = (await response.text()).trim()
    log_info(`Latest release version: ${META_VERSION}`)
    await setCachedVersion('META_VERSION', META_VERSION)
  } catch (err) {
    log_error('Error fetching latest release version:', err.message)
    process.exit(1)
  }
}

// =======================
// Validate availability
// =======================
if (!META_MAP[`${platform}-${arch}`]) {
  throw new Error(
    `unsupported host "${platform}-${arch}" — only win32-x64 (Windows x64) is supported`,
  )
}

// =======================
// Build meta objects
// =======================
function clashMeta() {
  const name = META_MAP[`${platform}-${arch}`]
  return {
    name: 'mini-mihomo',
    targetFile: `mini-mihomo-${SIDECAR_HOST}.exe`,
    exeFile: `${name}.exe`,
    zipFile: `${name}-${META_VERSION}.zip`,
    downloadURL: `${META_URL_PREFIX}/${META_VERSION}/${name}-${META_VERSION}.zip`,
  }
}

// =======================
// download helper (Enhanced: status + magic bytes)
// =======================
async function downloadFile(url, outPath) {
  const options = {}
  const httpProxy =
    process.env.HTTP_PROXY ||
    process.env.http_proxy ||
    process.env.HTTPS_PROXY ||
    process.env.https_proxy
  if (httpProxy) options.agent = new HttpsProxyAgent(httpProxy)

  const response = await fetch(url, {
    ...options,
    method: 'GET',
    headers: { 'Content-Type': 'application/octet-stream' },
  })
  if (!response.ok) {
    const body = await response.text().catch(() => '')
    // 失败响应体写到 .error 文件便于排查，绝不写入目标文件：
    // 否则残留的非空垃圾文件会被后续构建当作有效文件而跳过下载
    await fsp.mkdir(path.dirname(outPath), { recursive: true })
    await fsp.writeFile(`${outPath}.error`, body)
    throw new Error(`Failed to download ${url}: status ${response.status}`)
  }

  const buf = Buffer.from(await response.arrayBuffer())

  // Simple magic bytes check
  if (url.endsWith('.gz') || url.endsWith('.tgz')) {
    if (!(buf[0] === 0x1f && buf[1] === 0x8b)) {
      throw new Error(
        `Downloaded file for ${url} is not a valid gzip (magic mismatch).`,
      )
    }
  } else if (url.endsWith('.zip')) {
    if (!(buf[0] === 0x50 && buf[1] === 0x4b)) {
      throw new Error(
        `Downloaded file for ${url} is not a valid zip (magic mismatch).`,
      )
    }
  }

  // 先写临时文件、校验通过后再替换目标文件，避免中途失败留下半截文件
  await fsp.mkdir(path.dirname(outPath), { recursive: true })
  const tmpPath = `${outPath}.tmp`
  await fsp.writeFile(tmpPath, buf)
  await fsp.rename(tmpPath, outPath)
  log_success(`download finished: ${url}`)
}

// =======================
// resolveSidecar (Supports zip / tgz / gz)
// =======================
async function resolveSidecar(binInfo) {
  const { name, targetFile, zipFile, exeFile, downloadURL } = binInfo
  const sidecarPath = path.join(SIDECAR_DIR, targetFile)
  await fsp.mkdir(SIDECAR_DIR, { recursive: true })

  if (!FORCE && fs.existsSync(sidecarPath)) {
    log_success(`"${name}" already exists, skipping download`)
    return
  }

  const tempDir = path.join(TEMP_DIR, name)
  const tempZip = path.join(tempDir, zipFile)
  const tempExe = path.join(tempDir, exeFile)
  await fsp.mkdir(tempDir, { recursive: true })

  try {
    if (!fs.existsSync(tempZip)) {
      await downloadFile(downloadURL, tempZip)
    }

    if (zipFile.endsWith('.zip')) {
      const zip = new AdmZip(tempZip)
      zip.getEntries().forEach((entry) => {
        log_debug(`"${name}" entry: ${entry.entryName}`)
      })
      zip.extractAllTo(tempDir, true)
      // Try renaming according to exeFile, otherwise find the first executable file
      if (fs.existsSync(tempExe)) {
        await fsp.rename(tempExe, sidecarPath)
      } else {
        // Search candidates
        const files = await fsp.readdir(tempDir)
        const candidate = files.find(
          (f) =>
            f === path.basename(exeFile) ||
            f.endsWith('.exe') ||
            !f.includes('.'),
        )
        if (!candidate)
          throw new Error(`Expected binary not found in ${tempDir}`)
        await fsp.rename(path.join(tempDir, candidate), sidecarPath)
      }
      log_success(`unzip finished: "${name}"`)
    } else if (zipFile.endsWith('.tgz')) {
      await extract({ cwd: tempDir, file: tempZip })
      const files = await fsp.readdir(tempDir)
      log_debug(`"${name}" extracted files:`, files)
      // Prioritize searching for the given exeFile or known prefixes
      let extracted = files.find(
        (f) =>
          f === path.basename(exeFile) ||
          f.startsWith('\u865a\u7a7a\u7ec8\u7aef-') ||
          !f.includes('.'),
      )
      if (!extracted) extracted = files[0]
      if (!extracted) throw new Error(`Expected file not found in ${tempDir}`)
      await fsp.rename(path.join(tempDir, extracted), sidecarPath)
      execSync(`chmod 755 ${sidecarPath}`)
      log_success(`tgz processed: "${name}"`)
    } else {
      // .gz
      const readStream = fs.createReadStream(tempZip)
      const writeStream = fs.createWriteStream(sidecarPath)
      await new Promise((resolve, reject) => {
        readStream
          .pipe(zlib.createGunzip())
          .on('error', (e) => {
            log_error(`gunzip error for ${name}:`, e.message)
            reject(e)
          })
          .pipe(writeStream)
          .on('finish', () => {
            resolve()
          })
          .on('error', (e) => {
            log_error(`write stream error for ${name}:`, e.message)
            reject(e)
          })
      })
      log_success(`gz binary processed: "${name}"`)
    }
  } catch (err) {
    await fsp.rm(sidecarPath, { recursive: true, force: true })
    throw err
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true })
  }
}

async function resolveResource(binInfo) {
  const { file, downloadURL, localPath, dir } = binInfo
  const baseDir = dir ?? RESOURCES_DIR
  const targetPath = path.join(baseDir, file)

  if (!FORCE && fs.existsSync(targetPath) && !downloadURL && !localPath) {
    try {
      if (fs.statSync(targetPath).size > 0) {
        log_success(`"${file}" already exists, skipping`)
        return
      }
    } catch {}
  }

  if (downloadURL) {
    if (!FORCE && fs.existsSync(targetPath)) {
      try {
        if (fs.statSync(targetPath).size > 0) {
          log_success(`"${file}" already exists, skipping download`)
          return
        }
      } catch {}
    }
    await fsp.mkdir(baseDir, { recursive: true })
    await downloadFile(downloadURL, targetPath)
    await updateHashCache(targetPath)
  }

  if (localPath) {
    if (!(await hasFileChanged(localPath, targetPath))) {
      return
    }
    await fsp.mkdir(baseDir, { recursive: true })
    await fsp.copyFile(localPath, targetPath)
    await updateHashCache(targetPath)
    log_success(`Copied file: ${file}`)
  }

  log_success(`${file} finished`)
}

// SimpleSC.dll (win plugin)
const resolvePlugin = async () => {
  const url =
    'https://nsis.sourceforge.io/mediawiki/images/e/ef/NSIS_Simple_Service_Plugin_Unicode_1.30.zip'
  const tempDir = path.join(TEMP_DIR, 'SimpleSC')
  const tempZip = path.join(
    tempDir,
    'NSIS_Simple_Service_Plugin_Unicode_1.30.zip',
  )
  const tempDll = path.join(tempDir, 'SimpleSC.dll')
  const pluginDir = path.join(process.env.APPDATA || '', 'Local/NSIS')
  const pluginPath = path.join(pluginDir, 'SimpleSC.dll')
  await fsp.mkdir(pluginDir, { recursive: true })
  await fsp.mkdir(tempDir, { recursive: true })
  if (!FORCE && fs.existsSync(pluginPath)) return
  try {
    if (!fs.existsSync(tempZip)) {
      await downloadFile(url, tempZip)
    }
    const zip = new AdmZip(tempZip)
    zip
      .getEntries()
      .forEach((entry) => log_debug(`"SimpleSC" entry`, entry.entryName))
    zip.extractAllTo(tempDir, true)
    if (fs.existsSync(tempDll)) {
      await fsp.cp(tempDll, pluginPath, { recursive: true, force: true })
      log_success(`unzip finished: "SimpleSC"`)
    } else {
      // If the dll name is different, try to find the dll
      const files = await fsp.readdir(tempDir)
      const dll = files.find((f) => f.toLowerCase().endsWith('.dll'))
      if (dll) {
        await fsp.cp(path.join(tempDir, dll), pluginPath, {
          recursive: true,
          force: true,
        })
        log_success(`unzip finished: "SimpleSC" (found ${dll})`)
      } else {
        throw new Error('SimpleSC.dll not found in zip')
      }
    }
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true })
  }
}

// =======================
// Other resource resolvers (service, mmdb, geosite, enableLoopback)
// =======================
// IMPORTANT: pinned to a known-good daemon release — do NOT fetch from
// /releases/latest. On 2026-07-23 upstream published v2.5.0, whose daemon fails
// to start the Windows service (cannot open "C:\ProgramData\clash-verge-service",
// os error 5 -> ExitCode 1), which broke TUN mode in v2.7.4 / v2.7.5. v2.3.3 is
// the last release before that regression (it shipped fine in v2.7.3).
const SERVICE_LATEST_URL =
  'https://github.com/clash-verge-rev/clash-verge-service-ipc/releases/tag/v2.3.3'
const SERVICE_URL_PREFIX =
  'https://github.com/clash-verge-rev/clash-verge-service-ipc/releases/download'
let SERVICE_VERSION

const SERVICE_BINARIES = [
  'clash-verge-service',
  'clash-verge-service-install',
  'clash-verge-service-uninstall',
]

function serviceFileInfo(name) {
  return {
    sourceFile: `${name}.exe`,
    targetFile: `${name}.exe`,
  }
}

function parseServiceVersionFromUrl(url) {
  const match = url.match(/\/releases\/tag\/([^/?#]+)/)
  return match ? decodeURIComponent(match[1]) : null
}

async function getLatestServiceVersion() {
  if (!FORCE) {
    const cached = await getCachedVersion('SERVICE_VERSION')
    if (cached) {
      SERVICE_VERSION = cached
      return
    }
  }

  const options = {}
  const httpProxy =
    process.env.HTTP_PROXY ||
    process.env.http_proxy ||
    process.env.HTTPS_PROXY ||
    process.env.https_proxy
  if (httpProxy) options.agent = new HttpsProxyAgent(httpProxy)

  try {
    const response = await fetch(SERVICE_LATEST_URL, {
      ...options,
      method: 'GET',
      redirect: 'follow',
    })
    if (!response.ok)
      throw new Error(
        `Failed to fetch ${SERVICE_LATEST_URL}: ${response.status}`,
      )

    SERVICE_VERSION = parseServiceVersionFromUrl(response.url)
    if (!SERVICE_VERSION)
      throw new Error(
        `Unable to resolve service release tag from ${response.url}`,
      )

    log_info(`Latest service version: ${SERVICE_VERSION}`)
    await setCachedVersion('SERVICE_VERSION', SERVICE_VERSION)
  } catch (err) {
    log_error('Error fetching latest service version:', err.message)
    process.exit(1)
  }
}

async function findExtractedFile(dir, fileName) {
  const entries = await fsp.readdir(dir, { withFileTypes: true })
  for (const entry of entries) {
    const entryPath = path.join(dir, entry.name)
    if (entry.isFile() && entry.name === fileName) return entryPath
    if (entry.isDirectory()) {
      const found = await findExtractedFile(entryPath, fileName)
      if (found) return found
    }
  }
  return null
}

async function resolveServiceBundle() {
  const files = SERVICE_BINARIES.map((name) => {
    const info = serviceFileInfo(name)
    return {
      ...info,
      targetPath: path.join(SERVICE_DIR, info.targetFile),
    }
  })

  if (!FORCE && files.every(({ targetPath }) => fs.existsSync(targetPath))) {
    log_success('"clash-verge-service-ipc" already exists, skipping download')
    return
  }

  await getLatestServiceVersion()

  const archiveExt = 'zip'
  const archiveFile = `clash-verge-service-ipc-${SERVICE_VERSION}-${SIDECAR_HOST}.${archiveExt}`
  const downloadURL = `${SERVICE_URL_PREFIX}/${SERVICE_VERSION}/${archiveFile}`
  const tempDir = path.join(TEMP_DIR, 'clash-verge-service-ipc')
  const tempArchive = path.join(tempDir, archiveFile)

  await fsp.mkdir(tempDir, { recursive: true })
  await fsp.mkdir(SERVICE_DIR, { recursive: true })

  try {
    await downloadFile(downloadURL, tempArchive)

    const zip = new AdmZip(tempArchive)
    zip
      .getEntries()
      .forEach((entry) =>
        log_debug('"clash-verge-service-ipc" entry:', entry.entryName),
      )
    zip.extractAllTo(tempDir, true)

    for (const { sourceFile, targetFile, targetPath } of files) {
      const extractedFile = await findExtractedFile(tempDir, sourceFile)
      if (!extractedFile) {
        throw new Error(`Expected binary ${sourceFile} not found in archive`)
      }

      await fsp.copyFile(extractedFile, targetPath)
      await updateHashCache(targetPath)
      log_success(`Extracted service file: ${targetFile}`)
    }

    log_success(`service bundle finished: ${archiveFile}`)
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true })
  }
}

const resolveMmdb = () =>
  resolveResource({
    file: 'Country.mmdb',
    downloadURL: `https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/country.mmdb`,
  })
const resolveGeosite = () =>
  resolveResource({
    file: 'geosite.dat',
    downloadURL: `https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat`,
  })
const resolveEnableLoopback = () =>
  resolveResource({
    file: 'enableLoopback.exe',
    downloadURL: `https://github.com/Kuingsmile/uwp-tool/releases/download/latest/enableLoopback.exe`,
  })

// =======================
// Tasks
// =======================
const tasks = [
  {
    name: 'mini-mihomo',
    func: () =>
      getLatestReleaseVersion().then(() => resolveSidecar(clashMeta())),
    retry: 5,
  },
  { name: 'plugin', func: resolvePlugin, retry: 5, winOnly: true },
  { name: 'service', func: resolveServiceBundle, retry: 5 },
  { name: 'mmdb', func: resolveMmdb, retry: 5 },
  { name: 'geosite', func: resolveGeosite, retry: 5 },
  {
    name: 'enableLoopback',
    func: resolveEnableLoopback,
    retry: 5,
    winOnly: true,
  },
  {
    name: 'copy_readme',
    func: async () => {
      // Copy 用户必读.txt to src-tauri/ (for NSIS)
      const src = path.join(cwd, '用户必读.txt')
      const dest = path.join(cwd, 'src-tauri', '用户必读.txt')
      if (fs.existsSync(src)) {
        await fsp.copyFile(src, dest)
        log_success('Copied 用户必读.txt to src-tauri/')
      } else {
        log_info('用户必读.txt not found, skipping copy')
      }

      // Copy 用户必读.txt to src-tauri/resources/ (bundled resource)
      const dest_res = path.join(cwd, 'src-tauri', 'resources', '用户必读.txt')
      if (fs.existsSync(src)) {
        await fsp.copyFile(src, dest_res)
        log_success('Copied 用户必读.txt to src-tauri/resources/')
      }
    },
    retry: 3,
  },
]

async function runTask() {
  const task = tasks.shift()
  if (!task) return
  if (task.winOnly && platform !== 'win32') return runTask()

  for (let i = 0; i < task.retry; i++) {
    try {
      await task.func()
      break
    } catch (err) {
      log_error(`task::${task.name} try ${i} ==`, err.message)
      if (i === task.retry - 1) throw err
    }
  }
  return runTask()
}

runTask()
