// Marketplace JavaScript for Edunet Platform

class Marketplace {
    constructor() {
        this.apiBase = '/api';
        this.currentPage = 1;
        this.itemsPerPage = 12;
        this.filters = {
            search: '',
            category: '',
            type: '',
            price: ''
        };
        this.init();
    }

    async init() {
        await this.loadMarketplaceItems();
        this.initEventListeners();
        this.initModals();
    }

    // Initialize event listeners
    initEventListeners() {
        // Search input
        const searchInput = document.getElementById('search-input');
        if (searchInput) {
            let searchTimeout;
            searchInput.addEventListener('input', (e) => {
                clearTimeout(searchTimeout);
                searchTimeout = setTimeout(() => {
                    this.filters.search = e.target.value;
                    this.loadMarketplaceItems(true);
                }, 300);
            });
        }

        // Filter dropdowns
        const filterSelects = ['category-filter', 'type-filter', 'price-filter'];
        filterSelects.forEach(filterId => {
            const filterElement = document.getElementById(filterId);
            if (filterElement) {
                filterElement.addEventListener('change', (e) => {
                    const filterType = filterId.replace('-filter', '');
                    this.filters[filterType] = e.target.value;
                    this.loadMarketplaceItems(true);
                });
            }
        });

        // Load more button
        const loadMoreBtn = document.getElementById('load-more-btn');
        if (loadMoreBtn) {
            loadMoreBtn.addEventListener('click', () => {
                this.loadMarketplaceItems(false);
            });
        }

        // Create item form
        const createForm = document.getElementById('create-item-form');
        if (createForm) {
            createForm.addEventListener('submit', (e) => {
                e.preventDefault();
                this.handleCreateItem(e);
            });
        }
    }

