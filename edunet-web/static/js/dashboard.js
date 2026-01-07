// Dashboard JavaScript for Edunet Platform

class Dashboard {
    constructor() {
        this.apiBase = '/api';
        this.init();
    }

    async init() {
        await this.loadDashboardStats();
        await this.loadRecentActivity();
        this.initPortfolioChart();
        this.initEventListeners();
        this.startPeriodicUpdates();
        
        // Load blockchain data after shared app is ready
        this.waitForSharedApp();
    }
    
    // Wait for shared app to be available and load blockchain data
    waitForSharedApp() {
        const checkApp = () => {
            if (window.edunetApp && window.edunetApp.getCurrentWallet()) {
                console.log('Shared app available, loading blockchain data...');
                this.loadBlockchainData();
            } else {
                setTimeout(checkApp, 500);
            }
        };
        checkApp();
    }

    // Load dashboard statistics from real blockchain data
    async loadDashboardStats() {
        try {
            // Get blockchain info for real stats
            const chainResponse = await fetch(`${this.apiBase}/blockchain/chain-info`);
            const chainResult = await chainResponse.json();
            
            let stats = {
                total_students: 1, // Current single-user system
                active_listings: 0, // No marketplace items yet
                total_loans: 0, // No loan system implemented yet
                minted_nfts: 0, // No NFT system implemented yet
                blockchain_height: 0,
                total_supply: 0
            };
            
            if (chainResult.success) {
                stats.blockchain_height = chainResult.data.height || 0;
                stats.total_supply = chainResult.data.total_supply || 900.50;
            }
            
            this.updateStatCards(stats);
        } catch (error) {
            console.error('Error loading dashboard stats:', error);
            // Fallback to empty stats
            this.updateStatCards({
                total_students: 1,
                active_listings: 0,
                total_loans: 0,
                minted_nfts: 0
            });
        }
    }

    // Load real blockchain data
    async loadBlockchainData() {
        try {
            // Use wallet from global app instead of fetching again
            if (window.edunetApp && window.edunetApp.getCurrentWallet()) {
                this.currentWallet = window.edunetApp.getCurrentWallet();
                const studentAddress = this.currentWallet.address;
                console.log('Dashboard using global wallet address:', studentAddress);
                
                // The shared app already loads balance, just update display
                const balance = this.currentWallet.blockchain_balance || this.currentWallet.balance || 0;
                this.updateWalletBalance({ balance });
                
                // Load transaction history
                const txResponse = await fetch(`${this.apiBase}/blockchain/history/${studentAddress}`);
                const txResult = await txResponse.json();
                
                if (txResult.success) {
                    this.updateTransactionHistory(txResult.data);
                }
                
                // Load chain info
                const chainResponse = await fetch(`${this.apiBase}/blockchain/chain-info`);
                const chainResult = await chainResponse.json();
                
                if (chainResult.success) {
                    this.updateChainInfo(chainResult.data);
                }
            } else {
                console.warn('Global edunetApp not available, skipping blockchain data load');
                // Don't load independently, wait for shared app
            }
        } catch (error) {
            console.error('Error loading blockchain data:', error);
        }
    }

    // Update wallet balance display
    updateWalletBalance(balanceData) {
        const balanceElement = document.getElementById('wallet-balance');
        if (balanceElement) {
            balanceElement.textContent = `${balanceData.balance.toFixed(2)} EDU`;
        }
        
        // Update balance info in sidebar if exists
        const walletInfo = document.querySelector('.wallet-info .balance');
        if (walletInfo) {
            walletInfo.textContent = `${balanceData.balance.toFixed(2)} EDU`;
        }
        
        console.log('Wallet balance updated:', balanceData);
    }

    // Update transaction history
    updateTransactionHistory(transactions) {
        // Convert blockchain transactions to activity format
        const activities = transactions.map(tx => ({
            type: tx.type === 'received' ? 'marketplace' : 'payment',
            user: tx.type === 'received' ? 'Marketplace Sale' : 'Payment Sent',
            action: tx.type === 'received' ? 'received payment' : 'sent payment',
            time: this.formatTime(tx.timestamp),
            amount: `${tx.amount > 0 ? '+' : ''}${tx.amount} EDU`,
            hash: tx.hash
        }));
        
        this.renderRecentActivity(activities);
    }

