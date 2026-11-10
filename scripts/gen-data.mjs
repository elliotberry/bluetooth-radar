// Regenerates the JSON data the Rust backend embeds (src-tauri/data/):
//   words.json      word list, from cli/lib/deterministic-id.js (shared with the CLI)
//   companies.json  Bluetooth SIG company identifiers, "0x004C" -> name
//   services.json   Bluetooth service UUID names, "180F" -> name
//
// Companies and services come from the Nordic bluetooth-numbers-database
// (a mirror of the Bluetooth SIG assigned numbers). If the download fails,
// companies fall back to the older list in cli/lib/companies.js and the
// existing services.json is left untouched.
import fs from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import legacyCompanies from '../cli/lib/companies.js'
import { words } from '../cli/lib/deterministic-id.js'

const BASE =
    'https://raw.githubusercontent.com/NordicSemiconductor/bluetooth-numbers-database/master/v1'

const here = path.dirname(fileURLToPath(import.meta.url))
const outDir = path.join(here, '..', 'src-tauri', 'data')
await fs.mkdir(outDir, { recursive: true })

const writeJson = (name, value) =>
    fs.writeFile(path.join(outDir, name), JSON.stringify(value, null, 2) + '\n')

async function fetchJson(file) {
    try {
        const res = await fetch(`${BASE}/${file}`)
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return await res.json()
    } catch (err) {
        console.warn(`Could not download ${file}: ${err.message}`)
        return null
    }
}

const hex4 = (code) => `0x${code.toString(16).padStart(4, '0').toUpperCase()}`

// Companies: start from the old list, let the SIG data override and extend it.
const companies = {}
for (const [key, name] of Object.entries(legacyCompanies)) {
    if (name) companies[key.toUpperCase().replace('0X', '0x')] = name
}
const sigCompanies = await fetchJson('company_ids.json')
if (sigCompanies) {
    for (const { code, name } of sigCompanies) {
        if (name) companies[hex4(code)] = name
    }
}

// Services: keyed by the short UUID (4 hex digits) or the full 128-bit UUID, uppercase.
let services = null
const sigServices = await fetchJson('service_uuids.json')
if (sigServices) {
    services = {}
    for (const { uuid, name } of sigServices) {
        if (uuid && name) services[uuid.toUpperCase()] = name
    }
}

await writeJson('words.json', words)
await writeJson('companies.json', companies)
if (services) await writeJson('services.json', services)

console.log(
    `Wrote ${words.length} words, ${Object.keys(companies).length} companies` +
        (services ? `, ${Object.keys(services).length} services` : ', services unchanged') +
        ` to ${outDir}`
)