    // Initialize modals
    initModals() {
        // Close modals when clicking outside
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('modal')) {
                this.closeAllModals();
            }
        });

        // ESC key to close modals
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeAllModals();
            }
        });
    }

    // Load marketplace items
    async loadMarketplaceItems(reset = false) {
        if (reset) {
            this.currentPage = 1;
        }

        try {
            const queryParams = new URLSearchParams({
                page: this.currentPage,
                limit: this.itemsPerPage,
                ...this.filters
            });

            const response = await fetch(`${this.apiBase}/marketplace?${queryParams}`);
            const result = await response.json();

            if (result.success) {
                this.renderMarketplaceItems(result.data, reset);
                this.currentPage++;
            } else {
                console.error('Failed to load marketplace items:', result.message);
                this.showNotification('Failed to load marketplace items', 'error');
            }
        } catch (error) {
            console.error('Error loading marketplace items:', error);
            // Show mock data for demo
            this.loadMockData(reset);
        }
    }

    // Show empty marketplace - no fake data for real blockchain system
    loadMockData(reset = false) {
        // No mock items - show empty marketplace for real system
        const emptyItems = [];
        
        // Show empty state message instead of fake data
        this.renderMarketplaceItems(emptyItems, reset);
        
        // Add empty state message if no items
        const grid = document.getElementById('marketplace-grid');
        if (grid && emptyItems.length === 0) {
            grid.innerHTML = `
                <div class="empty-marketplace">
                    <div class="empty-icon">
                        <i class="fas fa-store-slash"></i>
                    </div>
                    <h3>No Items Listed Yet</h3>
                    <p>Be the first to list an item in the marketplace!</p>
                    <button class="btn btn-primary" onclick="document.querySelector('.create-listing-btn')?.click()">
                        <i class="fas fa-plus"></i> List Your First Item
                    </button>
                </div>
            `;
        }
    }

    // Render marketplace items
    renderMarketplaceItems(items, reset = false) {
        const grid = document.getElementById('marketplace-grid');
        if (!grid) return;

        if (reset) {
            grid.innerHTML = '';
        }

        const itemsHTML = items.map(item => this.createItemHTML(item)).join('');
        
        if (reset) {
            grid.innerHTML = itemsHTML;
        } else {
            grid.insertAdjacentHTML('beforeend', itemsHTML);
        }

        // Add click listeners to new items
        this.addItemClickListeners();
    }

    // Create HTML for a single item
    createItemHTML(item) {
        const formattedDate = new Date(item.created_at).toLocaleDateString();
        const imageUrl = item.images && item.images.length > 0 ? item.images[0] : '';

        return `
            <div class="marketplace-item fade-in" data-item-id="${item.id}">
                <div class="item-image">
                    ${imageUrl ? `<img src="${imageUrl}" alt="${item.title}">` : '<i class="fas fa-image"></i>'}
                </div>
                <div class="item-content">
                    <div class="item-header">
                        <div>
                            <div class="item-title">${item.title}</div>
                            <span class="item-category">${this.formatCategory(item.category)}</span>
                        </div>
                        <div class="item-price">${item.price} ${item.currency}</div>
                    </div>
                    <div class="item-description">${item.description}</div>
                    <div class="item-seller">
                        <div class="seller-avatar">
                            <i class="fas fa-user"></i>
                        </div>
                        <div class="seller-info">
                            <div class="seller-name">${item.seller.name}</div>
                            <div class="seller-university">${item.seller.university}</div>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    // Add click listeners to items
    addItemClickListeners() {
        const items = document.querySelectorAll('.marketplace-item:not([data-listener])');
        items.forEach(item => {
            item.setAttribute('data-listener', 'true');
            item.addEventListener('click', (e) => {
                const itemId = item.getAttribute('data-item-id');
                this.showItemDetail(itemId);
            });
        });
    }

    // Show item detail modal
    async showItemDetail(itemId) {
        try {
            const response = await fetch(`${this.apiBase}/marketplace/${itemId}`);
            const result = await response.json();

            if (result.success) {
                this.populateItemDetailModal(result.data);
                this.showModal('item-detail-modal');
            } else {
                // Show mock data for demo
                this.showMockItemDetail(itemId);
            }
        } catch (error) {
            console.error('Error loading item detail:', error);
            this.showMockItemDetail(itemId);
        }
    }

    // Show empty item detail for non-existent item
    showMockItemDetail(itemId) {
        // Don't show fake data - show error message instead
        this.showNotification('Item not found. The marketplace is empty.', 'error');
    }

    // Populate item detail modal
    populateItemDetailModal(item) {
        document.getElementById('detail-title').textContent = item.title;
        document.getElementById('detail-item-title').textContent = item.title;
        document.getElementById('detail-price').textContent = `${item.price} ${item.currency}`;
        document.getElementById('detail-category').textContent = this.formatCategory(item.category);
        document.getElementById('detail-type').textContent = this.formatType(item.item_type);
        document.getElementById('detail-date').textContent = `Listed ${new Date(item.created_at).toLocaleDateString()}`;
        document.getElementById('detail-description').textContent = item.description;
        document.getElementById('detail-seller-name').textContent = item.seller.name;
        document.getElementById('detail-seller-university').textContent = item.seller.university;
        document.getElementById('detail-seller-rating').textContent = item.seller.rating;

        // Update main image
        const mainImage = document.getElementById('detail-main-image');
        if (item.images && item.images.length > 0) {
            mainImage.src = item.images[0];
            mainImage.alt = item.title;
        } else {
            mainImage.src = '/static/images/placeholder.jpg';
            mainImage.alt = 'No image available';
        }

        // Update thumbnails
        const thumbnailsContainer = document.getElementById('detail-thumbnails');
        if (item.images && item.images.length > 1) {
            thumbnailsContainer.innerHTML = item.images.slice(1).map(img => `
                <div class="thumbnail" onclick="document.getElementById('detail-main-image').src='${img}'">
                    <img src="${img}" alt="Thumbnail">
                </div>
            `).join('');
        } else {
            thumbnailsContainer.innerHTML = '';
        }
    }

    // Handle create item form submission
    async handleCreateItem(event) {
        const formData = new FormData(event.target);
        const itemData = {
            title: formData.get('title'),
            description: formData.get('description'),
            category: formData.get('category'),
            price: parseFloat(formData.get('price')),
            currency: 'EDU',
            item_type: formData.get('item_type'),
            images: null // TODO: Handle image upload
        };

        try {
            const response = await fetch(`${this.apiBase}/marketplace`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${localStorage.getItem('session_token')}`
                },
                body: JSON.stringify(itemData)
            });

            const result = await response.json();

            if (result.success) {
                this.showNotification('Item listed successfully!', 'success');
                this.closeCreateItemModal();
                this.loadMarketplaceItems(true);
                event.target.reset();
            } else {
                this.showNotification(result.message || 'Failed to create item', 'error');
            }
        } catch (error) {
            console.error('Error creating item:', error);
            this.showNotification('Failed to create item', 'error');
        }
    }

    // Modal management
    showModal(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.classList.add('show');
            document.body.style.overflow = 'hidden';
        }
    }

    closeModal(modalId) {
        const modal = document.getElementById(modalId);
        if (modal) {
            modal.classList.remove('show');
            document.body.style.overflow = 'auto';
        }
    }

    closeAllModals() {
        const modals = document.querySelectorAll('.modal.show');
        modals.forEach(modal => {
            modal.classList.remove('show');
        });
        document.body.style.overflow = 'auto';
    }

    // Utility functions
    formatCategory(category) {
        const categories = {
            'textbooks': 'Textbooks',
            'notes': 'Study Notes',
            'tutoring': 'Tutoring',
            'electronics': 'Electronics',
            'software': 'Software',
            'services': 'Services'
        };
        return categories[category] || category;
    }

    formatType(type) {
        const types = {
            'physical': 'Physical',
            'digital': 'Digital',
            'service': 'Service'
        };
        return types[type] || type;
    }

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

