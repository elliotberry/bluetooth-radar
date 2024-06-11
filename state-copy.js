import fs from 'fs/promises'
import { generate, count } from 'random-words'
import formatDeviceInfo from './format-device-info.js'

class State {
    constructor() {
        this.fileSync = false
        this.devices = {}
        this.lastWriteTimestamp = 0
    }
    activeDevices() {
        //filter out of object

        return Object.keys(this.devices).filter(
            (key) => this.devices[key].active
        ).map((key) => this.devices[key])
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
        if (this.fileSync === false) {
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
    }
    readCharacteristic(characteristic) {
        return new Promise((resolve, reject) => {
          characteristic.read((error, data) => {
            if (error) {
              reject(error);
            } else {
              resolve(data.toString('utf8'));
            }
          });
        });
      }
    async connectDevice(deviceId) {
        peripheral.connect((error) => {
            if (error) {
              reject(error);
            } else {
              this.connected = true;
              console.log(`Connected to ${this.name} (${this.address})`);
              peripheral.discoverAllServicesAndCharacteristics((error, services) => {
                if (error) {
                  reject(error);
                } else {
                  services.forEach((service) => {
                    service.discoverAllCharacteristics((error, characteristics) => {
                      if (error) {
                        reject(error);
                      } else {
                        this.characteristics.push(...characteristics);
                      }
                    });
                  });
                  resolve();
                }
              });
            }
          });
    }
    async logDevice(device) {
        this.fileSync = false
        let obj = formatDeviceInfo(device)
        if (
            !this.devices[device.id] ||
            !this.devices[device.id].word ||
            typeof this.devices[device.id].word !== 'string'
        ) {
            obj.word = await generate()
        }
        obj.active = true
        obj.lastSeen = Date.now()
        this.devices[obj.id] = obj
       // await this.persistState()

        return obj
    }

    async getLoggedDevices() {
       // await this.initPersistedState()
        return this.devices
    }
}

export default State
