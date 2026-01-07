// NFT Module for EduNet Blockchain
// Handles NFT minting, transfers, and viewing

class NFTManager {
    constructor(edunetApp) {
        this.edunetApp = edunetApp;
        this.apiBase = '/api/nft';
        this.nfts = [];
        this.ownedNFTs = [];
        this.init();
    }

    async init() {
        console.log('Initializing NFT Manager...');
        await this.loadAllNFTs();
        await this.loadOwnedNFTs();
        this.setupEventListeners();
    }

    // Load all NFTs from the blockchain
    async loadAllNFTs(limit = 100, offset = 0) {
        try {
            const response = await fetch(`${this.apiBase}/list?limit=${limit}&offset=${offset}`);
            const result = await response.json();
            
            if (result.success) {
                this.nfts = result.data;
                console.log(`Loaded ${this.nfts.length} NFTs`);
                return this.nfts;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error loading NFTs:', error);
            this.edunetApp.showNotification('Failed to load NFTs: ' + error.message, 'error');
            return [];
        }
    }

    // Load NFTs owned by current user
    async loadOwnedNFTs() {
        try {
            const wallet = this.edunetApp.getCurrentWallet();
            if (!wallet) {
                console.warn('No wallet available');
                return [];
            }

            const response = await fetch(`${this.apiBase}/owned/${wallet.address}`);
            const result = await response.json();
            
            if (result.success) {
                this.ownedNFTs = result.data;
                console.log(`Loaded ${this.ownedNFTs.length} owned NFTs`);
                return this.ownedNFTs;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error loading owned NFTs:', error);
            return [];
        }
    }

    // Mint a new NFT
    async mintNFT(name, description, imageUrl, metadata = {}) {
        try {
            const wallet = this.edunetApp.getCurrentWallet();
            if (!wallet) {
                throw new Error('No wallet available');
            }

            const mintRequest = {
                name: name,
                description: description,
                image_url: imageUrl,
                metadata: JSON.stringify(metadata)
            };

            const response = await fetch(`${this.apiBase}/mint`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(mintRequest)
            });

            const result = await response.json();
            
            if (result.success) {
                console.log('NFT minted successfully:', result.data);
                this.edunetApp.showNotification('NFT minted successfully!', 'success');
                
                // Refresh NFT lists
                await this.loadAllNFTs();
                await this.loadOwnedNFTs();
                
                // Refresh wallet balance
                await this.edunetApp.refreshWalletBalance();
                
                return result.data;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error minting NFT:', error);
            this.edunetApp.showNotification('Failed to mint NFT: ' + error.message, 'error');
            throw error;
        }
    }

    // Transfer NFT to another address
    async transferNFT(nftId, toAddress) {
        try {
            const wallet = this.edunetApp.getCurrentWallet();
            if (!wallet) {
                throw new Error('No wallet available');
            }

            const transferRequest = {
                nft_id: nftId,
                to_address: toAddress
            };

            const response = await fetch(`${this.apiBase}/transfer`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(transferRequest)
            });

            const result = await response.json();
            
            if (result.success) {
                console.log('NFT transferred successfully:', result.data);
                this.edunetApp.showNotification('NFT transferred successfully!', 'success');
                
                // Refresh NFT lists
                await this.loadAllNFTs();
                await this.loadOwnedNFTs();
                
                return result.data;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error transferring NFT:', error);
            this.edunetApp.showNotification('Failed to transfer NFT: ' + error.message, 'error');
            throw error;
        }
    }

    // Render NFT card
    renderNFTCard(nft) {
        const isOwned = this.ownedNFTs.some(owned => owned.nft_id === nft.nft_id);
        
        return `
            <div class="nft-card" data-nft-id="${nft.nft_id}">
                <div class="nft-image">
                    <img src="${nft.image_url || '/static/images/nft-placeholder.png'}" 
                         alt="${nft.name}"
                         onerror="this.src='/static/images/nft-placeholder.png'">
                    ${isOwned ? '<div class="nft-owned-badge"><i class="fas fa-check-circle"></i> Owned</div>' : ''}
                </div>
                <div class="nft-content">
                    <div class="nft-header">
                        <h3 class="nft-title">${this.escapeHtml(nft.name)}</h3>
                        <span class="nft-id">#${nft.nft_id.substring(0, 8)}...</span>
                    </div>
                    <p class="nft-description">${this.escapeHtml(nft.description)}</p>
                    <div class="nft-meta">
                        <div class="nft-creator">
                            <i class="fas fa-user"></i>
                            <span>${this.formatAddress(nft.creator)}</span>
                        </div>
                        <div class="nft-owner">
                            <i class="fas fa-wallet"></i>
                            <span>${this.formatAddress(nft.current_owner)}</span>
                        </div>
                    </div>
                    <div class="nft-actions">
                        ${isOwned ? `
                            <button class="btn-primary btn-sm" onclick="nftManager.showTransferModal('${nft.nft_id}')">
                                <i class="fas fa-paper-plane"></i> Transfer
                            </button>
                        ` : ''}
                        <button class="btn-secondary btn-sm" onclick="nftManager.showNFTDetails('${nft.nft_id}')">
                            <i class="fas fa-info-circle"></i> Details
                        </button>
                    </div>
                </div>
            </div>
        `;
    }

    // Render all NFTs to container
    renderNFTGallery(containerId = 'nft-gallery', filterOwned = false) {
        const container = document.getElementById(containerId);
        if (!container) {
            console.warn(`Container ${containerId} not found`);
            return;
        }

        const nftsToRender = filterOwned ? this.ownedNFTs : this.nfts;

        if (nftsToRender.length === 0) {
            container.innerHTML = `
                <div class="empty-state">
                    <i class="fas fa-palette fa-3x"></i>
                    <h3>${filterOwned ? 'No NFTs Owned' : 'No NFTs Available'}</h3>
                    <p>${filterOwned ? 'Mint your first NFT to get started!' : 'Be the first to mint an NFT on EduNet!'}</p>
                    <button class="btn-primary" onclick="nftManager.showMintModal()">
                        <i class="fas fa-plus"></i> Mint NFT
                    </button>
                </div>
            `;
            return;
        }

        const nftCards = nftsToRender.map(nft => this.renderNFTCard(nft)).join('');
        container.innerHTML = `<div class="nft-grid">${nftCards}</div>`;
    }

    // Show mint modal
    showMintModal() {
        const modal = document.getElementById('mint-nft-modal');
        if (modal) {
            modal.classList.add('show');
        }
    }

    // Close mint modal
    closeMintModal() {
        const modal = document.getElementById('mint-nft-modal');
        if (modal) {
            modal.classList.remove('show');
            // Reset form
            const form = document.getElementById('mint-nft-form');
            if (form) form.reset();
        }
    }

    // Show transfer modal
    showTransferModal(nftId) {
        // Create modal if it doesn't exist
        let modal = document.getElementById('transfer-nft-modal');
        if (!modal) {
            modal = this.createTransferModal();
            document.body.appendChild(modal);
        }

        // Set NFT ID
        modal.dataset.nftId = nftId;
        modal.classList.add('show');
    }

    // Create transfer modal
    createTransferModal() {
        const modal = document.createElement('div');
        modal.id = 'transfer-nft-modal';
        modal.className = 'modal';
        modal.innerHTML = `
            <div class="modal-content">
                <div class="modal-header">
                    <h2>Transfer NFT</h2>
                    <button class="modal-close" onclick="nftManager.closeTransferModal()">
                        <i class="fas fa-times"></i>
                    </button>
                </div>
                <form id="transfer-nft-form" class="modal-form">
                    <div class="form-group">
                        <label for="transfer-to-address">Recipient Address *</label>
                        <input type="text" id="transfer-to-address" required 
                               placeholder="edu1q...">
                    </div>
                    <div class="form-actions">
                        <button type="button" class="btn-secondary" onclick="nftManager.closeTransferModal()">Cancel</button>
                        <button type="submit" class="btn-primary">Transfer NFT</button>
                    </div>
                </form>
            </div>
        `;

        // Add submit handler
        modal.querySelector('#transfer-nft-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            const nftId = modal.dataset.nftId;
            const toAddress = document.getElementById('transfer-to-address').value;
            
            await this.transferNFT(nftId, toAddress);
            this.closeTransferModal();
            this.renderNFTGallery(); // Refresh gallery
        });

        return modal;
    }

    // Close transfer modal
    closeTransferModal() {
        const modal = document.getElementById('transfer-nft-modal');
        if (modal) {
            modal.classList.remove('show');
        }
    }

    // Show NFT details
    showNFTDetails(nftId) {
        const nft = this.nfts.find(n => n.nft_id === nftId) || 
                    this.ownedNFTs.find(n => n.nft_id === nftId);
        
        if (!nft) {
            console.error('NFT not found:', nftId);
            return;
        }

        // Parse metadata if it's a string
        let metadata = {};
        try {
            metadata = typeof nft.metadata === 'string' ? JSON.parse(nft.metadata) : nft.metadata;
        } catch (e) {
            console.warn('Failed to parse NFT metadata:', e);
        }

        this.edunetApp.showNotification(
            `NFT: ${nft.name}<br>Creator: ${this.formatAddress(nft.creator)}<br>Owner: ${this.formatAddress(nft.current_owner)}`,
            'info',
            8000
        );
    }

    // Setup event listeners
    setupEventListeners() {
        // Mint form submission
        const mintForm = document.getElementById('mint-nft-form');
        if (mintForm) {
            mintForm.addEventListener('submit', async (e) => {
                e.preventDefault();
                
                const name = document.getElementById('nft-title').value;
                const description = document.getElementById('nft-description').value;
                const imageUrl = document.getElementById('nft-image')?.value || '';
                
                // Get metadata
                const metadata = {};
                const category = document.getElementById('nft-category')?.value;
                if (category) metadata.category = category;
                
                const metadataField = document.getElementById('nft-metadata')?.value;
                if (metadataField) {
                    try {
                        Object.assign(metadata, JSON.parse(metadataField));
                    } catch (e) {
                        console.warn('Failed to parse metadata JSON:', e);
                    }
                }

                await this.mintNFT(name, description, imageUrl, metadata);
                this.closeMintModal();
                this.renderNFTGallery(); // Refresh gallery
            });
        }

        // Close modals on background click
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('modal')) {
                e.target.classList.remove('show');
            }
        });
    }

    // Utility: Format address
    formatAddress(address) {
        if (!address || address.length < 10) return address;
        return `${address.substring(0, 10)}...${address.substring(address.length - 6)}`;
    }

    // Utility: Escape HTML
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

// Initialize NFT manager when DOM loads
let nftManager;

document.addEventListener('DOMContentLoaded', () => {
    if (window.edunetApp) {
        nftManager = new NFTManager(window.edunetApp);
        window.nftManager = nftManager;
        console.log('NFT Manager initialized');
    } else {
        console.error('EdunetApp not found - NFT Manager cannot initialize');
    }
});