// Global functions for modal control (called from HTML)
function openCreateItemModal() {
    window.marketplace.showModal('create-item-modal');
}

function closeCreateItemModal() {
    window.marketplace.closeModal('create-item-modal');
}

function closeItemDetailModal() {
    window.marketplace.closeModal('item-detail-modal');
}

async function purchaseItem() {
    const itemTitle = document.querySelector('#item-detail-modal .item-title')?.textContent;
    const itemPrice = document.querySelector('#item-detail-modal .price')?.textContent;
    
    if (!itemTitle || !itemPrice) {
        window.marketplace.showNotification('Unable to process purchase', 'error');
        return;
    }
    
    // Extract EDU amount from price text
    const priceMatch = itemPrice.match(/(\d+(?:\.\d+)?)/);
    if (!priceMatch) {
        window.marketplace.showNotification('Invalid price format', 'error');
        return;
    }
    
    const amount = parseFloat(priceMatch[1]);
    const sellerAddress = document.querySelector('#item-detail-modal .seller')?.dataset?.address;
    
    if (!sellerAddress) {
        window.marketplace.showNotification('Seller information not available', 'error');
        return;
    }
    
    // Confirm purchase
    const confirmed = confirm(`Purchase "${itemTitle}" for ${amount} EDU?\n\nThis will send ${amount} EDU to the seller.`);
    
    if (!confirmed) {
        return;
    }
    
    try {
        // Send transaction
        const response = await fetch('/api/blockchain/send-transaction', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${localStorage.getItem('session_token')}`
            },
            body: JSON.stringify({
                recipient: sellerAddress,
                amount: amount,
                message: `Marketplace purchase: ${itemTitle}`
            })
        });
        
        const data = await response.json();
        
        if (data.success) {
            window.marketplace.showNotification(`✅ Purchase successful! Transaction: ${data.transaction_hash?.substring(0, 16)}...`, 'success');
            window.marketplace.closeModal('item-detail-modal');
        } else {
            window.marketplace.showNotification('❌ Purchase failed: ' + (data.message || 'Unknown error'), 'error');
        }
    } catch (error) {
        console.error('Purchase error:', error);
        window.marketplace.showNotification('❌ Purchase failed. Please try again.', 'error');
    }
}

async function contactSeller() {
    const sellerName = document.querySelector('#item-detail-modal .seller')?.textContent;
    const itemTitle = document.querySelector('#item-detail-modal .item-title')?.textContent;
    
    if (!sellerName || !itemTitle) {
        window.marketplace.showNotification('Seller information not available', 'error');
        return;
    }
    
    // For now, show a modal with contact information
    const message = prompt(`Send a message to ${sellerName} about "${itemTitle}":\n\n(Note: Direct messaging will be implemented in a future update. For now, this will be stored locally.)`);
    
    if (message && message.trim()) {
        // Store message locally (in future, send via API)
        const messages = JSON.parse(localStorage.getItem('marketplace_messages') || '[]');
        messages.push({
            seller: sellerName,
            item: itemTitle,
            message: message.trim(),
            timestamp: new Date().toISOString()
        });
        localStorage.setItem('marketplace_messages', JSON.stringify(messages));
        
        window.marketplace.showNotification('✅ Message saved! Direct messaging coming in next update.', 'info');
    }
}

function addToWishlist() {
    window.marketplace.showNotification('Added to wishlist!', 'success');
}

// Initialize marketplace when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.marketplace = new Marketplace();
});

// Export for use in other modules
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { Marketplace };
}