import fs from 'fs/promises'
import { generate, count } from 'random-words'
import formatDeviceInfo from './format-device-info.js'
var loggedDevices = {}

// Create a variable to store the last write timestamp
let lastWriteTimestamp = 0

// Your async function to write JSON to a file
async function persistState() {
    const currentTime = Date.now()

    // Calculate the time elapsed since the last write
    const timeElapsed = currentTime - lastWriteTimestamp

    // Check if at least one second has passed since the last write
    if (timeElapsed >= 1000) {
        try {
            // Write the JSON data to the file
            await fs.writeFile('./data.json', JSON.stringify(loggedDevices))

            // Update the last write timestamp
            lastWriteTimestamp = currentTime
        } catch (error) {
            console.error('Error writing to file:', error)
        }
    }
}

const initPersistedState = async () => {
    try {
        let info = {}
        let fileData = await fs.readFile('./data.json', 'utf-8')
        if (fileData) {
            info = JSON.parse(fileData)
            for (let key in info) {
                info[key].active = false
            }
        }

        loggedDevices = info
    } catch (error) {
        console.log(error)
    }
}

export const logDevice = async (device) => {
    let obj = formatDeviceInfo(device)
    if (!loggedDevices[device.id] || !loggedDevices[device.id].word || typeof loggedDevices[device.id].word !== 'string') {
        obj.word = await generate()
    }
    obj.active = true
    obj.lastSeen = Date.now()
    loggedDevices[obj.id] = obj
    await persistState()
    return obj
}
export const getLoggedDevices = async () => {
    await initPersistedState()
    return loggedDevices
}