    // Update chain info display
    updateChainInfo(chainInfo) {
        // Add blockchain stats to dashboard
        const statsGrid = document.querySelector('.stats-grid');
        if (statsGrid && !document.getElementById('blockchain-stats')) {
            const blockchainStats = document.createElement('div');
            blockchainStats.id = 'blockchain-stats';
            blockchainStats.className = 'stat-card';
            blockchainStats.innerHTML = `
                <div class="stat-icon blockchain">
                    <i class="fas fa-link"></i>
                </div>
                <div class="stat-info">
                    <h3>${chainInfo.height}</h3>
                    <p>Block Height</p>
                    <span class="stat-change positive">Network Active</span>
                </div>
            `;
            statsGrid.appendChild(blockchainStats);
        }
        
        console.log('Chain info updated:', chainInfo);
    }

    // Update stat cards with data
    updateStatCards(stats) {
        document.getElementById('total-students').textContent = stats.total_students;
        document.getElementById('active-listings').textContent = stats.active_listings;
        document.getElementById('total-loans').textContent = stats.total_loans;
        document.getElementById('minted-nfts').textContent = stats.minted_nfts;
    }

    // Load recent activity from real blockchain data or show empty state
    async loadRecentActivity() {
        try {
            // Try to load real transaction history if available
            if (this.currentWallet && this.currentWallet.address) {
                const response = await fetch(`${this.apiBase}/blockchain/history/${this.currentWallet.address}`);
                const result = await response.json();
                
                if (result.success && result.data.length > 0) {
                    // Convert blockchain transactions to activity format
                    const activities = result.data.slice(0, 4).map(tx => ({
                        type: tx.type === 'received' ? 'blockchain' : 'payment',
                        user: tx.type === 'received' ? 'Blockchain Network' : 'Transaction',
                        action: tx.type === 'received' ? 'received funds' : 'sent payment',
                        time: this.formatTime(tx.timestamp),
                        amount: `${tx.amount > 0 ? '+' : ''}${tx.amount} EDU`
                    }));
                    
                    this.renderRecentActivity(activities);
                    return;
                }
            }
            
            // Show empty state for new blockchain
            const emptyActivity = [
                {
                    type: 'blockchain',
                    user: 'System',
                    action: 'genesis block created',
                    time: 'Recently',
                    amount: '900.50 EDU'
                },
                {
                    type: 'info',
                    user: 'Welcome',
                    action: 'start using the platform to see activity',
                    time: 'Now',
                    amount: '--'
                }
            ];

            this.renderRecentActivity(emptyActivity);
        } catch (error) {
            console.error('Error loading recent activity:', error);
            this.renderRecentActivity([]);
        }
    }

    // Render recent activity items
    renderRecentActivity(activities) {
        const activityList = document.getElementById('recent-activity');
        if (!activityList) return;

        activityList.innerHTML = activities.map(activity => `
            <div class="activity-item fade-in">
                <div class="activity-icon ${activity.type}">
                    <i class="fas ${this.getActivityIcon(activity.type)}"></i>
                </div>
                <div class="activity-content">
                    <p><strong>${activity.user}</strong> ${activity.action}</p>
                    <span class="activity-time">${activity.time}</span>
                </div>
                <div class="activity-amount">${activity.amount}</div>
            </div>
        `).join('');
    }

    // Get icon for activity type
    getActivityIcon(type) {
        const icons = {
            'marketplace': 'fa-shopping-cart',
            'loans': 'fa-hand-holding-usd',
            'nfts': 'fa-palette',
            'investment': 'fa-rocket'
        };
        return icons[type] || 'fa-circle';
    }

