// Wallet Management JavaScript

let wallets = [];
let selectedWallet = null;

// Initialize wallet page
document.addEventListener('DOMContentLoaded', function() {
    // Wait for shared app to be available
    if (window.edunetApp) {
        loadWallets();
    } else {
        // Poll for shared app availability
        const checkApp = setInterval(() => {
            if (window.edunetApp) {
                clearInterval(checkApp);
                loadWallets();
            }
        }, 100);
    }
});

// Load all wallets
async function loadWallets() {
    try {
        const response = await fetch('/api/wallets');
        const data = await response.json();
        
        if (data.success) {
            wallets = data.data;
            updateWalletUI();
            updateHeaderBalance(); // Update header with total balance
        } else {
            showError('Failed to load wallets: ' + data.message);
        }
    } catch (error) {
        console.error('Error loading wallets:', error);
        showError('Failed to connect to wallet service');
    }
}

// Update header balance with real blockchain data
async function updateHeaderBalance() {
    if (wallets.length === 0) return;
    
    try {
        // Use global app wallet if available
        let wallet = null;
        if (window.edunetApp && window.edunetApp.getCurrentWallet()) {
            wallet = window.edunetApp.getCurrentWallet();
        } else {
            wallet = selectedWallet || wallets[0];
        }
        
        const response = await fetch(`/api/blockchain/balance/${wallet.address}`);
        const data = await response.json();
        
        if (data.success) {
            // Update through global app if available
            if (window.edunetApp) {
                window.edunetApp.updateWalletDisplay(data.data.balance);
            } else {
                // Fallback to direct update
                const headerBalance = document.getElementById('header-wallet-balance');
                if (headerBalance) {
                    headerBalance.textContent = `${data.data.balance.toFixed(2)} EDU`;
                }
                
                const walletBalance = document.getElementById('wallet-balance');
                if (walletBalance) {
                    walletBalance.textContent = `${data.data.balance.toFixed(2)} EDU`;
                }
            }
        }
    } catch (error) {
        console.error('Error updating header balance:', error);
    }
}

// Update wallet UI based on loaded data
function updateWalletUI() {
    const loading = document.getElementById('loading');
    const noWallets = document.getElementById('no-wallets');
    const walletContent = document.getElementById('wallet-content');
    
    loading.style.display = 'none';
    
    if (wallets.length === 0) {
        noWallets.style.display = 'block';
        walletContent.style.display = 'none';
    } else {
        noWallets.style.display = 'none';
        walletContent.style.display = 'block';
        populateWalletSelectors();
        
        // Auto-select first wallet if none selected
        if (!selectedWallet && wallets.length > 0) {
            selectWallet(wallets[0].id);
        }
    }
}

// Populate wallet selector dropdowns
function populateWalletSelectors() {
    const selectors = ['wallet-select', 'fromWallet', 'receiveWallet'];
    
    selectors.forEach(selectorId => {
        const select = document.getElementById(selectorId);
        if (select) {
            // Clear existing options (except first)
            while (select.children.length > 1) {
                select.removeChild(select.lastChild);
            }
            
            // Add wallet options
            wallets.forEach(wallet => {
                const option = document.createElement('option');
                option.value = wallet.id;
                option.textContent = `${wallet.name} (${wallet.balance.toFixed(2)} EDU)`;
                select.appendChild(option);
            });
        }
    });
}

// Select a wallet
async function selectWallet(walletId) {
    if (!walletId) {
        selectedWallet = null;
        document.getElementById('wallet-details').style.display = 'none';
        document.getElementById('wallet-qr').style.display = 'none';
        document.getElementById('no-wallet-selected').style.display = 'block';
        document.getElementById('transaction-list').innerHTML = '<p>Select a wallet to view transaction history</p>';
        return;
    }
    
    try {
        const response = await fetch(`/api/wallets/${walletId}`);
        const data = await response.json();
        
        if (data.success) {
            selectedWallet = data.data;
            updateSelectedWalletUI();
            loadTransactionHistory();
        } else {
            showError('Failed to load wallet details: ' + data.message);
        }
    } catch (error) {
        console.error('Error loading wallet details:', error);
        showError('Failed to load wallet details');
    }
}

