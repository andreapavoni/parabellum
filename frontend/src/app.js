function initializeCountdownHelpers() {
  if (window.updateCountdowns && window.shouldReloadForCountdown) {
    return;
  }
  const pad = (num) => num.toString().padStart(2, '0');
  window.updateCountdowns = (selector) => {
    const nodes = Array.from(document.querySelectorAll(selector));
    if (!nodes.length) {
      return [];
    }
    return nodes.map((timer) => {
      let remaining = parseInt(timer.dataset.seconds || '0', 10);
      if (!Number.isFinite(remaining)) {
        remaining = 0;
      }
      if (remaining <= 0) {
        timer.dataset.seconds = 0;
        timer.textContent = '00:00:00';
        return 0;
      }
      remaining -= 1;
      timer.dataset.seconds = remaining;
      const hours = Math.floor(remaining / 3600);
      const minutes = Math.floor((remaining % 3600) / 60);
      const seconds = remaining % 60;
      timer.textContent = `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
      return remaining;
    });
  };
  window.shouldReloadForCountdown = (values) =>
    Array.isArray(values) && values.some((value) => value === 0);
}

function startCountdownTicker() {
  if (window.__countdownTickerStarted) {
    return;
  }
  window.__countdownTickerStarted = true;
  const selectors = [
    '.queue-timer[data-seconds]',
    '.countdown-timer[data-seconds]',
  ];
  let reloadScheduled = false;
  const scheduleReload = () => {
    if (reloadScheduled) {
      return;
    }
    reloadScheduled = true;
    setTimeout(() => window.location.reload(), 1500);
  };
  const tick = () => {
    if (!window.updateCountdowns || !window.shouldReloadForCountdown) {
      return;
    }
    if (reloadScheduled) {
      return;
    }
    let reload = false;
    selectors.forEach((selector) => {
      const values = window.updateCountdowns(selector);
      if (window.shouldReloadForCountdown(values)) {
        reload = true;
      }
    });
    if (reload) {
      scheduleReload();
    }
  };
  tick();
  setInterval(tick, 1000);
}

function startMapHandler() {
  const DEFAULT_RADIUS = 7;
  const mapRoot = document.getElementById('map-page');

  if (mapRoot) {
    let currentX = parseInt(mapRoot.dataset.centerX ?? '0', 10);
    let currentY = parseInt(mapRoot.dataset.centerY ?? '0', 10);
    const homeX = parseInt(mapRoot.dataset.homeX ?? `${currentX}`, 10);
    const homeY = parseInt(mapRoot.dataset.homeY ?? `${currentY}`, 10);

    const homeVillageId = parseInt(mapRoot.dataset.homeVillageId ?? '0', 10);
    const parsedWorldSize = parseInt(mapRoot.dataset.worldSize ?? '100', 10);
    const worldSize = Number.isFinite(parsedWorldSize) && parsedWorldSize > 0 ? parsedWorldSize : 100;
    let currentRadius = DEFAULT_RADIUS;
    let tileLookup = new Map();
    let loadingRegion = false;

    const tilesContainer = document.getElementById('map-tiles-container');
    const yAxisEl = document.getElementById('axis-y-container');
    const xAxisEl = document.getElementById('axis-x-container');
    const headerCoordsEl = document.getElementById('header-coords');
    const inputX = document.getElementById('input-x');
    const inputY = document.getElementById('input-y');
    const mapContainer = document.querySelector('.map-container-main');
    const detailsPanelContainer = document.getElementById('details-panel-container');
    const detailsPanel = document.getElementById('details-panel');
    const DETAIL_DELAY_MS = 150;
    let detailsTimeout = null;

    if (
      !tilesContainer ||
      !yAxisEl ||
      !xAxisEl ||
      !headerCoordsEl ||
      !inputX ||
      !inputY ||
      !mapContainer ||
      !detailsPanelContainer ||
      !detailsPanel
    ) {
      return;
    }

    const coordKey = (x, y) => `${x}:${y}`;

    const wrapCoordinate = (value) => {
      if (!Number.isFinite(worldSize) || worldSize <= 0) {
        return value;
      }
      const span = worldSize * 2 + 1;
      const normalized = ((value + worldSize) % span + span) % span;
      return normalized - worldSize;
    };

    tilesContainer.addEventListener('mouseleave', () => {
      hideDetails();
    });

    async function fetchRegion(params) {
      const search = new URLSearchParams();
      if (params.x !== undefined && params.y !== undefined) {
        search.set('x', params.x.toString());
        search.set('y', params.y.toString());
      }
      if (params.villageId !== undefined) {
        search.set('village_id', params.villageId.toString());
      }

      const response = await fetch(`/map/data?${search.toString()}`, {
        headers: { Accept: 'application/json' },
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || 'Unable to load map data');
      }

      return response.json();
    }

    async function updateRegion(params) {
      if (loadingRegion) {
        return;
      }
      loadingRegion = true;

      try {
        const data = await fetchRegion(params);
        currentX = data.center.x;
        currentY = data.center.y;
        currentRadius = data.radius ?? DEFAULT_RADIUS;
        tileLookup = new Map(
          data.tiles.map((tile) => [coordKey(tile.x, tile.y), tile]),
        );
        renderMap();
      } catch (error) {
        console.error('Failed to load map region', error);
      } finally {
        loadingRegion = false;
      }
    }

    function renderMap() {
      headerCoordsEl.innerText = `(${currentX}|${currentY})`;
      inputX.value = currentX;
      inputY.value = currentY;

      // Clear SVG tiles container
      tilesContainer.innerHTML = '';
      yAxisEl.innerHTML = '';
      xAxisEl.innerHTML = '';
      hideDetails();

      // Render Y axis labels (vertical, left side)
      for (let y = currentY + currentRadius; y >= currentY - currentRadius; y--) {
        const wrappedY = wrapCoordinate(y);
        const div = document.createElement('div');
        div.className = `y-label ${wrappedY === currentY ? 'highlight-axis' : ''}`;
        div.innerText = wrappedY;
        yAxisEl.appendChild(div);
      }

      // Render X axis labels (horizontal, bottom)
      for (let x = currentX - currentRadius; x <= currentX + currentRadius; x++) {
        const wrappedX = wrapCoordinate(x);
        const div = document.createElement('div');
        div.className = `x-label ${wrappedX === currentX ? 'highlight-axis' : ''}`;
        div.innerText = wrappedX;
        xAxisEl.appendChild(div);
      }

      // Render SVG tiles (15x15 grid)
      const gridSize = currentRadius * 2 + 1; // 15
      const cellSize = 100; // SVG units per cell (matches viewBox 1500/15)

      for (let row = 0; row < gridSize; row++) {
        const y = currentY + currentRadius - row;
        const wrappedY = wrapCoordinate(y);

        for (let col = 0; col < gridSize; col++) {
          const x = currentX - currentRadius + col;
          const wrappedX = wrapCoordinate(x);

          const tileData = tileLookup.get(coordKey(wrappedX, wrappedY));
          const visual = describeTile(tileData, wrappedX, wrappedY);

          // Calculate position in SVG coordinate space
          const tx = col * cellSize;
          const ty = row * cellSize;

          // Create SVG group element
          const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
          g.classList.add('map-tile');
          g.setAttribute('transform', `translate(${tx}, ${ty})`);
          g.setAttribute('data-x', wrappedX);
          g.setAttribute('data-y', wrappedY);

          // Hover background
          const hoverBg = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
          hoverBg.classList.add('hover-bg');
          hoverBg.setAttribute('width', '100');
          hoverBg.setAttribute('height', '100');
          hoverBg.setAttribute('fill', 'transparent');
          g.appendChild(hoverBg);

          // Background color for oases
          if (visual.typeClass.includes('oasis')) {
            const oasisBg = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
            oasisBg.setAttribute('width', '100');
            oasisBg.setAttribute('height', '100');

            // Set background color based on oasis type
            let bgColor = '#F1F8E9'; // default
            if (visual.typeClass.includes('lumber')) {
              bgColor = '#c5e1a5';
            } else if (visual.typeClass.includes('clay')) {
              bgColor = '#ffe0b2';
            } else if (visual.typeClass.includes('iron')) {
              bgColor = '#e0e0e0';
            } else if (visual.typeClass.includes('crop')) {
              bgColor = '#fff9c4';
            }
            oasisBg.setAttribute('fill', bgColor);
            g.appendChild(oasisBg);
          }

          // Border for villages
          if (visual.typeClass.includes('village')) {
            const border = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
            border.setAttribute('x', '5');
            border.setAttribute('y', '5');
            border.setAttribute('width', '90');
            border.setAttribute('height', '90');
            border.setAttribute('fill', 'none');
            border.setAttribute('stroke', visual.typeClass.includes('own') ? 'orange' : 'green');
            border.setAttribute('stroke-width', '10');
            g.appendChild(border);
          }

          // Icon/emoji text
          if (visual.icon) {
            const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            text.setAttribute('x', '50');
            text.setAttribute('y', '50');
            text.setAttribute('text-anchor', 'middle');
            text.setAttribute('dominant-baseline', 'central');
            text.setAttribute('font-size', '60');
            text.setAttribute('pointer-events', 'none');
            text.textContent = visual.icon;
            g.appendChild(text);
          }

          // Event handlers
          g.addEventListener('mouseenter', () => {
            scheduleDetails(tileData, visual, wrappedX, wrappedY, g);
          });
          g.addEventListener('mouseleave', () => {
            cancelDetailsTimeout();
          });
          g.addEventListener('click', () => {
            showDetails(tileData, visual, wrappedX, wrappedY, g);
          });

          tilesContainer.appendChild(g);
        }
      }
    }

    function describeTile(tile, x, y) {
      if (!tile) {
        return {
          icon: '',
          typeClass: '',
          title: `Unknown (${x}|${y})`,
        };
      }


      const isVillageTile =
        tile.village_id !== undefined && tile.village_id !== null;

      if (isVillageTile) {
        const matchesHomeId =
          Number.isFinite(homeVillageId) &&
          tile.village_id !== undefined &&
          tile.village_id !== null &&
          tile.village_id === homeVillageId;
        const matchesHomeCoords = x === homeX && y === homeY;
        const isHome = matchesHomeId || matchesHomeCoords;
        const villageLabel = tile.village_name ?? 'Village';
        return {
          icon: '🏠',
          typeClass: isHome ? 'is-own-village' : 'is-village',
          title: `${villageLabel}`,
        };
      }


      if (tile.tile_type === 'oasis') {
        const variant = (tile.oasis || 'oasis').toLowerCase();
        return {
          icon: selectOasisIcon(variant),
          typeClass: `oasis-${variant.replace(/[^a-z0-9]/g, '-')}`,
          title: `${tile.oasis ?? 'Oasis'} (${x}|${y})`,
        };
      }

      const valleySummary = tile.valley
        ? `${tile.valley.lumber}-${tile.valley.clay}-${tile.valley.iron}-${tile.valley.crop}`
        : '';

      return {
        icon: '',
        typeClass: '',
        title: valleySummary
          ? `Valley ${valleySummary}`
          : `Valley (${x}|${y})`,
      };
    }

    function selectOasisIcon(variant) {
      if (variant.includes('lumber')) {
        return '🌲';
      }
      if (variant.includes('clay')) {
        return '🧱';
      }
      if (variant.includes('iron')) {
        return '⛰️';
      }
      return '🌾';
    }

    function scheduleDetails(tile, visual, x, y, tileElement) {
      cancelDetailsTimeout();
      detailsTimeout = setTimeout(() => {
        showDetails(tile, visual, x, y, tileElement);
      }, DETAIL_DELAY_MS);
    }

    function cancelDetailsTimeout() {
      if (detailsTimeout) {
        clearTimeout(detailsTimeout);
        detailsTimeout = null;
      }
    }

    function showDetails(tile, visual, x, y, tileElement) {
      const data = tile || {};
      const ownerLabel = data.player_name || '-';
      const population = data.village_population || '-';
      const tribe = data.tribe || '-';

      const html = `
      <div class="text-center mb-4">
        <div class="text-4xl mb-2">${visual.icon || '🌲'}</div>
        <div class="font-bold text-sm text-gray-800">${visual.title}</div>
        <div class="text-xs text-gray-500 mt-1">
          <span class="font-mono font-bold text-black">${x}|${y}</span>
        </div>
      </div>
      <table class="w-full text-xs">
        <tr class="border-b border-gray-200">
          <td class="py-2 text-gray-600">Player</td>
          <td class="py-2 text-right font-bold text-black">
            ${ownerLabel}
          </td>
        </tr>
        <tr class="border-b border-gray-200">
          <td class="py-2 text-gray-600">Population</td>
          <td class="py-2 text-right font-bold text-black">
            ${population}
          </td>
        </tr>
        <tr class="border-b border-gray-200">
          <td class="py-2 text-gray-600">Tribe</td>
          <td class="py-2 text-right font-bold text-black">
            ${tribe}
          </td>
        </tr>
      </table>
    `;

      detailsPanel.innerHTML = html;
      detailsPanelContainer.classList.remove('hidden');
      detailsPanelContainer.style.pointerEvents = 'none';
      positionDetailsPanel(tileElement);
    }

    function hideDetails() {
      cancelDetailsTimeout();
      if (!detailsPanelContainer.classList.contains('hidden')) {
        detailsPanelContainer.classList.add('hidden');
        detailsPanel.innerHTML = '';
        detailsPanelContainer.style.left = '';
        detailsPanelContainer.style.top = '';
      }
    }

    function positionDetailsPanel(tileElement) {
      const containerRect = mapContainer.getBoundingClientRect();
      const panelRect = detailsPanelContainer.getBoundingClientRect();
      const tileRect = tileElement.getBoundingClientRect();

      const offset = 12;
      const maxLeft = containerRect.width - panelRect.width - offset;
      const maxTop = containerRect.height - panelRect.height - offset;

      let left =
        tileRect.left - containerRect.left + tileRect.width + offset;
      let top = tileRect.top - containerRect.top;

      if (left > maxLeft) {
        left = tileRect.left - containerRect.left - panelRect.width - offset;
      }

      left = Math.max(offset, Math.min(left, maxLeft));
      top = Math.max(offset, Math.min(top, maxTop));

      detailsPanelContainer.style.left = `${left}px`;
      detailsPanelContainer.style.top = `${top}px`;
    }

    window.moveMap = (dx, dy) => {
      const nextX = wrapCoordinate(currentX + dx);
      const nextY = wrapCoordinate(currentY + dy);
      updateRegion({ x: nextX, y: nextY });
    };

    window.goToCoords = () => {
      const parsedX = parseInt(inputX.value, 10);
      const parsedY = parseInt(inputY.value, 10);
      if (Number.isFinite(parsedX) && Number.isFinite(parsedY)) {
        const targetX = wrapCoordinate(parsedX);
        const targetY = wrapCoordinate(parsedY);
        updateRegion({ x: targetX, y: targetY });
      }
    };

    updateRegion({ x: currentX, y: currentY });
  }
}

function startServerClock() {
  if (window.__serverClockStarted) {
    return;
  }
  window.__serverClockStarted = true;

  const pad = (num) => num.toString().padStart(2, '0');
  const element = document.getElementById('server-time');
  if (!element) {
    return;
  }
  let timestamp = parseInt(element.dataset.timestamp || '0', 10);
  if (!Number.isFinite(timestamp) || timestamp <= 0) {
    return;
  }
  const tick = () => {
    timestamp += 1;
    const date = new Date(timestamp * 1000);
    const hours = pad(date.getUTCHours());
    const minutes = pad(date.getUTCMinutes());
    const seconds = pad(date.getUTCSeconds());
    element.textContent = `${hours}:${minutes}:${seconds}`;
    element.dataset.timestamp = timestamp;
  };
  setInterval(tick, 1000);
}

function startResourceTicker() {
  if (window.__resourceTickerStarted) {
    return;
  }
  window.__resourceTickerStarted = true;

  const parseNumber = (value) => {
    const parsed = parseFloat(value);
    return Number.isFinite(parsed) ? parsed : 0;
  };
  const resources = Array.from(document.querySelectorAll('.res-value[data-prod-per-hour]')).map((el) => {
    const amount = parseNumber(el.dataset.amount);
    const capacity = parseNumber(el.dataset.capacity);
    const prodPerHour = parseNumber(el.dataset.prodPerHour);
    return {
      el,
      amount,
      capacity,
      capacityDisplay: Math.floor(capacity),
      perSecond: prodPerHour / 3600,
    };
  });
  if (!resources.length) {
    return;
  }
  const render = (resource) => {
    const amountInt = Math.max(0, Math.floor(resource.amount));
    resource.el.textContent = `${amountInt}/${resource.capacityDisplay}`;
  };
  let lastTick = Date.now();
  const tick = () => {
    const now = Date.now();
    const deltaSeconds = Math.max(0, (now - lastTick) / 1000);
    lastTick = now;
    resources.forEach((resource) => {
      if (resource.perSecond === 0) {
        return;
      }
      resource.amount += resource.perSecond * deltaSeconds;
      resource.amount = Math.min(resource.capacity, Math.max(0, resource.amount));
      render(resource);
    });
  };
  resources.forEach(render);
  setInterval(tick, 1000);
}

(function () {
  initializeCountdownHelpers();
  startCountdownTicker();
  startServerClock();
  startResourceTicker();
  startMapHandler();
})();
