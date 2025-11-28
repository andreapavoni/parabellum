(function () {
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

    const gridEl = document.getElementById('map-grid');
    const yAxisEl = document.getElementById('y-axis-container');
    const xAxisEl = document.getElementById('x-axis-container');
    const headerCoordsEl = document.getElementById('header-coords');
    const inputX = document.getElementById('input-x');
    const inputY = document.getElementById('input-y');
    const mapContainer = document.querySelector('.map-container-main');
    const detailsPanelContainer = document.getElementById('details-panel-container');
    const detailsPanel = document.getElementById('details-panel');
    const DETAIL_DELAY_MS = 150;
    let detailsTimeout = null;

    if (
      !gridEl ||
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

    gridEl.addEventListener('mouseleave', () => {
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

      gridEl.innerHTML = '';
      yAxisEl.innerHTML = '';
      xAxisEl.innerHTML = '';
      hideDetails();

      for (let y = currentY + currentRadius; y >= currentY - currentRadius; y--) {
        const wrappedY = wrapCoordinate(y);
        const div = document.createElement('div');
        div.className = `y-label ${wrappedY === currentY ? 'highlight-axis' : ''
          }`;
        div.innerText = wrappedY;
        yAxisEl.appendChild(div);
      }

      for (let x = currentX - currentRadius; x <= currentX + currentRadius; x++) {
        const wrappedX = wrapCoordinate(x);
        const div = document.createElement('div');
        div.className = `x-label ${wrappedX === currentX ? 'highlight-axis' : ''
          }`;
        div.innerText = wrappedX;
        xAxisEl.appendChild(div);
      }

      for (let y = currentY + currentRadius; y >= currentY - currentRadius; y--) {
        const wrappedY = wrapCoordinate(y);
        for (let x = currentX - currentRadius; x <= currentX + currentRadius; x++) {
          const wrappedX = wrapCoordinate(x);
          const tile = document.createElement('div');
          const tileData = tileLookup.get(coordKey(wrappedX, wrappedY));
          const visual = describeTile(tileData, wrappedX, wrappedY);

          tile.className = `tile ${visual.typeClass}`;
          tile.innerHTML = `<span class="tile-content">${visual.icon}</span>`;
          tile.onmouseenter = (event) => {
            scheduleDetails(tileData, visual, wrappedX, wrappedY, tile);
          };
          tile.onmouseleave = () => {
            cancelDetailsTimeout();
          };
          tile.onclick = () => showDetails(tileData, visual, wrappedX, wrappedY, tile);

          gridEl.appendChild(tile);
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
        const ownerSuffix = tile.player_name ? ` – ${tile.player_name}` : '';
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
      // const isVillage =
      //   data.village_id !== undefined && data.village_id !== null;
      // const isOasis = data.tile_type === 'oasis';
      // const valleySummary = data.valley
      // ? `${data.valley.lumber}-${data.valley.clay}-${data.valley.iron}-${data.valley.crop}`
      // : null;
      const ownerLabel = data.player_name || data.player_id || '-';
      // const oasisLabel = data.oasis || null;
      // const typeLabel = isVillage
      //   ? 'Village'
      //   : isOasis
      //     ? oasisLabel || 'Oasis'
      //     : 'Valley';
      // const topologyLabel = isOasis
      //   ? oasisLabel || '-'
      //   : valleySummary || '-';

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
            ${ownerLabel}
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
})();
