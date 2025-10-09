// Live reload functionality for mdBook development
(function() {
    'use strict';
    
    // Only enable live reload in development (when served locally)
    if (window.location.hostname !== 'localhost' && window.location.hostname !== '127.0.0.1') {
        return;
    }
    
    // Create WebSocket connection for live reload
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/__livereload`;
    
    let ws;
    let reconnectInterval = 1000; // Start with 1 second
    const maxReconnectInterval = 30000; // Max 30 seconds
    
    function connect() {
        ws = new WebSocket(wsUrl);
        
        ws.onopen = function() {
            console.log('[LiveReload] Connected');
            reconnectInterval = 1000; // Reset reconnect interval on successful connection
        };
        
        ws.onmessage = function(event) {
            if (event.data === 'reload') {
                console.log('[LiveReload] Reloading page...');
                window.location.reload();
            }
        };
        
        ws.onclose = function() {
            console.log('[LiveReload] Connection closed, attempting to reconnect...');
            setTimeout(connect, reconnectInterval);
            reconnectInterval = Math.min(reconnectInterval * 1.5, maxReconnectInterval);
        };
        
        ws.onerror = function(error) {
            console.log('[LiveReload] WebSocket error:', error);
        };
    }
    
    // Start connection
    connect();
    
    // Clean up on page unload
    window.addEventListener('beforeunload', function() {
        if (ws) {
            ws.close();
        }
    });
})();