    // Initialize portfolio chart with real wallet balance
    initPortfolioChart() {
        const ctx = document.getElementById('portfolio-chart');
        if (!ctx) return;

        // Real portfolio data - flat line showing current balance since it's a new blockchain
        const currentBalance = this.currentWallet ? (this.currentWallet.blockchain_balance || this.currentWallet.balance || 900.50) : 900.50;
        
        const portfolioData = {
            labels: ['Genesis', 'Block 1', 'Block 2', 'Block 3', 'Block 4', 'Current'],
            datasets: [{
                label: 'Wallet Balance (EDU)',
                data: [900.50, currentBalance, currentBalance, currentBalance, currentBalance, currentBalance],
                borderColor: '#2563eb',
                backgroundColor: 'rgba(37, 99, 235, 0.1)',
                fill: true,
                tension: 0.2
            }]
        };

        this.portfolioChart = new Chart(ctx, {
            type: 'line',
            data: portfolioData,
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        display: false
                    }
                },
                scales: {
                    y: {
                        beginAtZero: true,
                        grid: {
                            color: '#f3f4f6'
                        },
                        ticks: {
                            callback: function(value) {
                                return value.toFixed(2) + ' EDU';
                            }
                        }
                    },
                    x: {
                        grid: {
                            display: false
                        }
                    }
                },
                elements: {
                    point: {
                        radius: 4,
                        hoverRadius: 6
                    }
                },
                interaction: {
                    intersect: false,
                    mode: 'index'
                }
            }
        });
    }

    // Initialize event listeners
    initEventListeners() {
        // Tab switching for performers panel
        const tabBtns = document.querySelectorAll('.tab-btn');
        tabBtns.forEach(btn => {
            btn.addEventListener('click', () => {
                this.switchTab(btn.dataset.tab);
            });
        });

        // Time filter for portfolio
        const timeFilter = document.querySelector('.time-filter');
        if (timeFilter) {
            timeFilter.addEventListener('change', () => {
                this.updatePortfolioChart(timeFilter.value);
            });
        }

        // Refresh button (if exists)
        const refreshBtn = document.querySelector('.refresh-btn');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', () => {
                this.refreshDashboard();
            });
        }
    }

    // Switch tab in performers panel
    switchTab(tab) {
        // Update active tab button
        document.querySelectorAll('.tab-btn').forEach(btn => {
            btn.classList.remove('active');
        });
        document.querySelector(`[data-tab="${tab}"]`).classList.add('active');

        // Update performers list based on tab
        this.loadPerformers(tab);
    }

    // Load performers based on type - show real blockchain data or development note
    async loadPerformers(type) {
        const performersList = document.getElementById('performers-list');
        if (!performersList) return;

        // Show current user and development status for single-user blockchain
        const currentUser = this.currentWallet ? 'Current User' : 'Anonymous';
        const currentBalance = this.currentWallet ? this.currentWallet.blockchain_balance || 0 : 0;
        
        let data = [];
        
        if (type === 'students') {
            data = [
                { 
                    rank: 1, 
                    name: currentUser, 
                    detail: 'Blockchain University • Student', 
                    score: currentBalance.toFixed(1)
                },
                {
                    rank: '--',
                    name: 'Multi-user features',
                    detail: 'Coming soon • Development in progress',
                    score: '--'
                }
            ];
        } else if (type === 'items') {
            data = [
                {
                    rank: '--',
                    name: 'Marketplace items',
                    detail: 'No items listed yet • Be the first to list',
                    score: '--'
                },
                {
                    rank: '--',
                    name: 'Educational content',
                    detail: 'Upload your study materials',
                    score: '--'
                }
            ];
        } else if (type === 'projects') {
            data = [
                {
                    rank: '--',
                    name: 'Research projects',
                    detail: 'No active projects • Submit a proposal',
                    score: '--'
                },
                {
                    rank: '--',
                    name: 'Funding opportunities',
                    detail: 'Community funding coming soon',
                    score: '--'
                }
            ];
        }
        
        performersList.innerHTML = data.map(item => `
            <div class="performer-item fade-in">
                <div class="performer-rank">${item.rank}</div>
                <div class="performer-avatar">
                    <i class="fas ${item.rank === '--' ? 'fa-hourglass-half' : 'fa-user'}"></i>
                </div>
                <div class="performer-info">
                    <div class="performer-name">${item.name}</div>
                    <div class="performer-detail">${item.detail}</div>
                </div>
                <div class="performer-score">${item.score}</div>
            </div>
        `).join('');
    }

    // Update portfolio chart with different time period
    updatePortfolioChart(period) {
        // In a real application, this would fetch new data based on the period
        console.log(`Updating portfolio chart for period: ${period}`);
    }

    // Refresh entire dashboard
    async refreshDashboard() {
        const refreshBtn = document.querySelector('.refresh-btn');
        if (refreshBtn) {
            refreshBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Refreshing...';
            refreshBtn.disabled = true;
        }

        try {
            await Promise.all([
                this.loadDashboardStats(),
                this.loadRecentActivity()
            ]);
        } finally {
            if (refreshBtn) {
                refreshBtn.innerHTML = '<i class="fas fa-refresh"></i> Refresh';
                refreshBtn.disabled = false;
            }
        }
    }

    // Start periodic updates
    startPeriodicUpdates() {
        // Update stats every 5 minutes
        setInterval(() => {
            this.loadDashboardStats();
        }, 5 * 60 * 1000);

        // Update activity every 2 minutes
        setInterval(() => {
            this.loadRecentActivity();
        }, 2 * 60 * 1000);
    }

    // Utility function to format numbers
    formatNumber(num) {
        if (num >= 1000000) {
            return (num / 1000000).toFixed(1) + 'M';
        } else if (num >= 1000) {
            return (num / 1000).toFixed(1) + 'K';
        }
        return num.toString();
    }

    // Utility function to format currency
    formatCurrency(amount, currency = 'EDU') {
        return `${amount.toLocaleString()} ${currency}`;
    }

    // Utility function to format relative time
    formatTime(timestamp) {
        if (!timestamp) return 'Unknown time';
        
        const date = new Date(timestamp);
        const now = new Date();
        const diffMs = now - date;
        const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
        const diffDays = Math.floor(diffHours / 24);
        
        if (diffDays > 0) {
            return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
        } else if (diffHours > 0) {
            return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
        } else {
            return 'Less than 1 hour ago';
        }
    }

    // Create blockchain transaction
    async createTransaction(fromAddress, toAddress, amount, transactionType, metadata = null) {
        try {
            const transactionRequest = {
                from_address: fromAddress,
                to_address: toAddress,
                amount: amount,
                transaction_type: transactionType,
                metadata: metadata
            };

            const response = await fetch(`${this.apiBase}/blockchain/transaction`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(transactionRequest)
            });

            const result = await response.json();
            
            if (result.success) {
                console.log('Transaction created successfully:', result.data);
                // Refresh blockchain data
                await this.loadBlockchainData();
                return result.data;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error creating transaction:', error);
            throw error;
        }
    }

    // Purchase marketplace item with real blockchain transaction
    async purchaseMarketplaceItem(itemId, price, sellerAddress) {
        if (!this.currentWallet) {
            throw new Error('No wallet loaded');
        }
        
        const studentAddress = this.currentWallet.address;
        const metadata = {
            item_id: itemId,
            marketplace_purchase: true
        };

        try {
            const txHash = await this.createTransaction(
                studentAddress,
                sellerAddress,
                price,
                'marketplace',
                metadata
            );
            
            // Show success message
            this.showTransactionSuccess(`Purchase successful! Transaction: ${txHash}`);
            return txHash;
        } catch (error) {
            this.showTransactionError(`Purchase failed: ${error.message}`);
            throw error;
        }
    }

    // Show transaction success message
    showTransactionSuccess(message) {
        const notification = document.createElement('div');
        notification.className = 'transaction-notification success';
        notification.innerHTML = `<i class="fas fa-check-circle"></i> ${message}`;
        document.body.appendChild(notification);
        
        setTimeout(() => {
            notification.remove();
        }, 5000);
    }

    // Show transaction error message
    showTransactionError(message) {
        const notification = document.createElement('div');
        notification.className = 'transaction-notification error';
        notification.innerHTML = `<i class="fas fa-exclamation-circle"></i> ${message}`;
        document.body.appendChild(notification);
        
        setTimeout(() => {
            notification.remove();
        }, 5000);
    }

    // Show notification
    showNotification(message, type = 'info') {
        const notification = document.createElement('div');
        notification.className = `notification notification-${type}`;
        notification.textContent = message;
        
        document.body.appendChild(notification);
        
        setTimeout(() => {
            notification.classList.add('show');
        }, 100);
        
        setTimeout(() => {
            notification.classList.remove('show');
            setTimeout(() => {
                document.body.removeChild(notification);
            }, 300);
        }, 3000);
    }
}

// NOTE: WalletManager class removed - functionality moved to shared.js EdunetApp
// All wallet management now happens through window.edunetApp

// Initialize dashboard when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    // Wait for shared app to be available, then init dashboard
    const initDashboard = () => {
        if (window.edunetApp) {
            window.dashboard = new Dashboard();
            // NOTE: Removed WalletManager - using shared edunetApp instead
        } else {
            // Retry after a short delay
            setTimeout(initDashboard, 100);
        }
    };
    
    initDashboard();
});

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { Dashboard };
}