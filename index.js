//import display from './terminal-log.js'
import chalk from 'chalk'
import setup from './blessed.js'
import terminalLog from './terminal-log.js'
var state
import State from './state.js'
const truncateId = (str) => {
    if (str) {
        if (str.length > 12) {
            return `${str.slice(0, 4)}...${str.slice(-4)}`
        }
        return str
    }
}

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