// Update UI for selected wallet
function updateSelectedWalletUI() {
    if (!selectedWallet) return;
    
    const wallet = selectedWallet.wallet;
    
    // Update wallet details
    document.getElementById('wallet-balance').textContent = `${wallet.balance.toFixed(2)} EDU`;
    document.getElementById('wallet-address').textContent = wallet.address;
    document.getElementById('wallet-details').style.display = 'block';
    
    // Update QR code
    generateWalletQR(wallet.id);
    
    document.getElementById('no-wallet-selected').style.display = 'none';
}

// Generate QR code for wallet
async function generateWalletQR(walletId) {
    try {
        const response = await fetch(`/api/wallets/${walletId}/qr`);
        const data = await response.json();
        
        if (data.success) {
            const qrImage = document.getElementById('qr-image');
            qrImage.src = 'data:image/png;base64,' + data.data.qr_code;
            document.getElementById('wallet-qr').style.display = 'block';
        } else {
            console.error('Failed to generate QR code:', data.message);
        }
    } catch (error) {
        console.error('Error generating QR code:', error);
    }
}

// Load transaction history for selected wallet
function loadTransactionHistory() {
    if (!selectedWallet || !selectedWallet.transactions) {
        document.getElementById('transaction-list').innerHTML = '<p>No transactions found</p>';
        return;
    }
    
    const transactions = selectedWallet.transactions;
    const listContainer = document.getElementById('transaction-list');
    
    if (transactions.length === 0) {
        listContainer.innerHTML = '<p>No transactions found</p>';
        return;
    }
    
    listContainer.innerHTML = '';
    
    transactions.forEach(tx => {
        const txElement = document.createElement('div');
        const isIncoming = tx.to_address === selectedWallet.wallet.address;
        
        txElement.className = `transaction-item ${isIncoming ? 'transaction-incoming' : 'transaction-outgoing'}`;
        
        const statusBadge = getStatusBadge(tx.status);
        const date = new Date(tx.created_at).toLocaleDateString();
        
        txElement.innerHTML = `
            <div>
                <strong>${isIncoming ? '📥 Received' : '📤 Sent'}</strong> ${tx.amount_edu.toFixed(2)} EDU
                <br>
                <small>${isIncoming ? 'From' : 'To'}: ${isIncoming ? tx.from_address : tx.to_address}</small>
                ${tx.message ? `<br><small>💬 ${tx.message}</small>` : ''}
            </div>
            <div style="text-align: right;">
                ${statusBadge}
                <br>
                <small>${date}</small>
            </div>
        `;
        
        listContainer.appendChild(txElement);
    });
}

// Get status badge for transaction
function getStatusBadge(status) {
    const badges = {
        'pending': '<span style="background: #f39c12; color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.8em;">⏳ Pending</span>',
        'broadcasting': '<span style="background: #3498db; color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.8em;">📡 Broadcasting</span>',
        'confirmed': '<span style="background: #27ae60; color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.8em;">✅ Confirmed</span>',
        'failed': '<span style="background: #e74c3c; color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.8em;">❌ Failed</span>'
    };
    
    return badges[status] || '<span style="background: #95a5a6; color: white; padding: 2px 8px; border-radius: 4px; font-size: 0.8em;">❓ Unknown</span>';
}

// Modal functions
function createWalletModal() {
    document.getElementById('createWalletModal').style.display = 'block';
}

function sendTransactionModal() {
    if (wallets.length === 0) {
        showError('Please create a wallet first');
        return;
    }
    document.getElementById('sendModal').style.display = 'block';
}

function receiveModal() {
    if (wallets.length === 0) {
        showError('Please create a wallet first');
        return;
    }
    document.getElementById('receiveModal').style.display = 'block';
}

function closeModal(modalId) {
    document.getElementById(modalId).style.display = 'none';
    
    // Clear forms
    if (modalId === 'createWalletModal') {
        document.getElementById('createWalletForm').reset();
    } else if (modalId === 'sendModal') {
        document.getElementById('sendForm').reset();
    } else if (modalId === 'qrScanModal') {
        document.getElementById('qrInput').value = '';
        document.getElementById('qrParseResult').innerHTML = '';
    }
}

