// Port of stringToColor from lib/terminal-log.js, keyed on the device id so
// each device keeps a stable color (the CLI hashes the whole line, which
// changes whenever RSSI does).
export function stringToRgb(str) {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
        hash = str.charCodeAt(i) + ((hash << 5) - hash)
    }
    return [(hash >> 0) & 0xff, (hash >> 8) & 0xff, (hash >> 16) & 0xff]
}

// The terminal is dark, so very dark hashes are unreadable; lift them.
export function deviceColor(id) {
    const [r, g, b] = stringToRgb(id)
    const lift = (c) => Math.max(c, 90)
    return `rgb(${lift(r)}, ${lift(g)}, ${lift(b)})`
}

// Port of lib/truncate-id.js
export function truncateId(str) {
    if (!str) return ''
    if (str.length > 12) return `${str.slice(0, 4)}...${str.slice(-4)}`
    return str
}
