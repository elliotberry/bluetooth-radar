import noble from 'noble'
import blessed from 'blessed'
import contrib from 'blessed-contrib'
import { setInterval } from 'timers/promises'
import fs from 'fs/promises'
import formatDeviceInfo from './format-device-info.js'

var screen
var table
var devices = {}
const initBlessed = () => {
    // Terminal UI setup
    screen = blessed.screen({
        smartCSR: true,
        title: 'BLE Device Scanner',
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
        //  style: {
        //      selected: {
        //           bg: 'green',
        //      },
        //  },
        select: function () {
            //   fs.writeFile('test.txt', 'Hello World!', function (err) {
            //       if (err) return console.log(err)
            //       console.log('Hello World > test.txt')
            //   })
        },
        columnSpacing: 2,
        columnWidth: [45, 45, 25, 25],
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

    noble.on('discover', (peripheral) => {
        const id = peripheral.id
        const obj = formatDeviceInfo(peripheral)

        devices[id] = obj
    })
}

// Function to update the UI table
async function updateUI() {
    let data = Object.values(devices).map((device) => {
        const row = [device.name, device.rssi, device.connectable]
        return row
    })

    table.setData({
        headers: ['Name', 'RSSI', 'Connectable'],
        data,
    })
    /*
    var box = blessed.box({
        top: 'center',
        left: 'center',
        width: '50%',
        height: '50%',
        content: 'Hello {bold}world{/bold}!',
        tags: true,
        border: {
            type: 'line',
        },
        style: {
            fg: 'white',
            bg: 'magenta',
            border: {
                fg: '#f0f0f0',
            },
            hover: {
                bg: 'green',
            },
        },
    })

    // Append our box to the screen.
    screen.append(box) */
    screen.render()
}

// Update UI every 100ms
async function main() {
    initNoble()
    initBlessed()
    

    for await (const _ of setInterval(100)) {
        updateUI()
    }
}

main()
