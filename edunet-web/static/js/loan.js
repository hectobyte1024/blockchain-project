// Loan Module for EduNet Blockchain
// Handles loan applications, funding, and viewing

class LoanManager {
    constructor(edunetApp) {
        this.edunetApp = edunetApp;
        this.apiBase = '/api/loan';
        this.loans = [];
        this.myApplications = [];
        this.init();
    }

    async init() {
        console.log('Initializing Loan Manager...');
        await this.loadAllLoans();
        await this.loadMyApplications();
        this.setupEventListeners();
    }

    // Load all loans
    async loadAllLoans(status = 'all', limit = 50, offset = 0) {
        try {
            let url = `${this.apiBase}/list?limit=${limit}&offset=${offset}`;
            if (status !== 'all') {
                url += `&status=${status}`;
            }

            const response = await fetch(url);
            const result = await response.json();
            
            if (result.success) {
                this.loans = result.data;
                console.log(`Loaded ${this.loans.length} loans`);
                return this.loans;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error loading loans:', error);
            this.edunetApp.showNotification('Failed to load loans: ' + error.message, 'error');
            return [];
        }
    }

    // Load my loan applications
    async loadMyApplications() {
        try {
            const wallet = this.edunetApp.getCurrentWallet();
            if (!wallet) {
                console.warn('No wallet available');
                return [];
            }

            // Filter loans where borrower matches current wallet
            this.myApplications = this.loans.filter(loan => 
                loan.borrower_address === wallet.address
            );
            
            console.log(`Found ${this.myApplications.length} personal loan applications`);
            return this.myApplications;
        } catch (error) {
            console.error('Error loading my applications:', error);
            return [];
        }
    }

    // Apply for a loan
    async applyForLoan(applicationData) {
        try {
            const wallet = this.edunetApp.getCurrentWallet();
            if (!wallet) {
                throw new Error('No wallet available');
            }

            // Ensure all required fields are present
            const loanApplication = {
                full_name: applicationData.full_name,
                university: applicationData.university,
                field_of_study: applicationData.field_of_study,
                year_of_study: applicationData.year_of_study || 'Undergraduate',
                gpa: parseFloat(applicationData.gpa),
                test_score: parseInt(applicationData.test_score),
                academic_achievements: applicationData.academic_achievements || '',
                requested_amount: parseInt(applicationData.requested_amount), // in satoshis
                loan_purpose: applicationData.loan_purpose,
                loan_purpose_detail: applicationData.loan_purpose_detail || '',
                graduation_year: parseInt(applicationData.graduation_year),
                career_field: applicationData.career_field,
                expected_salary: parseInt(applicationData.expected_salary),
                repayment_term_months: parseInt(applicationData.repayment_term_months)
            };

            const response = await fetch(`${this.apiBase}/apply`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(loanApplication)
            });

            const result = await response.json();
            
            if (result.success) {
                console.log('Loan application submitted successfully:', result.data);
                this.edunetApp.showNotification(
                    `Loan application submitted! Your Proof-of-Potential score is ${result.data.proof_of_potential_score.toFixed(1)}/10`,
                    'success'
                );
                
                // Refresh loan lists
                await this.loadAllLoans();
                await this.loadMyApplications();
                
                return result.data;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error applying for loan:', error);
            this.edunetApp.showNotification('Failed to submit loan application: ' + error.message, 'error');
            throw error;
        }
    }

    // Fund a loan
    async fundLoan(loanId, amount) {
        try {
            const wallet = this.edunetApp.getCurrentWallet();
            if (!wallet) {
                throw new Error('No wallet available');
            }

            const fundingRequest = {
                loan_id: loanId,
                amount: parseInt(amount) // in satoshis
            };

            const response = await fetch(`${this.apiBase}/fund`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(fundingRequest)
            });

            const result = await response.json();
            
            if (result.success) {
                console.log('Loan funded successfully:', result.data);
                this.edunetApp.showNotification('Loan funded successfully!', 'success');
                
                // Refresh loan lists
                await this.loadAllLoans();
                await this.edunetApp.refreshWalletBalance();
                
                return result.data;
            } else {
                throw new Error(result.message);
            }
        } catch (error) {
            console.error('Error funding loan:', error);
            this.edunetApp.showNotification('Failed to fund loan: ' + error.message, 'error');
            throw error;
        }
    }

