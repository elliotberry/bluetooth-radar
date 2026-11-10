import fs from 'node:fs/promises'
import deterministicID from './deterministic-id.js'
import formatDeviceInfo from './format-device-info.js'
import noble from 'noble'
class State {
    constructor() {
        this.fileSync = false
        this.devices = {}
        this.lastWriteTimestamp = 0
        this.noble = noble
    }
    truncateId(str) {
        if (str.length > 12) {
            return `${str.slice(0, 4)}...${str.slice(-4)}`
        }
        return str
    }
    initNoble() {
        // Update the table with discovered devices
        // Start scanning when BLE adapter is powered on
        noble.on('stateChange', (state) => {
            if (state === 'poweredOn') {
                noble.startScanning([], true)
                //.log('Scanning for BLE devices...')
            } else {
                noble.stopScanning()
            }
        })

        noble.on('discover', async (peripheral) => {
            await this.logDevice(peripheral)
        })
    }
    getDevices() {
        if (this.devices && Object.keys(this.devices).length > 0) {
            return Object.keys(this.devices).map((key) => this.devices[key])
        } else {
            return []
        }
    }
    activeDevices() {
        //filter out of object

        return Object.keys(this.devices)
            .filter((key) => this.devices[key].active)
            .map((key) => this.devices[key])
    }
    async loadState() {
        try {
            let info = {}
            let fileData = await fs.readFile('./data.json', 'utf-8')
            if (fileData) {
                info = JSON.parse(fileData)
                for (let key in info) {
                    info[key].active = false
                }
            }
            this.devices = info
            this.fileSync = true
        } catch (error) {
            console.log(error)
        }
    }
    async persistState() {
        const currentTime = Date.now()
        const timeElapsed = currentTime - this.lastWriteTimestamp

        if (timeElapsed >= 1000) {
            try {
                await fs.writeFile('./data.json', JSON.stringify(this.devices))
                this.lastWriteTimestamp = currentTime
                this.fileSync = true
            } catch (error) {
                console.error('Error writing to file:', error)
            }
        }
    }

    async init() {
        this.initNoble()
        await this.loadState()
    }

    async deviceSeen(theDevice) {
        let newInfo = {
            rssi: theDevice.rssi,
            // advertisement: theDevice.advertisement,
            // services: theDevice.services,
            state: theDevice.state,
        }
        await this.updateDevice(theDevice.uuid, newInfo)
    }

    async updateDevice(id, object) {
        let device = this.devices[id]

        let newDevice = Object.assign(device, object)
        this.devices[id] = newDevice
        await this.persistState()
        return this.devices[id]
    }

    async getInformation(peripheral) {
        if (peripheral.connectable) {
            try {
                let vals = await this.connectAndRead(peripheral)
                // console.log(vals)
            } catch (error) {
                console.error(`Failed to connect and read: ${error}`)
            }
        }
    }
    connectAndRead(peripheral) {
        return new Promise((resolve, reject) => {
            peripheral.connect(async (error) => {
                if (error) {
                    return reject(`Error connecting to peripheral: ${error}`)
                }
                console.log(
                    `Connected to peripheral: ${peripheral.advertisement.localName}`
                )

                peripheral.discoverAllServicesAndCharacteristics(
                    async (error, services, characteristics) => {
                        if (error) {
                            return reject(
                                `Error discovering services and characteristics: ${error}`
                            )
                        }

                        const characteristicData = {}
                        const readPromises = characteristics.map(
                            (characteristic) => {
                                return new Promise((resolve) => {
                                    if (
                                        characteristic.properties.includes(
                                            'read'
                                        )
                                    ) {
                                        characteristic.read((error, data) => {
                                            if (error) {
                                                console.error(
                                                    `Error reading characteristic: ${error}`
                                                )
                                                resolve()
                                            } else {
                                                characteristicData[
                                                    characteristic.uuid
                                                ] = data.toString('utf8')
                                                resolve()
                                            }
                                        })
                                    } else {
                                        resolve()
                                    }
                                })
                            }
                        )

                        await Promise.all(readPromises)

                        // Disconnect after reading
                        peripheral.disconnect((error) => {
                            if (error) {
                                return reject(
                                    `Error disconnecting from peripheral: ${error}`
                                )
                            }
                            // console.log('Disconnected from peripheral')
                            resolve(
                                Object.keys(characteristicData).length > 0
                                    ? characteristicData
                                    : {}
                            )
                        })
                    }
                )
            })
        })
    }
    async logDevice(peripheral) {
        //  console.log('Discovered device:', peripheral.id)
        if (this.devices[peripheral.uuid]) {
            this.deviceSeen(peripheral)
        } else {
            this.devices[peripheral.uuid] = {}
            let obj = formatDeviceInfo(peripheral)

            obj.word = deterministicID(obj.uuid)

            obj.active = true
            obj.lastSeen = Date.now()
            //  await this.connectAndRead(peripheral);
            this.updateDevice(obj.id, obj)
        }
        return this.devices[peripheral.id]
    }

    async getLoggedDevices() {
        // await this.initPersistedState()
        return this.devices
    }
}

export default State
