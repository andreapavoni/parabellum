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
    const detailsPanel = document.getElementById('details-panel');
    const tooltip = document.getElementById('tile-info');

    if (
      !gridEl ||
      !yAxisEl ||
      !xAxisEl ||
      !headerCoordsEl ||
      !inputX ||
      !inputY ||
      !detailsPanel ||
      !tooltip
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
          tile.onmouseenter = (event) => showTooltip(event, visual.title);
          tile.onmouseleave = hideTooltip;
          tile.onmouseover = () => showDetails(tileData, visual, wrappedX, wrappedY);

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
        return {
          icon: '🏠',
          typeClass: isHome ? 'is-own-village' : 'is-village',
          title: isHome ? `MyVillage (${x}|${y})` : `Village (${x}|${y})`,
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

      return {
        icon: '',
        typeClass: '',
        title: `Valley (${x}|${y})`,
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

    function showDetails(tile, visual, x, y) {
      const isVillage =
        tile && tile.village_id !== undefined && tile.village_id !== null;
      const html = `
      <div class="text-center mb-4">
        <div class="text-4xl mb-2">${visual.icon || '🌲'}</div>
        <div class="font-bold text-sm text-gray-800">${visual.title}</div>
        <div class="text-xs text-gray-500 mt-1">
          Coordinate: <span class="font-mono font-bold text-black">${x}|${y}</span>
        </div>
      </div>
      <table class="w-full text-xs">
        <tr class="border-b border-gray-200">
          <td class="py-2 text-gray-600">Giocatore</td>
          <td class="py-2 text-right font-bold text-black">
            ${isVillage && tile.player_id ? tile.player_id : '-'}
          </td>
        </tr>
        <tr class="border-b border-gray-200">
          <td class="py-2 text-gray-600">Villaggio</td>
          <td class="py-2 text-right text-black font-semibold">
            ${isVillage && tile.village_id ? tile.village_id : '-'}
          </td>
        </tr>
        <tr>
          <td class="py-2 text-gray-600">Tipo</td>
          <td class="py-2 text-right">
            ${tile?.tile_type ?? '-'}
          </td>
        </tr>
      </table>
    `;

      detailsPanel.innerHTML = html;
    }

    function showTooltip(event, text) {
      tooltip.style.display = 'block';
      tooltip.innerText = text;

      tooltip.style.left = `${event.pageX + 15}px`;
      tooltip.style.top = `${event.pageY + 15}px`;
    }

    function hideTooltip() {
      tooltip.style.display = 'none';
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
