import { deviceColor } from './color.js'

function formatTime(ms) {
    return ms ? new Date(ms).toLocaleString() : 'n/a'
}

function Row({ label, children }) {
    return (
        <>
            <dt>{label}</dt>
            <dd>{children ?? 'n/a'}</dd>
        </>
    )
}

export default function DetailPane({ device }) {
    if (!device) {
        return (
            <aside className="detail">
                <p className="empty">Select a device to see its details.</p>
            </aside>
        )
    }

    return (
        <aside className="detail">
            <h2 style={{ color: deviceColor(device.id) }}>{device.word}</h2>
            <p className="detail-name">{device.name}</p>
            <dl>
                <Row label="Status">{device.active ? 'Active' : 'Inactive'}</Row>
                <Row label="ID">
                    <code>{device.id}</code>
                </Row>
                <Row label="Address">{device.address}</Row>
                <Row label="Company">
                    {device.companyName &&
                        `${device.companyName}${
                            device.companyId && device.companyName !== device.companyId
                                ? ` (${device.companyId})`
                                : ''
                        }`}
                </Row>
                <Row label="Manufacturer data">
                    {device.manufacturerDataHex && <code>{device.manufacturerDataHex}</code>}
                </Row>
                <Row label="RSSI">{device.rssi != null && `${device.rssi} dBm`}</Row>
                <Row label="TX power">
                    {device.txPower != null && `${device.txPower} dBm`}
                </Row>
                <Row label="Services">
                    {device.services.length > 0 && (
                        <ul className="services">
                            {device.services.map((s) => (
                                <li key={s}>
                                    <code>{s}</code>
                                </li>
                            ))}
                        </ul>
                    )}
                </Row>
                <Row label="Times seen">{(device.seenCount ?? 0).toLocaleString()}</Row>
                <Row label="First seen">{formatTime(device.firstSeen)}</Row>
                <Row label="Last seen">{formatTime(device.lastSeen)}</Row>
            </dl>
        </aside>
    )
}
