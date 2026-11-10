#!/usr/bin/env node
import terminalLog from './lib/terminal-log.js'
import State from './lib/state.js'
import truncateId from './lib/truncate-id.js'
var state


const formatStateOutput = () => {
    let active = []
    let inactive = []
    state.getDevices().forEach((e) => {
  
        if (e.active) {
            active.push(
                `${e.word} ${truncateId(e.id)}, ${e.name}, ${e.companyName}, ${
                    e.rssi
                }`
            )
        } else {
            inactive.push(
                `${`${e.word} ${truncateId(e.id)}, ${e.name}`}`
            )
        }
    })
    active.sort((a, b) => {
        return a.rssi - b.rssi
    })
    inactive.sort((a, b) => {
        return a.rssi - b.rssi
    })
   
    let toDisplay = state.getDevices().map((e) => {
        return `${truncateId(e.uuid)}, ${e.name}, ${
            e.rssi
        }`
    })
    return active;
}

async function main() {
    state = new State()
    await state.init()
 
    terminalLog(function() {
        return formatStateOutput()
    })

}

main()
