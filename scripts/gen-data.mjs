// Regenerates the JSON data the Rust backend embeds, so the CLI and the
// desktop app share one source of truth (lib/companies.js, lib/deterministic-id.js).
import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import companies from '../../lib/companies.js'
import { words } from '../../lib/deterministic-id.js'

const here = path.dirname(fileURLToPath(import.meta.url))
const outDir = path.join(here, '..', 'src-tauri', 'data')

await fs.mkdir(outDir, { recursive: true })
await fs.writeFile(
    path.join(outDir, 'companies.json'),
    JSON.stringify(companies, null, 2) + '\n'
)
await fs.writeFile(
    path.join(outDir, 'words.json'),
    JSON.stringify(words, null, 2) + '\n'
)
console.log(
    `Wrote ${Object.keys(companies).length} companies and ${words.length} words to ${outDir}`
)