    // Render loan card
    renderLoanCard(loan) {
        const wallet = this.edunetApp.getCurrentWallet();
        const isMyLoan = wallet && loan.borrower_address === wallet.address;
        const amountEDU = (loan.requested_amount / 100000000).toFixed(2);
        const fundedEDU = (loan.amount_funded / 100000000).toFixed(2);
        const remainingEDU = (loan.amount_remaining / 100000000).toFixed(2);
        const percentFunded = ((loan.amount_funded / loan.requested_amount) * 100).toFixed(0);

        const statusColors = {
            'pending': 'status-pending',
            'funded': 'status-funded',
            'active': 'status-active',
            'repaid': 'status-repaid',
            'defaulted': 'status-defaulted'
        };

        return `
            <div class="loan-card ${isMyLoan ? 'my-loan' : ''}" data-loan-id="${loan.loan_id}">
                <div class="loan-header">
                    <div class="loan-borrower">
                        <div class="borrower-avatar">
                            <i class="fas fa-user-graduate"></i>
                        </div>
                        <div class="borrower-info">
                            <h3>${this.escapeHtml(loan.full_name)}</h3>
                            <p>${this.escapeHtml(loan.university)} • ${this.escapeHtml(loan.field_of_study)}</p>
                        </div>
                    </div>
                    <div class="loan-status ${statusColors[loan.status] || ''}">
                        ${loan.status.toUpperCase()}
                    </div>
                </div>

                <div class="loan-score">
                    <div class="score-label">Proof-of-Potential Score</div>
                    <div class="score-value">${loan.proof_of_potential_score.toFixed(1)}<span>/10</span></div>
                    <div class="score-breakdown">
                        <div class="score-item">
                            <span>GPA: ${loan.gpa.toFixed(2)}/4.0</span>
                            <span>Test: ${loan.test_score}</span>
                        </div>
                    </div>
                </div>

                <div class="loan-amount">
                    <div class="amount-requested">
                        <label>Requested Amount</label>
                        <span class="amount">${amountEDU} EDU</span>
                    </div>
                    <div class="amount-progress">
                        <div class="progress-bar">
                            <div class="progress-fill" style="width: ${percentFunded}%"></div>
                        </div>
                        <div class="progress-text">
                            <span>${fundedEDU} EDU funded</span>
                            <span>${percentFunded}%</span>
                        </div>
                    </div>
                </div>

                <div class="loan-details">
                    <div class="detail-item">
                        <i class="fas fa-graduation-cap"></i>
                        <span>Graduates ${loan.graduation_year}</span>
                    </div>
                    <div class="detail-item">
                        <i class="fas fa-briefcase"></i>
                        <span>${this.escapeHtml(loan.career_field)}</span>
                    </div>
                    <div class="detail-item">
                        <i class="fas fa-calendar"></i>
                        <span>${loan.repayment_term_months} months</span>
                    </div>
                    <div class="detail-item">
                        <i class="fas fa-dollar-sign"></i>
                        <span>Expected: $${loan.expected_salary.toLocaleString()}/yr</span>
                    </div>
                </div>

                <div class="loan-purpose">
                    <strong>Purpose:</strong> ${this.escapeHtml(loan.loan_purpose_detail || loan.loan_purpose)}
                </div>

                <div class="loan-actions">
                    ${!isMyLoan && loan.status === 'pending' ? `
                        <button class="btn-primary" onclick="loanManager.showFundModal('${loan.loan_id}', ${loan.amount_remaining})">
                            <i class="fas fa-hand-holding-usd"></i> Fund Loan
                        </button>
                    ` : ''}
                    ${isMyLoan ? `
                        <div class="my-loan-badge">
                            <i class="fas fa-user"></i> Your Application
                        </div>
                    ` : ''}
                    <button class="btn-secondary btn-sm" onclick="loanManager.showLoanDetails('${loan.loan_id}')">
                        <i class="fas fa-info-circle"></i> Details
                    </button>
                </div>
            </div>
        `;
    }

