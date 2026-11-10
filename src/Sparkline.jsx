// Tiny RSSI-over-time chart. Scale is fixed (-100 to -30 dBm) so strength
// is comparable between devices; the line is oldest-to-newest, one sample per second.
export default function Sparkline({ values, width = 240, height = 40 }) {
    if (!values || values.length < 2) {
        return <span className="muted">Collecting samples...</span>
    }
    const min = -100
    const max = -30
    const step = width / (values.length - 1)
    const points = values
        .map((v, i) => {
            const clamped = Math.max(min, Math.min(max, v))
            const y = height - ((clamped - min) / (max - min)) * height
            return `${(i * step).toFixed(1)},${y.toFixed(1)}`
        })
        .join(' ')
    return (
        <svg
            className="sparkline"
            width={width}
            height={height}
            viewBox={`0 0 ${width} ${height}`}
            role="img"
            aria-label="Signal strength over the last two minutes"
        >
            <polyline points={points} fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
    )
}
