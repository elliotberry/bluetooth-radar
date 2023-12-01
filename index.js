import noble from 'noble';
import blessed from 'blessed';
import { setInterval } from 'timers/promises';
import fs from 'fs/promises';
// Terminal UI setup
const screen = blessed.screen({
  smartCSR: true,
  title: 'BLE Device Scanner',
});

const list = blessed.list({
  parent: screen,
  width: '100%',
  height: '100%',
  keys: true,
  vi: true,
  mouse: true,
  tags: true,
  border: {
    type: 'line',
  },
  style: {
    selected: {
      bg: 'green',
    },
  },
});

// Start scanning when BLE adapter is powered on
noble.on('stateChange', (state) => {
  if (state === 'poweredOn') {
    noble.startScanning([], true);
  } else {
    noble.stopScanning();
  }
});

// Update the list with discovered devices
const devices = new Map();

noble.on('discover', (peripheral) => {
  const { id, advertisement, rssi } = peripheral;
  fs.writeFile(`${id}.txt`, JSON.stringify(peripheral, null, 2));
  const name = advertisement.localName || 'Unknown';
  const manufacturerData = advertisement.manufacturerData?.toString('hex') || 'N/A';
  devices.set(id, { name, manufacturerData, rssi });
});

// Function to update the UI list
async function updateUI() {
  list.clearItems();
  devices.forEach((device, id) => {
fs.writeFile(`${id}.txt`, JSON.stringify(device, null, 2));
    list.addItem(`ID: ${id}, Name: ${device.name}, Manufacturer: ${device.manufacturerData}, RSSI: ${device.rssi}`);
  });
  screen.render();
}

// Update UI every 100ms
async function main() {
  for await (const _ of setInterval(100)) {
    updateUI();
  }
}

// Exit gracefully
screen.key(['escape', 'q', 'C-c'], () => process.exit(0));

// Run the app
main();
