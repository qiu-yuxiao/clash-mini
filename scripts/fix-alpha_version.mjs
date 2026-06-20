import { exec } from 'child_process'
import fs from 'fs/promises'
import path from 'path'
import { promisify } from 'util'

/**
 * Rename version number for Alpha release
 */
const execPromise = promisify(exec)

/**
 * Get HEAD commit hash
 */
async function getLatestCommitHash() {
  try {
    const { stdout } = await execPromise('git rev-parse HEAD')
    const commitHash = stdout.trim()
    // Format, only extract the first 7 characters
    const formathash = commitHash.substring(0, 7)
    console.log(`Found the latest commit hash code: ${commitHash}`)
    return formathash
  } catch (error) {
    console.error('pnpm run fix-alpha-version ERROR', error)
  }
}

/**
 * @param {string} newVersion The formatted hash
 * Write the new version number into package.json
 */
async function updatePackageVersion(newVersion) {
  // Get process working directory
  const _dirname = process.cwd()
  const packageJsonPath = path.join(_dirname, 'package.json')
  try {
    // Read file
    const data = await fs.readFile(packageJsonPath, 'utf8')
    const packageJson = JSON.parse(data)
    // Perform string replacement
    let result = packageJson.version.replace('alpha', newVersion)
    // Check if the current version already contains 'alpha-' suffix
    if (!packageJson.version.includes(`alpha-`)) {
      // If it only contains 'alpha' without '-', replace with 'alpha-newVersion'
      result = packageJson.version.replace('alpha', `alpha-${newVersion}`)
    } else {
      // If it's already in 'alpha-xxx' format, update the 'xxx' part
      result = packageJson.version.replace(/alpha-[^-]*/, `alpha-${newVersion}`)
    }
    console.log('[INFO]: Current version is: ', result)
    packageJson.version = result
    // Write version number
    await fs.writeFile(
      packageJsonPath,
      JSON.stringify(packageJson, null, 2),
      'utf8',
    )
    console.log(`[INFO]: Alpha version update to: ${newVersion}`)
  } catch (error) {
    console.error('pnpm run fix-alpha-version ERROR', error)
  }
}

const newVersion = await getLatestCommitHash()
updatePackageVersion(newVersion).catch(console.error)
