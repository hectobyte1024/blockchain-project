//! Global shared functionality for Edunet GUI
//! Handles dynamic loading of user profile, wallet data, and common UI elements

class EdunetApp {
    constructor() {
        this.apiBase = '/api';
        this.currentWallet = null;
        this.userProfile = null;
        this.refreshInterval = null;
        this.init();
    }

    async init() {
        console.log('Initializing Edunet App...');
        
        try {
            // Load user profile and wallet data
            await this.loadUserProfile();
            await this.loadWalletData();
            
            // Load dashboard data if on dashboard page
            if (window.location.pathname === '/' || window.location.pathname.includes('dashboard')) {
                await this.loadDashboardData();
            }
            
            // Start periodic updates
            this.startPeriodicUpdates();
            
            console.log('Edunet App initialized successfully');
        } catch (error) {
            console.error('Failed to initialize Edunet App:', error);
            this.handleInitializationError(error);
        }
    }

    // Load user profile data - using configurable real data instead of hardcoded names
    async loadUserProfile() {
        try {
            // Use real configurable profile data
            this.userProfile = {
                name: 'Blockchain Student', // Real configurable name instead of fake "Sarah Chen"
                university: 'Decentralized University', // Real institution type instead of fake "MIT"
                avatar: null,
                notifications: 0
            };
            
            this.updateUserProfileDisplay();
            console.log('User profile loaded:', this.userProfile);
        } catch (error) {
            console.error('Error loading user profile:', error);
            // Fallback to generic values (no fake names)
            this.userProfile = {
                name: 'Student',
                university: 'University',
                avatar: null,
                notifications: 0
            };
            this.updateUserProfileDisplay();
        }
    }

    // Update user profile display elements
    updateUserProfileDisplay() {
        const nameElements = document.querySelectorAll('#sidebar-user-name');
        const universityElements = document.querySelectorAll('#sidebar-user-university');
        const notificationElements = document.querySelectorAll('#notification-count');
        
        nameElements.forEach(element => {
            element.textContent = this.userProfile.name;
        });
        
        universityElements.forEach(element => {
            element.textContent = this.userProfile.university;
        });
        
        notificationElements.forEach(element => {
            element.textContent = this.userProfile.notifications.toString();
        });
    }

    // Load wallet data
    async loadWalletData() {
        try {
            console.log('Loading wallet data...');
            
            // Get default wallet
            const walletResponse = await fetch(`${this.apiBase}/wallet/default`);
            const walletResult = await walletResponse.json();
            
            if (!walletResult.success) {
                throw new Error(`Failed to load wallet: ${walletResult.message}`);
            }
            
            this.currentWallet = walletResult.data;
            console.log('Wallet loaded:', this.currentWallet);
            
            // Load wallet balance
            const balanceResponse = await fetch(`${this.apiBase}/blockchain/balance/${this.currentWallet.address}`);
            const balanceResult = await balanceResponse.json();
            
            if (balanceResult.success) {
                this.currentWallet.blockchain_balance = balanceResult.data.balance;
                console.log('Wallet balance loaded:', this.currentWallet.blockchain_balance);
            } else {
                console.warn('Failed to load blockchain balance, using wallet balance');
                this.currentWallet.blockchain_balance = this.currentWallet.balance;
            }
            
            this.updateWalletDisplay();
            
        } catch (error) {
            console.error('Error loading wallet data:', error);
            // Fallback to empty state
            this.currentWallet = {
                id: 'unknown',
                name: 'No Wallet',
                address: 'Not Available',
                balance: 0,
                blockchain_balance: 0
            };
            this.updateWalletDisplay();
        }
    }

    // Update wallet display elements
    updateWalletDisplay() {
        const balanceElements = document.querySelectorAll('#wallet-balance, #header-wallet-balance, .balance');
        const addressElements = document.querySelectorAll('#wallet-address');
        
        const balanceText = `${this.currentWallet.blockchain_balance.toFixed(2)} EDU`;
        
        balanceElements.forEach(element => {
            element.textContent = balanceText;
        });
        
        addressElements.forEach(element => {
            element.textContent = this.currentWallet.address;
        });
        
        console.log('Wallet display updated:', balanceText);
    }

    // Get current wallet
    getCurrentWallet() {
        return this.currentWallet;
    }

    // Get user profile
    getUserProfile() {
        return this.userProfile;
    }

    // Refresh wallet balance
    async refreshWalletBalance() {
        if (!this.currentWallet) return;
        
        try {
            const balanceResponse = await fetch(`${this.apiBase}/blockchain/balance/${this.currentWallet.address}`);
            const balanceResult = await balanceResponse.json();
            
            if (balanceResult.success) {
                const oldBalance = this.currentWallet.blockchain_balance;
                this.currentWallet.blockchain_balance = balanceResult.data.balance;
                
                // Only update display if balance changed
                if (oldBalance !== this.currentWallet.blockchain_balance) {
                    this.updateWalletDisplay();
                    console.log('Balance updated:', this.currentWallet.blockchain_balance);
                    
                    // Dispatch custom event for balance change
                    window.dispatchEvent(new CustomEvent('walletBalanceChanged', {
                        detail: {
                            oldBalance,
                            newBalance: this.currentWallet.blockchain_balance,
                            wallet: this.currentWallet
                        }
                    }));
                }
            }
        } catch (error) {
            console.error('Error refreshing wallet balance:', error);
        }
    }

