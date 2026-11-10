import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { deviceColor } from './color.js'
import Sparkline from './Sparkline.jsx'
import { TREND_SYMBOL, formatDistance, formatInterval, formatTime } from './format.js'

function Row({ label, children }) {
    if (children == null || children === false || children === '') return null
    return (
        <>
            <dt>{label}</dt>
            <dd>{children}</dd>
        </>
    )
}

function Section({ title, children }) {
    return (
        <section className="detail-section">
            <h3>{title}</h3>
            {children}
        </section>
    )
}

function Decoded({ fields }) {
    if (!fields.length) return null
    return (
        <ul className="decoded">
            {fields.map((f, i) => (
                <li key={i}>
                    <span className="muted">{f.label}:</span> {f.value}
                </li>
            ))}
        </ul>
    )
}

function addressValue(device, platform) {
    if (device.address) return <code>{device.address}</code>
    if (platform === 'macos') {
        return <span className="muted">Hidden by macOS (it only exposes the device ID)</span>
    }
    return null
}

const GATT_FIELDS = [
    ['deviceName', 'Device name'],
    ['manufacturerName', 'Manufacturer'],
    ['modelNumber', 'Model'],
    ['hardwareRevision', 'Hardware'],
    ['firmwareRevision', 'Firmware'],
    ['softwareRevision', 'Software'],
]

function QueryDevice({ device }) {
    const [state, setState] = useState({ busy: false, error: null })

    // Forget a previous error when switching devices.
    useEffect(() => setState({ busy: false, error: null }), [device.id])

    const run = async () => {
        setState({ busy: true, error: null })
        try {
            await invoke('query_device', { id: device.id })
            setState({ busy: false, error: null })
        } catch (e) {
            setState({ busy: false, error: String(e) })
        }
    }

    const gatt = device.gatt
    const blocked = device.connectable === false
    return (
        <Section title="Query device">
            <p className="muted">
                Connects to the device and reads its name, model, firmware and battery. This
                actively talks to it, can take up to a minute, and some devices may react to it.
            </p>
            <button onClick={run} disabled={state.busy || blocked || !device.active}>
                {state.busy ? 'Querying...' : gatt ? 'Query again' : 'Query device'}
            </button>
            {blocked && <p className="muted">This device says it is not connectable.</p>}
            {!blocked && !device.active && (
                <p className="muted">The device has to be nearby and advertising.</p>
            )}
            {state.error && <p className="error">{state.error}</p>}
            {gatt && (
                <dl className="gatt">
                    {GATT_FIELDS.map(([key, label]) => (
                        <Row key={key} label={label}>
                            {gatt[key]}
                        </Row>
                    ))}
                    <Row label="Battery">
                        {gatt.batteryPercent != null && `${gatt.batteryPercent}%`}
                    </Row>
                    <Row label="Appearance">{gatt.appearanceLabel}</Row>
                    <Row label="Queried">{formatTime(gatt.queriedAt)}</Row>
                </dl>
            )}
            {gatt && GATT_FIELDS.every(([k]) => !gatt[k]) && gatt.batteryPercent == null && (
                <p className="muted">The device did not expose any of the standard fields.</p>
            )}
        </Section>
    )
}

export default function DetailPane({ device, platform }) {
    if (!device) {
        return (
            <aside className="detail">
                <p className="empty">Select a device to see its details.</p>
            </aside>
        )
    }

    const differentNames =
        device.localName &&
        device.advertisementName &&
        device.localName !== device.advertisementName

    return (
        <aside className="detail">
            <h2 style={{ color: deviceColor(device.id) }}>{device.word}</h2>
            <p className="detail-name">{device.name}</p>
            <dl>
                <Row label="Status">{device.active ? 'Active' : 'Inactive'}</Row>
                <Row label="ID">
                    <code>{device.id}</code>
                </Row>
                <Row label="Address">{addressValue(device, platform)}</Row>
                <Row label="Address type">{device.addressType}</Row>
                <Row label="Rotating address">
                    {device.likelyRandom && `Likely. ${device.randomReason}`}
                </Row>
                <Row label="Connectable">
                    {device.connectable != null && (device.connectable ? 'Yes' : 'No')}
                </Row>
                {differentNames && (
                    <>
                        <Row label="Local name">{device.localName}</Row>
                        <Row label="Cached name">{device.advertisementName}</Row>
                    </>
                )}
                <Row label="Appearance">{device.appearanceLabel}</Row>
                <Row label="RSSI">
                    {device.rssi != null && (
                        <>
                            {device.rssi} dBm
                            {device.rssiTrend && (
                                <span className="trend">
                                    {' '}
                                    {TREND_SYMBOL[device.rssiTrend]} {device.rssiTrend}
                                </span>
                            )}
                        </>
                    )}
                </Row>
                <Row label="Distance">
                    {device.distanceM != null && (
                        <>
                            {formatDistance(device.distanceM)}{' '}
                            <span className="muted">(rough estimate)</span>
                        </>
                    )}
                </Row>
                <Row label="TX power">
                    {device.txPower != null && `${device.txPower} dBm`}
                </Row>
                <Row label="Advertises every">{formatInterval(device.advIntervalMs)}</Row>
                <Row label="Times seen">{(device.seenCount ?? 0).toLocaleString()}</Row>
                <Row label="First seen">{formatTime(device.firstSeen)}</Row>
                <Row label="Last seen">{formatTime(device.lastSeen)}</Row>
            </dl>

            <div className="spark" style={{ color: deviceColor(device.id) }}>
                <Sparkline values={device.rssiHistory} />
            </div>

            {device.manufacturers.length > 0 && (
                <Section title="Manufacturer data">
                    {device.manufacturers.map((m) => (
                        <div className="entry" key={m.companyId}>
                            <div>
                                {m.companyName}{' '}
                                {m.companyName !== m.companyId && (
                                    <span className="muted">({m.companyId})</span>
                                )}
                            </div>
                            <Decoded fields={m.decoded} />
                            {m.ascii && (
                                <div>
                                    <span className="muted">Text:</span> {m.ascii}
                                </div>
                            )}
                            <code className="hex">{m.dataHex || '(empty)'}</code>
                        </div>
                    ))}
                </Section>
            )}

            {device.serviceData.length > 0 && (
                <Section title="Service data">
                    {device.serviceData.map((s) => (
                        <div className="entry" key={s.uuid}>
                            <div>
                                {s.name ?? 'Unnamed service'}{' '}
                                <span className="muted">({s.uuid})</span>
                            </div>
                            <Decoded fields={s.decoded} />
                            {s.ascii && (
                                <div>
                                    <span className="muted">Text:</span> {s.ascii}
                                </div>
                            )}
                            <code className="hex">{s.dataHex || '(empty)'}</code>
                        </div>
                    ))}
                </Section>
            )}

            {device.services.length > 0 && (
                <Section title="Advertised services">
                    <ul className="services">
                        {device.services.map((s) => (
                            <li key={s.uuid}>
                                {s.name ?? 'Unnamed service'}{' '}
                                <span className="muted">({s.uuid})</span>
                            </li>
                        ))}
                    </ul>
                </Section>
            )}

            <QueryDevice device={device} />
        </aside>
    )
}