// Create new wallet
async function createWallet(event) {
    event.preventDefault();
    
    const walletName = document.getElementById('walletName').value.trim();
    if (!walletName) {
        showError('Please enter a wallet name');
        return;
    }
    
    try {
        const response = await fetch('/api/wallets', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                name: walletName
            })
        });
        
        const data = await response.json();
        
        if (data.success) {
            showSuccess('Wallet created successfully!');
            closeModal('createWalletModal');
            await loadWallets(); // Refresh wallet list
        } else {
            showError('Failed to create wallet: ' + data.message);
        }
    } catch (error) {
        console.error('Error creating wallet:', error);
        showError('Failed to create wallet');
    }
}

// Send transaction
async function sendTransaction(event) {
    event.preventDefault();
    
    const fromWalletId = document.getElementById('fromWallet').value;
    const toAddress = document.getElementById('toAddress').value.trim();
    const amount = parseFloat(document.getElementById('amount').value);
    const message = document.getElementById('message').value.trim();
    
    if (!fromWalletId) {
        showError('Please select a wallet to send from');
        return;
    }
    
    if (!toAddress) {
        showError('Please enter recipient address');
        return;
    }
    
    if (!amount || amount <= 0) {
        showError('Please enter a valid amount');
        return;
    }
    
    try {
        const response = await fetch('/api/wallets/send', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                from_wallet_id: fromWalletId,
                to_address: toAddress,
                amount: amount,
                message: message
            })
        });
        
        const data = await response.json();
        
        if (data.success) {
            showSuccess('Transaction sent successfully! Transaction ID: ' + data.data.transaction_id);
            closeModal('sendModal');
            await loadWallets(); // Refresh wallet list
            if (selectedWallet) {
                await selectWallet(selectedWallet.wallet.id); // Refresh selected wallet
            }
        } else {
            showError('Failed to send transaction: ' + data.message);
        }
    } catch (error) {
        console.error('Error sending transaction:', error);
        showError('Failed to send transaction');
    }
}

// Generate payment request QR
async function generatePaymentRequest() {
    const walletId = document.getElementById('receiveWallet').value;
    const amount = parseFloat(document.getElementById('requestAmount').value) || null;
    const message = document.getElementById('requestMessage').value.trim() || null;
    
    if (!walletId) {
        document.getElementById('paymentRequestQR').style.display = 'none';
        return;
    }
    
    try {
        const response = await fetch('/api/wallets/payment-request', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                wallet_id: walletId,
                amount: amount,
                message: message
            })
        });
        
        const data = await response.json();
        
        if (data.success) {
            const qrImage = document.getElementById('paymentQRImage');
            qrImage.src = 'data:image/png;base64,' + data.data.qr_code;
            
            document.getElementById('paymentQRData').textContent = data.data.qr_data;
            document.getElementById('paymentRequestQR').style.display = 'block';
        } else {
            console.error('Failed to generate payment request:', data.message);
        }
    } catch (error) {
        console.error('Error generating payment request:', error);
    }
}

// Copy payment request data
function copyPaymentRequest() {
    const qrData = document.getElementById('paymentQRData').textContent;
    
    if (navigator.clipboard) {
        navigator.clipboard.writeText(qrData).then(() => {
            showSuccess('Payment request copied to clipboard!');
        });
    } else {
        // Fallback for older browsers
        const textArea = document.createElement('textarea');
        textArea.value = qrData;
        document.body.appendChild(textArea);
        textArea.select();
        document.execCommand('copy');
        document.body.removeChild(textArea);
        showSuccess('Payment request copied to clipboard!');
    }
}

// Scan QR code
function scanQRCode() {
    document.getElementById('qrScanModal').style.display = 'block';
}

