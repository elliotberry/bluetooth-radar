export function signalPercent(rssi) {
    if (rssi == null) return 0
    // Map roughly -100 dBm (nothing) .. -30 dBm (right next to us) to 0..100.
    return Math.max(0, Math.min(100, ((rssi + 100) / 70) * 100))
}

export function formatAgo(ms) {
    const seconds = Math.max(0, Math.round((Date.now() - ms) / 1000))
    if (seconds < 60) return `${seconds}s ago`
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`
    return `${Math.floor(seconds / 86400)}d ago`
}

export function formatDistance(m) {
    if (m == null) return ''
    return m >= 10 ? `~${Math.round(m)} m` : `~${m.toFixed(1)} m`
}

export function formatInterval(ms) {
    if (ms == null) return null
    if (ms >= 1000) return `${(ms / 1000).toFixed(1)} s`
    return `${ms} ms`
}

export function formatTime(ms) {
    return ms ? new Date(ms).toLocaleString() : 'n/a'
}

export const TREND_SYMBOL = { closer: '▲', farther: '▼', steady: '•' }

// Primary company shown in the table: the first manufacturer entry, with a
// count of the others.
export function companyLabel(d) {
    const [first, ...rest] = d.manufacturers
    if (!first) return ''
    return rest.length ? `${first.companyName} +${rest.length}` : first.companyName
}

// One readable line for the table, from whatever we managed to decode.
export function infoLabel(d) {
    for (const m of d.manufacturers) {
        if (m.decoded.length) return m.decoded[0].value
    }
    for (const s of d.serviceData) {
        if (s.decoded.length) {
            const f = s.decoded[0]
            return f.label === f.value ? f.value : `${f.label}: ${f.value}`
        }
    }
    return d.appearanceLabel ?? ''
}
