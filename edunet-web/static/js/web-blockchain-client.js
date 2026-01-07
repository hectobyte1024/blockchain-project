// Web Blockchain Client for EduNet
// Handles WebSocket communication and blockchain operations

class BlockchainClient {
    constructor(wsUrl) {
        this.wsUrl = wsUrl;
        this.socket = null;
        this.eventHandlers = {};
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
    }

    // Connect to WebSocket
    async connect() {
        return new Promise((resolve, reject) => {
            try {
                this.socket = new WebSocket(this.wsUrl);
                
                this.socket.onopen = () => {
                    console.log('✅ Connected to EduNet blockchain');
                    this.reconnectAttempts = 0;
                    resolve();
                };

                this.socket.onmessage = (event) => {
                    try {
                        const data = JSON.parse(event.data);
                        this.handleMessage(data);
                    } catch (error) {
                        console.error('Failed to parse WebSocket message:', error);
                    }
                };

                this.socket.onclose = () => {
                    console.log('🔌 Blockchain connection closed');
                    this.attemptReconnect();
                };

                this.socket.onerror = (error) => {
                    console.error('❌ WebSocket error:', error);
                    reject(error);
                };

            } catch (error) {
                reject(error);
            }
        });
    }

    // Handle incoming messages
    handleMessage(data) {
        if (this.eventHandlers[data.type]) {
            this.eventHandlers[data.type].forEach(handler => {
                try {
                    handler(data.data || data);
                } catch (error) {
                    console.error('Error in event handler:', error);
                }
            });
        }
    }

    // Register event handler
    on(event, handler) {
        if (!this.eventHandlers[event]) {
            this.eventHandlers[event] = [];
        }
        this.eventHandlers[event].push(handler);
    }

    // Send message to server
    async send(type, data) {
        return new Promise((resolve, reject) => {
            if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
                reject(new Error('WebSocket not connected'));
                return;
            }

            const messageId = Date.now().toString();
            const message = {
                id: messageId,
                type: type,
                ...data
            };

            // Set up one-time response handler
            const responseHandler = (responseData) => {
                if (responseData.id === messageId || responseData.type === type + '_response') {
                    resolve(responseData);
                }
            };

            this.on(type + '_response', responseHandler);
            this.on('error', (error) => {
                if (error.id === messageId) {
                    reject(new Error(error.message));
                }
            });

            this.socket.send(JSON.stringify(message));

            // Timeout after 10 seconds
            setTimeout(() => {
                reject(new Error('Request timeout'));
            }, 10000);
        });
    }

    // Attempt to reconnect
    attemptReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            console.log(`🔄 Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);
            
            setTimeout(() => {
                this.connect().catch(error => {
                    console.error('Reconnection failed:', error);
                });
            }, 2000 * this.reconnectAttempts); // Exponential backoff
        } else {
            console.error('❌ Max reconnection attempts reached');
        }
    }

    // Close connection
    disconnect() {
        if (this.socket) {
            this.socket.close();
        }
    }

    // Get network status
    async getNetworkStatus() {
        try {
            const response = await fetch('/api/v1/network/status');
            const result = await response.json();
            return result.success ? result.data : null;
        } catch (error) {
            console.error('Failed to get network status:', error);
            return null;
        }
    }

    // Get wallet balance
    async getWalletBalance(address) {
        try {
            const response = await fetch(`/api/v1/wallets/${address}/balance`);
            const result = await response.json();
            return result.success ? result.data : null;
        } catch (error) {
            console.error('Failed to get wallet balance:', error);
            return null;
        }
    }

    // Send transaction
    async sendTransaction(fromAddress, toAddress, amount, message = '') {
        try {
            const response = await fetch('/api/v1/transactions/send', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${localStorage.getItem('session_token')}`
                },
                body: JSON.stringify({
                    from_address: fromAddress,
                    to_address: toAddress,
                    amount: amount,
                    transaction_type: 'transfer',
                    metadata: { message }
                })
            });

            const result = await response.json();
            return result;
        } catch (error) {
            console.error('Failed to send transaction:', error);
            throw error;
        }
    }

    // Get transaction history
    async getTransactionHistory(address) {
        try {
            const response = await fetch(`/api/v1/transactions/history/${address}`);
            const result = await response.json();
            return result.success ? result.data : [];
        } catch (error) {
            console.error('Failed to get transaction history:', error);
            return [];
        }
    }

    // Start mining
    async startMining(minerAddress) {
        return await this.send('start_mining', { miner_address: minerAddress });
    }

    // Stop mining
    async stopMining() {
        return await this.send('stop_mining', {});
    }

    // Get mining statistics
    async getMiningStats() {
        try {
            const response = await fetch('/api/v1/mining/stats');
            const result = await response.json();
            return result.success ? result.data : null;
        } catch (error) {
            console.error('Failed to get mining stats:', error);
            return null;
        }
    }

    // Create new wallet
    async createWallet(name) {
        try {
            const response = await fetch('/api/v1/wallets', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${localStorage.getItem('session_token')}`
                },
                body: JSON.stringify({ name })
            });

            const result = await response.json();
            return result;
        } catch (error) {
            console.error('Failed to create wallet:', error);
            throw error;
        }
    }

    // Get all wallets
    async getWallets() {
        try {
            const response = await fetch('/api/v1/wallets', {
                headers: {
                    'Authorization': `Bearer ${localStorage.getItem('session_token')}`
                }
            });

            const result = await response.json();
            return result.success ? result.data : [];
        } catch (error) {
            console.error('Failed to get wallets:', error);
            return [];
        }
    }
}

// Export for module use
if (typeof module !== 'undefined' && module.exports) {
    module.exports = BlockchainClient;
}

// Global availability
window.BlockchainClient = BlockchainClient;