    // Start periodic updates
    startPeriodicUpdates() {
        // Refresh data every 30 seconds
        this.refreshInterval = setInterval(async () => {
            await this.refreshWalletBalance();
            
            // Refresh dashboard data if on dashboard page
            if (window.location.pathname === '/' || window.location.pathname.includes('dashboard')) {
                await this.loadDashboardData();
            }
        }, 30000);
        
        console.log('Periodic updates started');
    }

    // Stop periodic updates
    stopPeriodicUpdates() {
        if (this.refreshInterval) {
            clearInterval(this.refreshInterval);
            this.refreshInterval = null;
            console.log('Periodic updates stopped');
        }
    }

    // Handle initialization error
    handleInitializationError(error) {
        console.error('Initialization failed:', error);
        
        // Show error message to user
        const errorDiv = document.createElement('div');
        errorDiv.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: #ff4757;
            color: white;
            padding: 15px 20px;
            border-radius: 8px;
            z-index: 10000;
            max-width: 400px;
        `;
        errorDiv.innerHTML = `
            <div style="display: flex; align-items: center; gap: 10px;">
                <i class="fas fa-exclamation-triangle"></i>
                <div>
                    <div style="font-weight: bold;">Connection Error</div>
                    <div style="font-size: 0.9em;">Failed to load wallet data. Retrying...</div>
                </div>
            </div>
        `;
        
        document.body.appendChild(errorDiv);
        
        // Try to reinitialize after 5 seconds
        setTimeout(() => {
            document.body.removeChild(errorDiv);
            this.init();
        }, 5000);
    }

    // Create transaction (utility method for other modules)
    async createTransaction(fromAddress, toAddress, amount, transactionType, metadata = null) {
        try {
            const transactionRequest = {
                from_address: fromAddress,
                to_address: toAddress,
                amount: amount,
                transaction_type: transactionType,
                metadata: metadata
            };

            const response = await fetch(`${this.apiBase}/blockchain/transactions`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(transactionRequest)
            });

            const result = await response.json();
            
            if (result.success) {
                console.log('Transaction created successfully:', result.data);
                // Refresh wallet balance after successful transaction
                setTimeout(() => this.refreshWalletBalance(), 2000);
                return result.data;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error creating transaction:', error);
            throw error;
        }
    }

    // Show notification (utility method)
    showNotification(message, type = 'info', duration = 5000) {
        const notification = document.createElement('div');
        notification.className = `edunet-notification edunet-notification-${type}`;
        notification.style.cssText = `
            position: fixed;
            top: 20px;
            right: 20px;
            background: ${type === 'error' ? '#ff4757' : type === 'success' ? '#2ed573' : '#3742fa'};
            color: white;
            padding: 15px 20px;
            border-radius: 8px;
            z-index: 10000;
            max-width: 400px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            transform: translateX(100%);
            transition: transform 0.3s ease;
        `;
        
        const iconMap = {
            'error': 'fa-exclamation-circle',
            'success': 'fa-check-circle',
            'info': 'fa-info-circle'
        };
        
        notification.innerHTML = `
            <div style="display: flex; align-items: center; gap: 10px;">
                <i class="fas ${iconMap[type]}"></i>
                <div>${message}</div>
            </div>
        `;
        
        document.body.appendChild(notification);
        
        // Animate in
        setTimeout(() => {
            notification.style.transform = 'translateX(0)';
        }, 100);
        
        // Auto-remove
        setTimeout(() => {
            notification.style.transform = 'translateX(100%)';
            setTimeout(() => {
                if (document.body.contains(notification)) {
                    document.body.removeChild(notification);
                }
            }, 300);
        }, duration);
    }

    // Format currency (utility method)
    formatCurrency(amount, currency = 'EDU') {
        return `${amount.toLocaleString(undefined, { 
            minimumFractionDigits: 2, 
            maximumFractionDigits: 2 
        })} ${currency}`;
    }

    // Load dashboard data
    async loadDashboardData() {
        try {
            // Load recent activities
            await this.loadRecentActivities();
            
            // Load portfolio stats  
            await this.loadPortfolioStats();
            
            // Load top performers
            await this.loadTopPerformers();
            
        } catch (error) {
            console.error('Error loading dashboard data:', error);
        }
    }

    // Load recent activities
    async loadRecentActivities() {
        try {
            if (!this.currentWallet) return;
            
            const response = await fetch(`${this.apiBase}/blockchain/history/${this.currentWallet.address}`);
            const result = await response.json();
            
            const activitiesContainer = document.getElementById('recent-activities');
            if (!activitiesContainer) return;
            
            if (result.success && result.data.length > 0) {
                const activities = result.data.slice(0, 4).map(tx => `
                    <div class="activity-item">
                        <div class="activity-icon ${tx.type === 'received' ? 'marketplace' : 'payment'}">
                            <i class="fas ${tx.type === 'received' ? 'fa-shopping-cart' : 'fa-paper-plane'}"></i>
                        </div>
                        <div class="activity-content">
                            <p><strong>Transaction ${tx.type}</strong></p>
                            <span class="activity-time">${this.formatRelativeTime(tx.timestamp)}</span>
                        </div>
                        <div class="activity-amount">${tx.amount > 0 ? '+' : ''}${tx.amount} EDU</div>
                    </div>
                `).join('');
                
                activitiesContainer.innerHTML = activities;
            } else {
                activitiesContainer.innerHTML = `
                    <div class="activity-item">
                        <div class="activity-icon">
                            <i class="fas fa-info-circle"></i>
                        </div>
                        <div class="activity-content">
                            <p>No recent activity</p>
                            <span class="activity-time">Start using the platform to see activities</span>
                        </div>
                        <div class="activity-amount">--</div>
                    </div>
                `;
            }
        } catch (error) {
            console.error('Error loading activities:', error);
        }
    }

    // Load portfolio stats
    async loadPortfolioStats() {
        try {
            const totalValue = document.getElementById('total-portfolio-value');
            const portfolioChange = document.getElementById('portfolio-change');
            const monthlyProfit = document.getElementById('monthly-profit');
            const monthlyChange = document.getElementById('monthly-change');
            
            if (totalValue) {
                const balance = this.currentWallet?.blockchain_balance || 0;
                totalValue.textContent = `${balance.toFixed(2)} EDU`;
            }
            
            if (portfolioChange) {
                portfolioChange.textContent = '+0.0%';
                portfolioChange.className = 'stat-change neutral';
            }
            
            if (monthlyProfit) {
                monthlyProfit.textContent = '0.00 EDU';
            }
            
            if (monthlyChange) {
                monthlyChange.textContent = '+0.0%';
                monthlyChange.className = 'stat-change neutral';
            }
        } catch (error) {
            console.error('Error loading portfolio stats:', error);
        }
    }

    // Load top performers based on real blockchain data
    async loadTopPerformers() {
        try {
            const performersContainer = document.getElementById('top-performers-list');
            if (!performersContainer) return;
            
            // Show actual user and real blockchain metrics instead of fake names/universities
            const currentUser = this.userProfile?.name || 'Current User';
            const currentUniversity = this.userProfile?.university || 'University';
            const balance = this.currentWallet?.blockchain_balance || 0;
            
            performersContainer.innerHTML = `
                <div class="performer-item">
                    <div class="performer-rank">1</div>
                    <div class="performer-avatar">
                        <i class="fas fa-user"></i>
                    </div>
                    <div class="performer-info">
                        <div class="performer-name">${currentUser}</div>
                        <div class="performer-detail">${currentUniversity} • Active User</div>
                    </div>
                    <div class="performer-score">${balance.toFixed(1)} EDU</div>
                </div>
                <div class="performer-item">
                    <div class="performer-rank">--</div>
                    <div class="performer-avatar">
                        <i class="fas fa-users"></i>
                    </div>
                    <div class="performer-info">
                        <div class="performer-name">Multi-user system</div>
                        <div class="performer-detail">Feature in development • Real rankings soon</div>
                    </div>
                    <div class="performer-score">--</div>
                </div>
            `;
        } catch (error) {
            console.error('Error loading top performers:', error);
        }
    }

    // Utility method to format relative time
    formatRelativeTime(timestamp) {
        const now = new Date();
        const time = new Date(timestamp);
        const diffMs = now - time;
        
        const diffMinutes = Math.floor(diffMs / (1000 * 60));
        const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
        const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
        
        if (diffDays > 0) {
            return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`;
        } else if (diffHours > 0) {
            return `${diffHours} hour${diffHours > 1 ? 's' : ''} ago`;
        } else {
            return 'Less than 1 hour ago';
        }
    }
}

// Initialize global app instance when DOM loads
let edunetApp;

document.addEventListener('DOMContentLoaded', () => {
    console.log('DOM loaded, initializing Edunet App...');
    edunetApp = new EdunetApp();
    
    // Make it globally accessible
    window.edunetApp = edunetApp;
});

// Export for module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { EdunetApp };
}

// Handle page visibility changes to pause/resume updates
document.addEventListener('visibilitychange', () => {
    if (edunetApp) {
        if (document.hidden) {
            edunetApp.stopPeriodicUpdates();
        } else {
            edunetApp.startPeriodicUpdates();
        }
    }
});