    // Render all loans to container
    renderLoans(containerId = 'loans-list', filterStatus = 'all') {
        const container = document.getElementById(containerId);
        if (!container) {
            console.warn(`Container ${containerId} not found`);
            return;
        }

        let loansToRender = this.loans;
        if (filterStatus !== 'all') {
            loansToRender = this.loans.filter(loan => loan.status === filterStatus);
        }

        if (loansToRender.length === 0) {
            container.innerHTML = `
                <div class="empty-state">
                    <i class="fas fa-hand-holding-usd fa-3x"></i>
                    <h3>No Loans ${filterStatus !== 'all' ? 'with status: ' + filterStatus : 'Available'}</h3>
                    <p>Be the first to apply for a Proof-of-Potential loan!</p>
                </div>
            `;
            return;
        }

        const loanCards = loansToRender.map(loan => this.renderLoanCard(loan)).join('');
        container.innerHTML = `<div class="loans-grid">${loanCards}</div>`;
    }

    // Show funding modal
    showFundModal(loanId, maxAmount) {
        const loan = this.loans.find(l => l.loan_id === loanId);
        if (!loan) {
            console.error('Loan not found:', loanId);
            return;
        }

        const maxEDU = (maxAmount / 100000000).toFixed(2);

        // Create modal if it doesn't exist
        let modal = document.getElementById('fund-loan-modal');
        if (!modal) {
            modal = this.createFundModal();
            document.body.appendChild(modal);
        }

        // Update modal content
        modal.dataset.loanId = loanId;
        modal.dataset.maxAmount = maxAmount;
        document.getElementById('fund-loan-title').textContent = `Fund Loan: ${loan.full_name}`;
        document.getElementById('fund-amount').max = maxEDU;
        document.getElementById('fund-amount').placeholder = `Max: ${maxEDU} EDU`;
        document.getElementById('fund-max-amount').textContent = `Maximum: ${maxEDU} EDU`;

        modal.classList.add('show');
    }

    // Create funding modal
    createFundModal() {
        const modal = document.createElement('div');
        modal.id = 'fund-loan-modal';
        modal.className = 'modal';
        modal.innerHTML = `
            <div class="modal-content">
                <div class="modal-header">
                    <h2 id="fund-loan-title">Fund Loan</h2>
                    <button class="modal-close" onclick="loanManager.closeFundModal()">
                        <i class="fas fa-times"></i>
                    </button>
                </div>
                <form id="fund-loan-form" class="modal-form">
                    <div class="form-group">
                        <label for="fund-amount">Funding Amount (EDU) *</label>
                        <input type="number" id="fund-amount" required 
                               min="0.01" step="0.01">
                        <small id="fund-max-amount">Maximum: 0 EDU</small>
                    </div>
                    <div class="info-box">
                        <i class="fas fa-info-circle"></i>
                        <div>
                            <strong>Note:</strong> You can partially fund this loan. Multiple funders can contribute to the same loan.
                        </div>
                    </div>
                    <div class="form-actions">
                        <button type="button" class="btn-secondary" onclick="loanManager.closeFundModal()">Cancel</button>
                        <button type="submit" class="btn-primary">Fund Loan</button>
                    </div>
                </form>
            </div>
        `;

        // Add submit handler
        modal.querySelector('#fund-loan-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            const loanId = modal.dataset.loanId;
            const maxAmount = parseInt(modal.dataset.maxAmount);
            const amountEDU = parseFloat(document.getElementById('fund-amount').value);
            const amountSatoshis = Math.floor(amountEDU * 100000000);

            if (amountSatoshis > maxAmount) {
                this.edunetApp.showNotification('Amount exceeds remaining loan amount', 'error');
                return;
            }

            await this.fundLoan(loanId, amountSatoshis);
            this.closeFundModal();
            this.renderLoans(); // Refresh loans
        });

