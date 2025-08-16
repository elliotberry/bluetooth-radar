function stringToColor(str) {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
        hash = str.charCodeAt(i) + ((hash << 5) - hash)
    }
    const r = (hash >> 0) & 0xff
    const g = (hash >> 8) & 0xff
    const b = (hash >> 16) & 0xff

    return `\x1b[38;2;${r};${g};${b}m${str}\x1b[0m`
}

async function main(getData) {
    let previousLineCount = 0

    function moveToTop() {
        process.stdout.write('\x1B[H')
    }
    function oldDisplay(lines) {
        const currentLineCount = lines.length

        // Move cursor to the top-left corner
        moveToTop()

        // Write new lines
        lines.forEach((line) =>
            process.stdout.write(`${stringToColor(line)}\n`)
        )

        // If previous output had more lines, clear the remaining lines
        if (previousLineCount > currentLineCount) {
            const extraLines = previousLineCount - currentLineCount
            for (let i = 0; i < extraLines; i++) {
                process.stdout.write('\x1B[K\n') // Clear the line and move to the next
            }
        }

        previousLineCount = currentLineCount
    }
    function refresh() {
        const lines = getData()
        console.clear()

        if (lines.length > 0) {
            let newLines = lines.map((line) => {
                let str = stringToColor(line)
                return str
            })
            console.log(newLines.join('\n'))
        } else {
            console.log('No devices found')
        }
    }

    // Initial move to top
    moveToTop()

    setInterval(refresh, 1000)
}

export default main
