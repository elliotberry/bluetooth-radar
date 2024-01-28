import fs from 'fs/promises'
var loggedDevices = {}



function filterOutFunctions(obj) {
    const filteredObj = {};
  
    for (const key in obj) {
      if (typeof obj[key] !== 'function') {
        if (typeof obj[key] === 'object' && obj[key] !== null) {
          filteredObj[key] = filterOutFunctions(obj[key]);
        } else {
          filteredObj[key] = obj[key];
        }
      }
    }
  
    return filteredObj;
  }
const formatDeviceInfo = (peripheral) => {
   
    let obj = {
        id: peripheral.id,
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
        manufacturerData: peripheral.advertisement.manufacturerData?.toString('hex') || 'N/A'
    }

    let connectableStr = 'n'
    if (peripheral.connectable && peripheral.connectable === true) {
        connectableStr = 'y'
    }
    obj.connectableSymbol = connectableStr
  
    return obj
}

export default formatDeviceInfo