        return modal;
    }

    // Close funding modal
    closeFundModal() {
        const modal = document.getElementById('fund-loan-modal');
        if (modal) {
            modal.classList.remove('show');
        }
    }

    // Show loan details
    showLoanDetails(loanId) {
        const loan = this.loans.find(l => l.loan_id === loanId);
        if (!loan) {
            console.error('Loan not found:', loanId);
            return;
        }

        const amountEDU = (loan.requested_amount / 100000000).toFixed(2);
        const fundedEDU = (loan.amount_funded / 100000000).toFixed(2);

        this.edunetApp.showNotification(
            `<strong>${loan.full_name}</strong><br>` +
            `${loan.university} - ${loan.field_of_study}<br>` +
            `Amount: ${amountEDU} EDU (${fundedEDU} funded)<br>` +
            `Score: ${loan.proof_of_potential_score.toFixed(1)}/10<br>` +
            `Status: ${loan.status}`,
            'info',
            10000
        );
    }

    // Setup event listeners
    setupEventListeners() {
        // Loan application form
        const loanForm = document.getElementById('loan-application-form');
        if (loanForm) {
            loanForm.addEventListener('submit', async (e) => {
                e.preventDefault();
                
                const formData = {
                    full_name: document.getElementById('loan-full-name').value,
                    university: document.getElementById('loan-university').value,
                    field_of_study: document.getElementById('loan-field').value,
                    year_of_study: document.getElementById('loan-year').value,
                    gpa: document.getElementById('loan-gpa').value,
                    test_score: document.getElementById('loan-test-score').value,
                    academic_achievements: document.getElementById('loan-achievements').value,
                    requested_amount: Math.floor(parseFloat(document.getElementById('loan-amount').value) * 100000000),
                    loan_purpose: document.getElementById('loan-purpose-select').value,
                    loan_purpose_detail: document.getElementById('loan-purpose-detail').value,
                    graduation_year: document.getElementById('loan-grad-year').value,
                    career_field: document.getElementById('loan-career').value,
                    expected_salary: document.getElementById('loan-salary').value,
                    repayment_term_months: parseInt(document.getElementById('loan-term').value.split(' ')[0])
                };

                await this.applyForLoan(formData);
                loanForm.reset();
            });
        }

        // Apply button in header
        const applyBtn = document.querySelector('.header-right .btn-primary');
        if (applyBtn && applyBtn.textContent.includes('Apply')) {
            applyBtn.addEventListener('click', () => {
                document.querySelector('.loan-application')?.scrollIntoView({ behavior: 'smooth' });
            });
        }
    }

    // Calculate Proof-of-Potential score (client-side preview)
    calculateScore(gpa, testScore) {
        let score = 5.0; // Base score
        
        // GPA contribution (up to 2.5 points)
        score += (parseFloat(gpa) / 4.0) * 2.5;
        
        // Test score contribution (up to 2.5 points)
        score += (parseInt(testScore) / 1600) * 2.5;
        
        return Math.min(score, 10.0);
    }

    // Utility: Escape HTML
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}

// Initialize Loan manager when DOM loads
let loanManager;

document.addEventListener('DOMContentLoaded', () => {
    if (window.edunetApp) {
        loanManager = new LoanManager(window.edunetApp);
        window.loanManager = loanManager;
        console.log('Loan Manager initialized');
    } else {
        console.error('EdunetApp not found - Loan Manager cannot initialize');
    }
});
