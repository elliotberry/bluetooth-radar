async function main(getData) {
  let previousLineCount = 0;

  function moveToTop() {
    process.stdout.write('\x1B[H');
  }
  function oldDisplay(lines) {
    const currentLineCount = lines.length;

    // Move cursor to the top-left corner
    moveToTop();

    // Write new lines
    lines.forEach(line => process.stdout.write(`${line}\n`));

    // If previous output had more lines, clear the remaining lines
    if (previousLineCount > currentLineCount) {
      const extraLines = previousLineCount - currentLineCount;
      for (let i = 0; i < extraLines; i++) {
        process.stdout.write('\x1B[K\n'); // Clear the line and move to the next
      }
    }

    previousLineCount = currentLineCount;
  }
  function refresh() {
    const lines = getData();
    console.clear();
    console.log("------devices------");
    if (lines.length > 0) {
      
      
      console.log(lines.join('\n'));
    }
    else {
      console.log('No devices found');
    }
  }

  // Initial move to top
  moveToTop();
  
  setInterval(refresh, 1000);
}

export default main;