// web-blockchain-client.js
// Client library for web users to interact with your blockchain

class EduNetWebClient {
    constructor(config = {}) {
        this.serverUrl = config.serverUrl || 'https://api.edunet.blockchain';
        this.wsUrl = config.wsUrl || 'wss://api.edunet.blockchain/ws';
        this.apiKey = config.apiKey;
        this.wallet = null;
        this.ws = null;
    }

    // ========================================================================
    // WALLET MANAGEMENT
    // ========================================================================

    async createWallet(username, password) {
        const response = await this.apiCall('/api/v1/wallets', {
            method: 'POST',
            body: {
                username,
                password_hash: this.hashPassword(password)
            }
        });

        this.wallet = {
            address: response.address,
            username: username,
            created_at: response.created_at
        };

        return this.wallet;
    }

    async loginWallet(username, password) {
        const response = await this.apiCall('/api/v1/auth/login', {
            method: 'POST', 
            body: {
                username,
                password_hash: this.hashPassword(password)
            }
        });

        this.apiKey = response.api_key;
        this.wallet = response.wallet_info;
        return this.wallet;
    }

    // ========================================================================
    // TRANSACTION OPERATIONS  
    // ========================================================================

    async getBalance(address = null) {
        const addr = address || this.wallet?.address;
        if (!addr) throw new Error('No wallet address available');

        const response = await this.apiCall(`/api/v1/wallets/${addr}/balance`);
        return {
            confirmed: response.confirmed_balance,
            pending: response.pending_balance,
            total: response.total_balance
        };
    }

    async sendTransaction(toAddress, amount, message = '') {
        if (!this.wallet) throw new Error('No wallet connected');

        const response = await this.apiCall('/api/v1/transactions/send', {
            method: 'POST',
            body: {
                from_address: this.wallet.address,
                to_address: toAddress,
                amount: amount,
                message: message,
                fee_rate: 'normal' // normal, fast, slow
            }
        });

        // Subscribe to transaction status updates
        this.subscribeToTransaction(response.txid);

        return {
            txid: response.txid,
            status: 'pending',
            estimated_confirmation: response.estimated_blocks
        };
    }

    async getTransactionHistory(limit = 50, offset = 0) {
        if (!this.wallet) throw new Error('No wallet connected');

        const response = await this.apiCall(
            `/api/v1/wallets/${this.wallet.address}/transactions?limit=${limit}&offset=${offset}`
        );

        return response.transactions.map(tx => ({
            txid: tx.txid,
            type: tx.type, // 'sent', 'received', 'mining'
            amount: tx.amount,
            address: tx.other_address,
            confirmations: tx.confirmations,
            timestamp: tx.timestamp,
            message: tx.message
        }));
    }

    // ========================================================================
    // NETWORK STATUS & STATS
    // ========================================================================

    async getNetworkStatus() {
        const response = await this.apiCall('/api/v1/network/status');
        return {
            blockHeight: response.block_height,
            difficulty: response.difficulty,
            hashRate: response.estimated_hash_rate,
            connectedPeers: response.connected_peers,
            mempoolSize: response.mempool_transactions,
            networkUptime: response.uptime_seconds
        };
    }

    async getMarketStats() {
        // This could integrate with your DeFi features
        const response = await this.apiCall('/api/v1/market/stats');
        return {
            totalSupply: response.total_supply,
            circulatingSupply: response.circulating_supply,
            activeAddresses: response.active_addresses_24h,
            transactionVolume: response.volume_24h
        };
    }

    // ========================================================================
    // REAL-TIME UPDATES
    // ========================================================================

    connectWebSocket() {
        this.ws = new WebSocket(this.wsUrl);
        
        this.ws.onopen = () => {
            console.log('🔗 Connected to EduNet network');
            
            // Subscribe to wallet updates if logged in
            if (this.wallet) {
                this.subscribe('wallet_updates', {
                    wallet_address: this.wallet.address
                });
            }

            // Subscribe to network updates
            this.subscribe('new_blocks');
            this.subscribe('network_stats');
        };

        this.ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            this.handleWebSocketMessage(data);
        };

        this.ws.onclose = () => {
            console.log('🔌 Disconnected from EduNet network');
            // Attempt to reconnect after 5 seconds
            setTimeout(() => this.connectWebSocket(), 5000);
        };
    }

    subscribe(eventType, params = {}) {
        if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not connected');
        }

        this.ws.send(JSON.stringify({
            type: 'subscribe',
            event: eventType,
            params: params
        }));
    }

    handleWebSocketMessage(data) {
        switch(data.type) {
            case 'new_block':
                this.onNewBlock?.(data.block);
                break;
            case 'new_transaction':
                if (data.transaction.involves_address === this.wallet?.address) {
                    this.onWalletTransaction?.(data.transaction);
                }
                break;
            case 'network_stats':
                this.onNetworkUpdate?.(data.stats);
                break;
            case 'transaction_confirmed':
                this.onTransactionConfirmed?.(data.transaction);
                break;
        }
    }

    // ========================================================================
    // UTILITY METHODS
    // ========================================================================

    async apiCall(endpoint, options = {}) {
        const url = this.serverUrl + endpoint;
        const config = {
            method: options.method || 'GET',
            headers: {
                'Content-Type': 'application/json',
                ...options.headers
            }
        };

        if (this.apiKey) {
            config.headers['Authorization'] = `Bearer ${this.apiKey}`;
        }

        if (options.body) {
            config.body = JSON.stringify(options.body);
        }

        const response = await fetch(url, config);
        
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.message || 'API request failed');
        }

        return await response.json();
    }

    hashPassword(password) {
        // Simple client-side hashing (use proper crypto in production)
        return btoa(password + 'edunet_salt').substr(0, 32);
    }

    subscribeToTransaction(txid) {
        this.subscribe('transaction_status', { txid });
    }

    // Event handlers (can be overridden)
    onNewBlock = null;
    onWalletTransaction = null; 
    onNetworkUpdate = null;
    onTransactionConfirmed = null;
}

// Export for use in web pages
if (typeof window !== 'undefined') {
    window.EduNetWebClient = EduNetWebClient;
}

// Export for Node.js usage
if (typeof module !== 'undefined') {
    module.exports = EduNetWebClient;
}