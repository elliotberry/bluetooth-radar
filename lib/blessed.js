import blessed from '@farjs/blessed';

async function main() {
  // Create a screen object.
  const screen = blessed.screen({
    smartCSR: true,
    title: 'Bluetooth Devices Monitor',
  });

  // Create a box for each device
  const container = blessed.box({
    top: 'center',
    left: 'center',
    width: '90%',
    height: '90%',
    tags: true,
    border: {
      type: 'line',
    },
    style: {
      border: {
        fg: 'cyan',
      },
    },
    scrollable: true,
    alwaysScroll: true,
  });

  screen.append(container);
  const deviceBoxes = {};
  // Function to update the devices display
  function updateDevices(devices) {
    devices.forEach(device => {
      if (!deviceBoxes[device.id]) {
        // Create a new box for each new device
        const deviceBox = blessed.box({
          top: Object.keys(deviceBoxes).length,
          left: 0,
          width: '100%',
          height: 3, // Adjust height if you need more space per device
          content: '',
          tags: true,
        });
        container.append(deviceBox);
        deviceBoxes[device.id] = deviceBox;
      }

      // Update the content of the device box
      deviceBoxes[device.id].setContent(
        `fqwf`,
      );
    });

    // Re-render the screen to show the updates
    screen.render();
  }

  // Quit on Escape, q, or Control-C.
  screen.key(['escape', 'q', 'C-c'], function() {
    return process.exit(0);
  });

  screen.render();
  return updateDevices;
}

export default main;