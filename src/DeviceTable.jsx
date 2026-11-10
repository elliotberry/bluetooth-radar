import { useMemo, useState } from 'react'
import { deviceColor, truncateId } from './color.js'

const COLUMNS = [
    { key: 'word', label: 'Word', value: (d) => d.word },
    { key: 'id', label: 'ID', value: (d) => d.id, render: (d) => truncateId(d.id) },
    { key: 'name', label: 'Name', value: (d) => d.name },
    { key: 'company', label: 'Company', value: (d) => d.companyName ?? '', active: true },
    { key: 'rssi', label: 'RSSI', value: (d) => d.rssi ?? -999, active: true },
    { key: 'seenCount', label: 'Seen', value: (d) => d.seenCount ?? 0 },
    { key: 'lastSeen', label: 'Last seen', value: (d) => d.lastSeen },
]

function signalPercent(rssi) {
    if (rssi == null) return 0
    // Map roughly -100 dBm (nothing) .. -30 dBm (right next to us) to 0..100.
    return Math.max(0, Math.min(100, ((rssi + 100) / 70) * 100))
}

function formatAgo(ms) {
    const seconds = Math.max(0, Math.round((Date.now() - ms) / 1000))
    if (seconds < 60) return `${seconds}s ago`
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`
    return `${Math.floor(seconds / 86400)}d ago`
}

function compare(a, b) {
    if (typeof a === 'string' || typeof b === 'string') {
        return String(a).localeCompare(String(b))
    }
    return a - b
}

export default function DeviceTable({
    title,
    devices,
    selectedId,
    onSelect,
    emptyText,
    collapsible = false,
    compact = false,
}) {
    const [sort, setSort] = useState({ key: 'rssi', dir: 'desc' })
    const [collapsed, setCollapsed] = useState(collapsible)

    // The inactive section drops the live-only columns, like the CLI does.
    const columns = COLUMNS.filter((c) => !(compact && c.active))

    const sorted = useMemo(() => {
        const column = COLUMNS.find((c) => c.key === sort.key) ?? COLUMNS[0]
        const factor = sort.dir === 'asc' ? 1 : -1
        return [...devices].sort(
            (a, b) =>
                factor * compare(column.value(a), column.value(b)) ||
                a.id.localeCompare(b.id)
        )
    }, [devices, sort])

    const toggleSort = (key) =>
        setSort((s) =>
            s.key === key
                ? { key, dir: s.dir === 'asc' ? 'desc' : 'asc' }
                : { key, dir: ['rssi', 'seenCount', 'lastSeen'].includes(key) ? 'desc' : 'asc' }
        )

    return (
        <section className="table-section">
            <h2
                className={collapsible ? 'collapsible' : ''}
                onClick={collapsible ? () => setCollapsed((c) => !c) : undefined}
            >
                {collapsible && <span className="chevron">{collapsed ? '▸' : '▾'}</span>}
                {title} <span className="count">({devices.length})</span>
            </h2>
            {!collapsed &&
                (sorted.length === 0 ? (
                    <p className="empty">{emptyText}</p>
                ) : (
                    <table>
                        <thead>
                            <tr>
                                {columns.map((c) => (
                                    <th key={c.key} onClick={() => toggleSort(c.key)}>
                                        {c.label}
                                        {sort.key === c.key && (
                                            <span className="sort">
                                                {sort.dir === 'asc' ? ' ▲' : ' ▼'}
                                            </span>
                                        )}
                                    </th>
                                ))}
                            </tr>
                        </thead>
                        <tbody>
                            {sorted.map((d) => (
                                <tr
                                    key={d.id}
                                    className={d.id === selectedId ? 'selected' : ''}
                                    style={{ color: deviceColor(d.id) }}
                                    onClick={() => onSelect(d.id)}
                                >
                                    {columns.map((c) => (
                                        <td key={c.key}>
                                            {c.key === 'rssi' ? (
                                                <span className="rssi">
                                                    <span className="bar">
                                                        <span
                                                            style={{
                                                                width: `${signalPercent(d.rssi)}%`,
                                                            }}
                                                        />
                                                    </span>
                                                    {d.rssi ?? 'n/a'}
                                                </span>
                                            ) : c.key === 'lastSeen' ? (
                                                formatAgo(d.lastSeen)
                                            ) : c.render ? (
                                                c.render(d)
                                            ) : (
                                                c.value(d)
                                            )}
                                        </td>
                                    ))}
                                </tr>
                            ))}
                        </tbody>
                    </table>
                ))}
        </section>
    )
}
