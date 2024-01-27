const formatDeviceInfo = (peripheral) => {
    const addToObjectIfDefined = (obj, key, value) => {
        if (value !== undefined) {
            obj[key] = value
        }
    }

    let obj = {}
    addToObjectIfDefined(obj, 'id', peripheral.id)
    addToObjectIfDefined(obj, 'address', peripheral.address)
    addToObjectIfDefined(obj, 'addressType', peripheral.addressType)
    addToObjectIfDefined(obj, 'connectable', peripheral.connectable)
    addToObjectIfDefined(obj, 'advertisement', peripheral.advertisement)
    addToObjectIfDefined(obj, 'rssi', peripheral.rssi)
    addToObjectIfDefined(obj, 'services', peripheral.services)
    addToObjectIfDefined(obj, 'state', peripheral.state)
    addToObjectIfDefined(obj, 'type', peripheral.type)
    addToObjectIfDefined(obj, 'uuid', peripheral.uuid)
    addToObjectIfDefined(obj, 'lastSeen', peripheral.lastSeen)

   
    addToObjectIfDefined(obj, 'rssi', peripheral.connectable)
    let connectableStr = 'n'
    if (peripheral.connectable && peripheral.connectable === true) {
        connectableStr = 'y'
    }
    obj.connectable = connectableStr
    obj.advertisement = peripheral.advertisement || {}
    obj.name = obj.advertisement.localName || 'Unknown'
    obj.manufacturerData = obj.advertisement.manufacturerData?.toString('hex') || 'N/A'
    return obj
}

export default formatDeviceInfo
