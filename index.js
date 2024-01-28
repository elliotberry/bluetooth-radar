import noble from 'noble'
import blessed from 'blessed'
import contrib from 'blessed-contrib'
import { setInterval } from 'timers/promises'

import formatDeviceInfo from './format-device-info.js'
import { logDevice, getLoggedDevices } from './state.js'

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

const initNoble = () => {
    // Update the table with discovered devices
    // Start scanning when BLE adapter is powered on
    noble.on('stateChange', (state) => {
        if (state === 'poweredOn') {
            noble.startScanning([], true)
        } else {
            noble.stopScanning()
        }
    })

    noble.on('discover', async (peripheral) => {
        let id = peripheral.id

        let finalOut = await logDevice(peripheral)

        devices[id] = finalOut
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

// Update UI every 100ms
async function main() {
    let olddevices = await getLoggedDevices()
    devices = olddevices
    initNoble()
    initBlessed()

    for await (const _ of setInterval(600)) {
        updateUI()
    }
}

main()
