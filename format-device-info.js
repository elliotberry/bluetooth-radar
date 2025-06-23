import fs from 'fs/promises'
var loggedDevices = {}
import companies from './companies.js'
function filterOutFunctions(obj) {
    const filteredObj = {}

    for (const key in obj) {
        if (typeof obj[key] !== 'function') {
            if (typeof obj[key] === 'object' && obj[key] !== null) {
                filteredObj[key] = filterOutFunctions(obj[key])
            } else {
                filteredObj[key] = obj[key]
            }
        }
    }

    return filteredObj
}
const manufacturerDataToString = (manufacturerData) => {
    const companyId = manufacturerData.readUInt16LE(0) // 16-bit Company Identifier
    const payload = manufacturerData.slice(2)
    return { companyId, payload: payload.toString('hex') }
}

const formatDeviceInfo = (peripheral) => {
    let id = peripheral.id || peripheral.uuid

    let obj = {
        id: id,
        address: peripheral.address,
        addressType: peripheral.addressType,
        connectable: peripheral.connectable,
        advertisement: peripheral.advertisement || {},
        rssi: peripheral.rssi,
        services: peripheral.services,
        state: peripheral.state,
        type: peripheral.type,
        uuid: peripheral.uuid,
        name: peripheral.advertisement.localName || 'Unknown',
        manufacturerData:
            peripheral.advertisement.manufacturerData?.toString('hex') || 'N/A',
    }
    if (peripheral.advertisement.manufacturerData) {
        let { companyId, payload } = manufacturerDataToString(
            peripheral.advertisement.manufacturerData
        )
        let companyIdHex = `0x${companyId.toString(16).padStart(4, '0')}`.toUpperCase();
        if (companies[companyIdHex]) {
            obj.companyName = companies[companyIdHex]
        }
        else {
            obj.companyName = companyIdHex;
        }
       // obj.companyId = `0x${companyId.toString(16).padStart(4, '0')}`;
        obj.manufacturerDataHex = payload
    }
    let connectableStr = 'n'
    if (peripheral.connectable && peripheral.connectable === true) {
        connectableStr = 'y'
    }
    obj.connectableSymbol = connectableStr

    return obj
}

export default formatDeviceInfo
