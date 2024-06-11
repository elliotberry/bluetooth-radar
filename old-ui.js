import blessed from 'blessed'
import contrib from 'blessed-contrib'

var screen
var table
var devices = {}

let propertiesToShow = ['word', 'id', 'name', 'rssi', 'connectable']
let columnWidths = propertiesToShow.map(
    (property) => 100 / propertiesToShow.length
)

const initBlessed = () => {
    // Terminal UI setup
    screen = blessed.screen({
        smartCSR: true,
        title: 'BLE Radar',
    })
    screen.key(['escape', 'q', 'C-c'], () => process.exit(0))
    table = contrib.table({
        parent: screen,
        keys: true,
        interactive: true,
        vi: true,
        mouse: true,
        tags: true,
        border: {
            type: 'line',
        },

        columnSpacing: 2,
        columnWidth: columnWidths,
    })
}


// Function to update the UI table
async function updateUI() {
    let out = []

    for (let key in devices) {
        let device = devices[key]

        if (device.active === true) {
            let thisArray = []
            for (let property of propertiesToShow) {
                let val = device[property]
                if (val === undefined) {
                    val = 'N/A'
                }
                thisArray.push(val)
            }
            out.push(thisArray)
        }
    }

    table.setData({
        headers: propertiesToShow,
        data: out,
    })

    screen.render()
}

