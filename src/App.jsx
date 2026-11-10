import { useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import DeviceTable from './DeviceTable.jsx'
import DetailPane from './DetailPane.jsx'

const STATUS_TEXT = {
    starting: 'Starting Bluetooth scan...',
    scanning: 'Scanning',
    'bluetooth-off': 'Bluetooth is turned off. Turn it on to start scanning.',
    unauthorized:
        'Bluetooth access is not allowed. Enable it in System Settings > Privacy & Security > Bluetooth.',
    'no-adapter': 'No Bluetooth adapter found.',
    error: 'Bluetooth error. Check that this app has Bluetooth permission.',
}

function StatusBanner({ status }) {
    const ok = status.state === 'scanning'
    return (
        <div className={`status ${ok ? 'ok' : 'warn'}`}>
            <span className="dot" />
            <span>{STATUS_TEXT[status.state] ?? status.state}</span>
            {!ok && status.message && (
                <span className="status-detail"> ({status.message})</span>
            )}
        </div>
    )
}

export default function App() {
    const [devices, setDevices] = useState([])
    const [status, setStatus] = useState({ state: 'starting', message: null })
    const [selectedId, setSelectedId] = useState(null)
    const [query, setQuery] = useState('')
    const [platform, setPlatform] = useState(null)
    const [detailOpen, setDetailOpen] = useState(
        () => localStorage.getItem('detailOpen') !== 'false'
    )

    useEffect(() => {
        localStorage.setItem('detailOpen', String(detailOpen))
    }, [detailOpen])

    useEffect(() => {
        let cancelled = false
        const unlisten = []

        invoke('get_devices').then((d) => !cancelled && setDevices(d))
        invoke('get_scan_status').then((s) => !cancelled && setStatus(s))
        invoke('get_app_info').then((i) => !cancelled && setPlatform(i.platform))

        listen('devices-updated', (e) => setDevices(e.payload)).then((fn) =>
            cancelled ? fn() : unlisten.push(fn)
        )
        listen('scan-status', (e) => setStatus(e.payload)).then((fn) =>
            cancelled ? fn() : unlisten.push(fn)
        )

        return () => {
            cancelled = true
            unlisten.forEach((fn) => fn())
        }
    }, [])

    const filtered = useMemo(() => {
        const q = query.trim().toLowerCase()
        if (!q) return devices
        return devices.filter((d) =>
            [
                d.word,
                d.id,
                d.name,
                d.address,
                d.appearanceLabel,
                ...d.manufacturers.flatMap((m) => [m.companyName, ...m.decoded.map((f) => f.value)]),
                ...d.services.map((s) => s.name),
            ]
                .filter(Boolean)
                .some((v) => v.toLowerCase().includes(q))
        )
    }, [devices, query])

    const active = filtered.filter((d) => d.active)
    const inactive = filtered.filter((d) => !d.active)
    const selected = devices.find((d) => d.id === selectedId) ?? null

    return (
        <div className="app">
            <header className="toolbar">
                <h1>BLE Radar</h1>
                <span className="counts">
                    {devices.filter((d) => d.active).length} active /{' '}
                    {devices.length} total
                </span>
                <input
                    className="search"
                    type="search"
                    placeholder="Filter by word, id, name, company, info..."
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                />
                <button onClick={() => invoke('clear_devices')}>Clear</button>
                <button
                    className={`icon-button ${detailOpen ? 'active' : ''}`}
                    onClick={() => setDetailOpen((open) => !open)}
                    aria-label={detailOpen ? 'Hide details panel' : 'Show details panel'}
                    aria-pressed={detailOpen}
                    title={detailOpen ? 'Hide details panel' : 'Show details panel'}
                >
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                        <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" stroke="currentColor" />
                        <line x1="10" y1="2.5" x2="10" y2="13.5" stroke="currentColor" />
                        {detailOpen && <rect x="10.5" y="3" width="4" height="10" fill="currentColor" opacity="0.35" />}
                    </svg>
                </button>
            </header>
            <StatusBanner status={status} />
            <main className="content">
                <div className="tables">
                    <DeviceTable
                        title="Active"
                        devices={active}
                        selectedId={selectedId}
                        onSelect={setSelectedId}
                        emptyText="No active devices"
                    />
                    <DeviceTable
                        title="Inactive"
                        devices={inactive}
                        selectedId={selectedId}
                        onSelect={setSelectedId}
                        collapsible
                        compact
                        emptyText="No inactive devices"
                    />
                </div>
                <DetailPane device={selected} platform={platform} />
            </main>
        </div>
    )
}
