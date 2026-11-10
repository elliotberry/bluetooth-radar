import { useMemo, useState } from 'react'
import { deviceColor, truncateId } from './color.js'
import {
    TREND_SYMBOL,
    companyLabel,
    formatAgo,
    formatDistance,
    infoLabel,
    signalPercent,
} from './format.js'

const COLUMNS = [
    {
        key: 'word',
        label: 'Word',
        value: (d) => d.word,
        render: (d) => (
            <>
                {d.word}
                {d.likelyRandom && (
                    <span className="flag" title={`Rotating address: ${d.randomReason}`}>
                        {' '}
                        ↻
                    </span>
                )}
            </>
        ),
    },
    { key: 'id', label: 'ID', value: (d) => d.id, render: (d) => truncateId(d.id) },
    { key: 'name', label: 'Name', value: (d) => d.name },
    { key: 'company', label: 'Company', value: companyLabel, active: true },
    { key: 'info', label: 'Info', value: infoLabel, active: true },
    {
        key: 'rssi',
        label: 'RSSI',
        value: (d) => d.rssi ?? -999,
        active: true,
        render: (d) => (
            <span className="rssi">
                <span className="bar">
                    <span style={{ width: `${signalPercent(d.rssi)}%` }} />
                </span>
                {d.rssi ?? 'n/a'}
                {d.rssiTrend && (
                    <span className="trend" title={`Getting ${d.rssiTrend}`}>
                        {TREND_SYMBOL[d.rssiTrend]}
                    </span>
                )}
            </span>
        ),
    },
    {
        key: 'distance',
        label: 'Distance',
        value: (d) => d.distanceM ?? Infinity,
        active: true,
        render: (d) => formatDistance(d.distanceM),
    },
    {
        key: 'connectable',
        label: 'Conn',
        value: (d) => (d.connectable == null ? -1 : d.connectable ? 1 : 0),
        active: true,
        render: (d) => (d.connectable == null ? '' : d.connectable ? 'y' : 'n'),
    },
    { key: 'seenCount', label: 'Seen', value: (d) => d.seenCount ?? 0 },
    {
        key: 'lastSeen',
        label: 'Last seen',
        value: (d) => d.lastSeen,
        render: (d) => formatAgo(d.lastSeen),
    },
]

export const DEFAULT_COLUMN_ORDER = COLUMNS.map((c) => c.key)

// Saved order first (ignoring keys that no longer exist), then any columns
// the saved order does not mention, so newly added columns still appear.
function resolveOrder(saved) {
    const known = new Set(DEFAULT_COLUMN_ORDER)
    const order = (Array.isArray(saved) ? saved : []).filter(
        (k, i, all) => known.has(k) && all.indexOf(k) === i
    )
    return [...order, ...DEFAULT_COLUMN_ORDER.filter((k) => !order.includes(k))]
}

function moveColumn(order, from, to) {
    const next = order.filter((k) => k !== from)
    next.splice(order.indexOf(to), 0, from)
    return next
}

const DRAG_TYPE = 'text/x-ble-column'

const DESC_FIRST = ['rssi', 'seenCount', 'lastSeen', 'connectable']

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
    columnOrder,
    onColumnOrderChange,
}) {
    const [sort, setSort] = useState({ key: 'rssi', dir: 'desc' })
    const [collapsed, setCollapsed] = useState(collapsible)
    const [dragOverKey, setDragOverKey] = useState(null)

    // The inactive section drops the live-only columns, like the CLI does.
    const order = resolveOrder(columnOrder)
    const columns = order
        .map((key) => COLUMNS.find((c) => c.key === key))
        .filter((c) => !(compact && c.active))

    const handleDrop = (e, targetKey) => {
        e.preventDefault()
        setDragOverKey(null)
        const from = e.dataTransfer.getData(DRAG_TYPE)
        if (from && from !== targetKey) {
            onColumnOrderChange(moveColumn(order, from, targetKey))
        }
    }

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
                : { key, dir: DESC_FIRST.includes(key) ? 'desc' : 'asc' }
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
                                    <th
                                        key={c.key}
                                        className={dragOverKey === c.key ? 'drag-over' : ''}
                                        draggable
                                        title="Click to sort, drag to reorder"
                                        onClick={() => toggleSort(c.key)}
                                        onDragStart={(e) => {
                                            e.dataTransfer.setData(DRAG_TYPE, c.key)
                                            e.dataTransfer.effectAllowed = 'move'
                                        }}
                                        onDragOver={(e) => {
                                            if (e.dataTransfer.types.includes(DRAG_TYPE)) {
                                                e.preventDefault()
                                                setDragOverKey(c.key)
                                            }
                                        }}
                                        onDragLeave={() => setDragOverKey(null)}
                                        onDragEnd={() => setDragOverKey(null)}
                                        onDrop={(e) => handleDrop(e, c.key)}
                                    >
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
                                            {c.render ? c.render(d) : c.value(d)}
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