// Parse QR code data
async function parseQRData() {
    const qrInput = document.getElementById('qrInput').value.trim();
    const resultDiv = document.getElementById('qrParseResult');
    
    if (!qrInput) {
        resultDiv.innerHTML = '<div class="error">Please enter QR data</div>';
        return;
    }
    
    try {
        const response = await fetch('/api/wallets/parse-qr', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                qr_data: qrInput
            })
        });
        
        const data = await response.json();
        
        if (data.success) {
            const paymentRequest = data.data;
            
            let html = '<div class="success">QR Code Parsed Successfully!</div>';
            html += `<p><strong>Address:</strong> ${paymentRequest.address}</p>`;
            
            if (paymentRequest.amount) {
                html += `<p><strong>Amount:</strong> ${paymentRequest.amount} EDU</p>`;
            }
            
            if (paymentRequest.message) {
                html += `<p><strong>Message:</strong> ${paymentRequest.message}</p>`;
            }
            
            html += '<button class="btn-wallet btn-success" onclick="fillSendForm(\'' + 
                    JSON.stringify(paymentRequest).replace(/'/g, "\\'") + '\')">Use in Send Form</button>';
            
            resultDiv.innerHTML = html;
        } else {
            // Try as plain address
            if (qrInput.length === 34 && qrInput.match(/^[13][a-km-zA-HJ-NP-Z1-9]{25,34}$/)) {
                resultDiv.innerHTML = `
                    <div class="success">Valid Address Found!</div>
                    <p><strong>Address:</strong> ${qrInput}</p>
                    <button class="btn-wallet btn-success" onclick="fillSendFormAddress('${qrInput}')">Use in Send Form</button>
                `;
            } else {
                resultDiv.innerHTML = '<div class="error">Invalid QR code or address format</div>';
            }
        }
    } catch (error) {
        console.error('Error parsing QR data:', error);
        resultDiv.innerHTML = '<div class="error">Failed to parse QR data</div>';
    }
}

// Fill send form with payment request data
function fillSendForm(paymentRequestJson) {
    const paymentRequest = JSON.parse(paymentRequestJson);
    
    closeModal('qrScanModal');
    document.getElementById('sendModal').style.display = 'block';
    
    document.getElementById('toAddress').value = paymentRequest.address;
    if (paymentRequest.amount) {
        document.getElementById('amount').value = paymentRequest.amount;
    }
    if (paymentRequest.message) {
        document.getElementById('message').value = paymentRequest.message;
    }
}

// Fill send form with just address
function fillSendFormAddress(address) {
    closeModal('qrScanModal');
    document.getElementById('sendModal').style.display = 'block';
    document.getElementById('toAddress').value = address;
}

// Refresh wallets
async function refreshWallets() {
    await loadWallets();
    if (selectedWallet) {
        await selectWallet(selectedWallet.wallet.id);
    }
    showSuccess('Wallets refreshed!');
}

// Utility functions
function showError(message) {
    // Create error notification
    const errorDiv = document.createElement('div');
    errorDiv.className = 'error';
    errorDiv.textContent = message;
    errorDiv.style.position = 'fixed';
    errorDiv.style.top = '20px';
    errorDiv.style.right = '20px';
    errorDiv.style.zIndex = '2000';
    errorDiv.style.maxWidth = '400px';
    
    document.body.appendChild(errorDiv);
    
    setTimeout(() => {
        if (errorDiv.parentNode) {
            errorDiv.parentNode.removeChild(errorDiv);
        }
    }, 5000);
}

function showSuccess(message) {
    // Create success notification
    const successDiv = document.createElement('div');
    successDiv.className = 'success';
    successDiv.textContent = message;
    successDiv.style.position = 'fixed';
    successDiv.style.top = '20px';
    successDiv.style.right = '20px';
    successDiv.style.zIndex = '2000';
    successDiv.style.maxWidth = '400px';
    
    document.body.appendChild(successDiv);
    
    setTimeout(() => {
        if (successDiv.parentNode) {
            successDiv.parentNode.removeChild(successDiv);
        }
    }, 3000);
}

// Close modals when clicking outside
window.onclick = function(event) {
    const modals = document.getElementsByClassName('modal');
    for (let modal of modals) {
        if (event.target === modal) {
            modal.style.display = 'none';
        }
    }
}