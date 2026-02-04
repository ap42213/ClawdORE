// ==UserScript==
// @name         ClawdORE Block Selector
// @namespace    https://clawdore.vercel.app
// @version      1.0
// @description  Auto-select recommended blocks on ore.supply from ClawdORE bot swarm
// @author       ClawdORE
// @match        https://ore.supply/*
// @match        https://*.ore.supply/*
// @grant        GM_xmlhttpRequest
// @grant        GM_addStyle
// @connect      clawdore.vercel.app
// @connect      localhost
// ==/UserScript==

(function() {
    'use strict';

    // ClawdORE API URL - change this if you're self-hosting
    const CLAWDORE_API = 'https://clawdore.vercel.app';

    // Add styles for the floating UI
    GM_addStyle(`
        #clawdore-panel {
            position: fixed;
            bottom: 20px;
            right: 20px;
            z-index: 99999;
            font-family: 'JetBrains Mono', 'Fira Code', monospace;
        }
        #clawdore-btn {
            background: linear-gradient(135deg, #ffd700, #ff9500);
            color: #000;
            border: none;
            padding: 12px 20px;
            border-radius: 12px;
            font-size: 14px;
            font-weight: bold;
            cursor: pointer;
            box-shadow: 0 4px 20px rgba(255, 215, 0, 0.4);
            display: flex;
            align-items: center;
            gap: 8px;
            transition: all 0.2s;
        }
        #clawdore-btn:hover {
            transform: translateY(-2px);
            box-shadow: 0 6px 30px rgba(255, 215, 0, 0.6);
        }
        #clawdore-btn.loading {
            opacity: 0.7;
            pointer-events: none;
        }
        #clawdore-btn.success {
            background: linear-gradient(135deg, #00ff88, #00cc6a);
        }
        #clawdore-btn.error {
            background: linear-gradient(135deg, #ff4444, #cc0000);
            color: #fff;
        }
        #clawdore-expanded {
            display: none;
            background: rgba(20, 20, 30, 0.95);
            border: 1px solid #ffd700;
            border-radius: 12px;
            padding: 16px;
            margin-bottom: 10px;
            min-width: 280px;
            color: #fff;
        }
        #clawdore-expanded.show {
            display: block;
        }
        #clawdore-expanded h3 {
            margin: 0 0 12px 0;
            color: #ffd700;
            font-size: 14px;
        }
        #clawdore-blocks {
            font-size: 24px;
            color: #00ff88;
            font-weight: bold;
            text-align: center;
            padding: 12px;
            background: rgba(0, 255, 136, 0.1);
            border-radius: 8px;
            margin-bottom: 12px;
        }
        #clawdore-status {
            font-size: 12px;
            color: #888;
            text-align: center;
        }
        #clawdore-manual {
            display: flex;
            gap: 8px;
            margin-top: 12px;
        }
        #clawdore-manual input {
            flex: 1;
            padding: 8px 12px;
            background: #0a0a15;
            border: 1px solid #333;
            border-radius: 6px;
            color: #fff;
            font-family: inherit;
            font-size: 14px;
        }
        #clawdore-manual button {
            padding: 8px 16px;
            background: #333;
            border: none;
            border-radius: 6px;
            color: #fff;
            cursor: pointer;
            font-weight: bold;
        }
        #clawdore-manual button:hover {
            background: #444;
        }
        .clawdore-grid {
            display: grid;
            grid-template-columns: repeat(5, 1fr);
            gap: 4px;
            margin: 12px 0;
        }
        .clawdore-sq {
            aspect-ratio: 1;
            background: #1a1a2e;
            border: 2px solid #333;
            border-radius: 4px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 12px;
            font-weight: bold;
            color: #666;
            cursor: pointer;
        }
        .clawdore-sq:hover {
            border-color: #ffd700;
        }
        .clawdore-sq.selected {
            background: linear-gradient(135deg, #ffd700, #ff9500);
            border-color: #ffd700;
            color: #000;
        }
    `);

    // Create the UI
    const panel = document.createElement('div');
    panel.id = 'clawdore-panel';
    panel.innerHTML = `
        <div id="clawdore-expanded">
            <h3>🦀 ClawdORE Recommendations</h3>
            <div id="clawdore-blocks">--</div>
            <div id="clawdore-status">Click "Fetch" to get recommendations</div>
            <div class="clawdore-grid" id="clawdore-grid"></div>
            <div id="clawdore-manual">
                <input type="text" id="clawdore-input" placeholder="Or enter: 5, 12, 17...">
                <button id="clawdore-apply">Apply</button>
            </div>
        </div>
        <button id="clawdore-btn">🦀 ClawdORE</button>
    `;
    document.body.appendChild(panel);

    // Initialize grid
    const grid = document.getElementById('clawdore-grid');
    let selectedBlocks = [];
    
    for (let i = 1; i <= 25; i++) {
        const sq = document.createElement('div');
        sq.className = 'clawdore-sq';
        sq.textContent = i;
        sq.dataset.num = i;
        sq.onclick = () => toggleBlock(i);
        grid.appendChild(sq);
    }

    function toggleBlock(num) {
        if (selectedBlocks.includes(num)) {
            selectedBlocks = selectedBlocks.filter(n => n !== num);
        } else {
            selectedBlocks.push(num);
        }
        selectedBlocks.sort((a, b) => a - b);
        updateGridUI();
        document.getElementById('clawdore-input').value = selectedBlocks.join(', ');
    }

    function updateGridUI() {
        document.querySelectorAll('.clawdore-sq').forEach(sq => {
            const num = parseInt(sq.dataset.num);
            sq.classList.toggle('selected', selectedBlocks.includes(num));
        });
        document.getElementById('clawdore-blocks').textContent = 
            selectedBlocks.length > 0 ? selectedBlocks.join(', ') : '--';
    }

    // Main button - toggle expanded panel
    const btn = document.getElementById('clawdore-btn');
    const expanded = document.getElementById('clawdore-expanded');
    
    btn.onclick = () => {
        if (expanded.classList.contains('show')) {
            // If already open, fetch and apply
            fetchAndApply();
        } else {
            // Open panel
            expanded.classList.add('show');
            btn.textContent = '⚡ Fetch & Apply';
        }
    };

    // Apply button - select blocks on the page
    document.getElementById('clawdore-apply').onclick = () => {
        const input = document.getElementById('clawdore-input').value;
        const matches = input.match(/\d+/g);
        if (matches) {
            selectedBlocks = [...new Set(matches.map(n => parseInt(n)).filter(n => n >= 1 && n <= 25))];
            updateGridUI();
            selectBlocksOnPage(selectedBlocks);
        }
    };

    // Fetch recommendations from ClawdORE API
    function fetchAndApply() {
        const status = document.getElementById('clawdore-status');
        btn.classList.add('loading');
        btn.textContent = '⏳ Fetching...';
        status.textContent = 'Connecting to ClawdORE...';

        // Try /api/state first (consensus recommendation)
        GM_xmlhttpRequest({
            method: 'GET',
            url: `${CLAWDORE_API}/api/state`,
            onload: function(response) {
                try {
                    const data = JSON.parse(response.responseText);
                    if (data.consensus_recommendation?.squares?.length > 0) {
                        selectedBlocks = data.consensus_recommendation.squares;
                        updateGridUI();
                        document.getElementById('clawdore-input').value = selectedBlocks.join(', ');
                        selectBlocksOnPage(selectedBlocks);
                        btn.classList.remove('loading');
                        btn.classList.add('success');
                        btn.textContent = '✅ Applied!';
                        status.textContent = `Selected ${selectedBlocks.length} blocks from consensus`;
                        setTimeout(() => {
                            btn.classList.remove('success');
                            btn.textContent = '⚡ Fetch & Apply';
                        }, 3000);
                        return;
                    }
                    // Fallback to /api/ore
                    fetchFromOreApi();
                } catch (e) {
                    fetchFromOreApi();
                }
            },
            onerror: function() {
                fetchFromOreApi();
            }
        });
    }

    function fetchFromOreApi() {
        const status = document.getElementById('clawdore-status');
        
        GM_xmlhttpRequest({
            method: 'GET',
            url: `${CLAWDORE_API}/api/ore`,
            onload: function(response) {
                btn.classList.remove('loading');
                try {
                    const data = JSON.parse(response.responseText);
                    if (data.recommended_blocks?.length > 0) {
                        selectedBlocks = data.recommended_blocks;
                        updateGridUI();
                        document.getElementById('clawdore-input').value = selectedBlocks.join(', ');
                        selectBlocksOnPage(selectedBlocks);
                        btn.classList.add('success');
                        btn.textContent = '✅ Applied!';
                        status.textContent = `Selected ${selectedBlocks.length} blocks (lowest deployment)`;
                        setTimeout(() => {
                            btn.classList.remove('success');
                            btn.textContent = '⚡ Fetch & Apply';
                        }, 3000);
                    } else {
                        btn.classList.add('error');
                        btn.textContent = '❌ No Data';
                        status.textContent = 'No recommendations available - select manually';
                        setTimeout(() => {
                            btn.classList.remove('error');
                            btn.textContent = '⚡ Fetch & Apply';
                        }, 3000);
                    }
                } catch (e) {
                    btn.classList.add('error');
                    btn.textContent = '❌ Error';
                    status.textContent = 'Could not parse response';
                    setTimeout(() => {
                        btn.classList.remove('error');
                        btn.textContent = '⚡ Fetch & Apply';
                    }, 3000);
                }
            },
            onerror: function() {
                btn.classList.remove('loading');
                btn.classList.add('error');
                btn.textContent = '❌ Offline';
                status.textContent = 'Could not reach ClawdORE API';
                setTimeout(() => {
                    btn.classList.remove('error');
                    btn.textContent = '⚡ Fetch & Apply';
                }, 3000);
            }
        });
    }

    // Select blocks on the ore.supply page
    function selectBlocksOnPage(blocks) {
        let clicked = 0;
        const status = document.getElementById('clawdore-status');

        // Method 1: Data attributes (data-index is 0-based, data-square is 1-based)
        blocks.forEach(num => {
            document.querySelectorAll(`[data-index="${num-1}"], [data-square="${num}"], [data-cell="${num-1}"]`).forEach(el => {
                el.click();
                clicked++;
            });
        });

        // Method 2: Grid children by index
        if (clicked === 0) {
            document.querySelectorAll('[class*="grid"] > *, [class*="board"] > *, [class*="game"] > *').forEach((el, i) => {
                if (blocks.includes(i + 1)) {
                    el.click();
                    clicked++;
                }
            });
        }

        // Method 3: Buttons/divs with number text content
        if (clicked === 0) {
            document.querySelectorAll('button, [role="button"], div[class*="square"], div[class*="cell"], div[class*="block"]').forEach(el => {
                const text = el.textContent.trim();
                const num = parseInt(text);
                if (blocks.includes(num) && text === String(num)) {
                    el.click();
                    clicked++;
                }
            });
        }

        // Method 4: Look for any clickable element in a grid-like container
        if (clicked === 0) {
            const containers = document.querySelectorAll('[style*="grid"], [style*="flex"]');
            containers.forEach(container => {
                const children = container.children;
                if (children.length >= 25) {
                    blocks.forEach(num => {
                        if (children[num-1]) {
                            children[num-1].click();
                            clicked++;
                        }
                    });
                }
            });
        }

        if (clicked > 0) {
            status.textContent = `✅ Clicked ${clicked} squares on page`;
        } else {
            status.textContent = `⚠️ Could not find squares - page structure may differ`;
        }

        console.log(`[ClawdORE] Selected blocks: ${blocks.join(', ')} (clicked ${clicked} elements)`);
    }

    console.log('[ClawdORE] Block Selector loaded! Click the 🦀 button in the bottom-right corner.');
})();
