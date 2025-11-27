let currentX = 125;
let currentY = 25;
const radius = 7;

const gridEl = document.getElementById('map-grid');
const yAxisEl = document.getElementById('y-axis-container');
const xAxisEl = document.getElementById('x-axis-container');
const headerCoordsEl = document.getElementById('header-coords');
const inputX = document.getElementById('input-x');
const inputY = document.getElementById('input-y');
const detailsPanel = document.getElementById('details-panel');
const tooltip = document.getElementById('tile-info');

function renderMap(centerX, centerY) {
  gridEl.innerHTML = '';
  yAxisEl.innerHTML = '';
  xAxisEl.innerHTML = '';

  headerCoordsEl.innerText = `(${centerX}|${centerY})`;
  inputX.value = centerX;
  inputY.value = centerY;

  for (let y = centerY + radius; y >= centerY - radius; y--) {
    const div = document.createElement('div');
    div.className = `y-label ${y === centerY ? 'highlight-axis' : ''}`;
    div.innerText = y;
    yAxisEl.appendChild(div);
  }

  for (let x = centerX - radius; x <= centerX + radius; x++) {
    const div = document.createElement('div');
    div.className = `x-label ${x === centerX ? 'highlight-axis' : ''}`;
    div.innerText = x;
    xAxisEl.appendChild(div);
  }

  for (let y = centerY + radius; y >= centerY - radius; y--) {
    for (let x = centerX - radius; x <= centerX + radius; x++) {
      const tile = document.createElement('div');
      const isCenter = (x === centerX && y === centerY);

      const seed = Math.abs((x * 123) + (y * 456));
      let content = '';
      let typeClass = '';
      let title = `Abandoned Valley (${x}|${y})`;

      if (isCenter) {
        typeClass = 'is-own-village';
        content = '🏠';
        title = `MyVillage (${x}|${y})`;
      } else if (seed % 17 === 0) {
        typeClass = 'is-village';
        content = '🏠';
        title = `Village (${x}|${y})`;
      } else if (seed % 23 === 0) {
        typeClass = 'oasis-wood';
        content = '🌲';
        title = `Lumber Oasis (${x}|${y})`;
      } else if (seed % 29 === 0) {
        typeClass = 'oasis-clay';
        content = '🧱';
        title = `Clay Oasis (${x}|${y})`;
      } else if (seed % 31 === 0) {
        typeClass = 'oasis-iron';
        content = '⛰️';
        title = `Iron Oasis (${x}|${y})`;
      } else if (seed % 37 === 0) {
        typeClass = 'oasis-crop';
        content = '🌾';
        title = `Crop Oasis (${x}|${y})`;
      }

      tile.className = `tile ${typeClass}`;
      tile.innerHTML = `<span class="tile-content">${content}</span>`;

      tile.onmouseenter = (e) => showTooltip(e, title);
      tile.onmouseleave = () => hideTooltip();
      tile.onmouseover = () => showDetails(title, x, y, content);

      gridEl.appendChild(tile);
    }
  }
}

function moveMap(dx, dy) {
  currentX += dx;
  currentY += dy;
  renderMap(currentX, currentY);
}

function goToCoords() {
  const x = parseInt(inputX.value) || 0;
  const y = parseInt(inputY.value) || 0;
  currentX = x;
  currentY = y;
  renderMap(currentX, currentY);
}

function showDetails(title, x, y, icon) {
  let html = `
            <div class="text-center mb-4">
                <div class="text-4xl mb-2">${icon || '🌲'}</div>
                <div class="font-bold text-sm text-gray-800">${title}</div>
                <div class="text-xs text-gray-500 mt-1">Coordinate: <span class="font-mono font-bold text-black">${x}|${y}</span></div>
            </div>
            <table class="w-full text-xs">
                <tr class="border-b border-gray-200"><td class="py-2 text-gray-600">Giocatore</td><td class="py-2 text-right font-bold text-black">${icon === '🏠' ? 'Giocatore_' + Math.abs(x + y) : '-'}</td></tr>
                <tr class="border-b border-gray-200"><td class="py-2 text-gray-600">Popolazione</td><td class="py-2 text-right text-black font-semibold">${icon === '🏠' ? Math.floor(Math.random() * 500) : '-'}</td></tr>
                <tr><td class="py-2 text-gray-600">Alleanza</td><td class="py-2 text-right text-blue-600 hover:underline cursor-pointer">${icon === '🏠' ? 'Alleanza_Alpha' : '-'}</td></tr>
            </table>
        `;
  detailsPanel.innerHTML = html;
}

function showTooltip(e, text) {
  tooltip.style.display = 'block';
  tooltip.innerText = text;

  const rect = gridEl.getBoundingClientRect();
  let left = e.pageX + 15;
  let top = e.pageY + 15;

  tooltip.style.left = left + 'px';
  tooltip.style.top = top + 'px';
}

function hideTooltip() {
  tooltip.style.display = 'none';
}

renderMap(currentX, currentY);
