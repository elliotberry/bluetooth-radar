import { useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import DeviceTable from './DeviceTable.jsx'
import DetailPane from './DetailPane.jsx'

const STATUS_TEXT = {
    starting: 'Starting Bluetooth scan...',
    scanning: 'Scanning',
    'bluetooth-off': 'Bluetooth is turned off. Turn it on to start scanning.',
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

    useEffect(() => {
        let cancelled = false
        const unlisten = []

        invoke('get_devices').then((d) => !cancelled && setDevices(d))
        invoke('get_scan_status').then((s) => !cancelled && setStatus(s))

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
            [d.word, d.id, d.name, d.companyName, d.address]
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
                    placeholder="Filter by word, id, name, company..."
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                />
                <button onClick={() => invoke('clear_devices')}>Clear</button>
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
                <DetailPane device={selected} />
            </main>
        </div>
    )
}
