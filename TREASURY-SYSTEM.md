# Treasury Coin Sales System - Complete Guide

**Date:** December 10, 2025  
**Feature:** Manual Coin Sales with Real Cash Payments  
**Status:** ✅ Implemented (Phase 2.5)

---

## Overview

The Treasury Coin Sales System enables you to sell EDU coins to users who pay with **real-world money** (cash, bank transfers, checks, etc.). This is an off-chain payment system where:

1. User pays you with **real money** (off-chain)
2. You manually approve and execute the sale (on-chain)
3. User receives EDU coins in their wallet

---

## How It Works

### The Complete Flow

```
┌─────────────┐
│   User      │  "I want to buy 100 EDU coins"
└──────┬──────┘
       │
       ↓
┌─────────────────────────────────────────────────────┐
│  Step 1: User Requests Purchase                     │
│  - Email, form, in-person, phone, etc.              │
│  - They tell you: "I want X EDU coins"              │
└──────────────────────┬──────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────┐
│  Step 2: You Quote the Price                        │
│  RPC: treasury_getPrice                             │
│  - Current rate: $0.10 per EDU (configurable)       │
│  - Example: 100 EDU = $10.00                        │
│  - You tell them: "Send $10 to [payment method]"    │
└──────────────────────┬──────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────┐
│  Step 3: User Pays You (OFF-CHAIN)                  │
│  ✓ Cash in person                                   │
│  ✓ Cash by mail                                     │
│  ✓ Check                                            │
│  ✓ Bank transfer / wire                             │
│  ✓ Venmo / CashApp / Zelle                          │
│  ✓ PayPal                                           │
│  ✓ Any other payment method                         │
└──────────────────────┬──────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────┐
│  Step 4: You Confirm Payment Received               │
│  - You physically have the cash/check               │
│  - Or bank confirms transfer received               │
│  - Document receipt number or proof                 │
└──────────────────────┬──────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────┐
│  Step 5: You Execute Sale (ON-CHAIN)                │
│  RPC: treasury_sellCoins                            │
│  - Creates blockchain transaction                   │
│  - Sends coins from treasury to user wallet         │
│  - Records sale with payment proof                  │
└──────────────────────┬──────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────┐
│  Step 6: Transaction Gets Mined                     │
│  - Mining daemon includes tx in next block          │
│  - Block gets added to blockchain                   │
│  - Transaction becomes permanent                    │
└──────────────────────┬──────────────────────────────┘
                       │
                       ↓
┌─────────────┐
│   User      │  Has 100 EDU coins in wallet!
└─────────────┘  Can check balance, send to others, etc.
```

---

## RPC API Reference

### 1. treasury_getPrice

**Get current price per EDU coin**

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_getPrice",
    "params": [],
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "price_cents": 10,
    "price_usd": "$0.10"
  },
  "id": 1
}
```

---

### 2. treasury_setPrice

**Set the price per EDU coin (admin only)**

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_setPrice",
    "params": [25],
    "id": 2
  }'
```

**Parameters:**
- `price_cents` (u64): Price in cents (e.g., 25 = $0.25)

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "success": true
  },
  "id": 2
}
```

---

### 3. treasury_sellCoins

**Sell coins to a buyer (after receiving cash payment)**

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_sellCoins",
    "params": {
      "buyer_address": "edu1qBuyer000000000000000000000",
      "amount": 100,
      "payment_method": "cash",
      "payment_proof": "Receipt #12345 - $10 cash received on 2025-12-10"
    },
    "id": 3
  }'
```

**Parameters:**
- `buyer_address` (string): Buyer's EDU wallet address
- `amount` (u64): Number of EDU coins to send (in smallest unit)
- `payment_method` (string): How they paid (cash, check, bank_transfer, venmo, etc.)
- `payment_proof` (string): Receipt number or proof of payment

**Response (Success):**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "sale_id": "SALE-1702252800-64",
    "buyer_address": "edu1qBuyer000000000000000000000",
    "amount": 100,
    "price_per_edu_cents": 10,
    "total_payment_cents": 1000,
    "payment_method": "cash",
    "payment_proof": "Receipt #12345",
    "tx_hash": "abc123...",
    "timestamp": 1702252800,
    "status": "completed"
  },
  "id": 3
}
```

**Response (Error):**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32603,
    "message": "Internal error"
  },
  "id": 3
}
```

---

### 4. treasury_getStats

**Get treasury statistics and sales summary**

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_getStats",
    "params": [],
    "id": 4
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "total_sales": 5,
    "completed_sales": 4,
    "failed_sales": 1,
    "pending_sales": 0,
    "total_edu_sold": 500,
    "total_revenue_cents": 5000,
    "treasury_balance": 1499500,
    "current_price_cents": 10
  },
  "id": 4
}
```

**Fields:**
- `total_sales`: Total number of sale attempts
- `completed_sales`: Successfully completed sales
- `failed_sales`: Failed sales
- `pending_sales`: Sales in progress
- `total_edu_sold`: Total EDU coins sold
- `total_revenue_cents`: Total money received (in cents)
- `treasury_balance`: Remaining coins in treasury
- `current_price_cents`: Current price per EDU

---

### 5. treasury_getSales

**Get list of all sales**

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_getSales",
    "params": [],
    "id": 5
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "sale_id": "SALE-1702252800-100",
      "buyer_address": "edu1qBuyer1...",
      "amount": 100,
      "price_per_edu_cents": 10,
      "total_payment_cents": 1000,
      "payment_method": "cash",
      "payment_proof": "Receipt #001",
      "tx_hash": "abc123...",
      "timestamp": 1702252800,
      "status": "completed"
    }
  ],
  "id": 5
}
```

---

## Treasury Configuration

### Genesis Allocation

The treasury wallet is funded at genesis:

**Address:** `edu1qTreasury00000000000000000000`  
**Initial Balance:** 1,500,000 EDU (150000000000000 satoshis)  
**Purpose:** Platform treasury for coin sales and operations

### Default Price

**Initial Price:** $0.10 per EDU (10 cents)  
**Configurable:** Yes, via `treasury_setPrice`

---

## Usage Examples

### Example 1: Simple Cash Sale

```bash
# Scenario: Someone hands you $50 cash for 500 EDU coins

# 1. Check current price
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "treasury_getPrice", "params": [], "id": 1}'

# Result: $0.10 per EDU, so 500 EDU = $50 ✓

# 2. Receive $50 cash in person

# 3. Execute sale
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_sellCoins",
    "params": {
      "buyer_address": "edu1qAlice000000000000000000000",
      "amount": 500,
      "payment_method": "cash_in_person",
      "payment_proof": "Cash receipt #001 - $50 received 2025-12-10 3:30pm"
    },
    "id": 2
  }'

# Done! Alice now has 500 EDU in her wallet
```

### Example 2: Bank Transfer Sale

```bash
# Scenario: Bob wires you $1,000 for 10,000 EDU coins

# 1. Bob initiates wire transfer to your bank account

# 2. Your bank confirms transfer received

# 3. Execute sale
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_sellCoins",
    "params": {
      "buyer_address": "edu1qBob0000000000000000000000",
      "amount": 10000,
      "payment_method": "bank_wire",
      "payment_proof": "Wire confirmation #ABC123 - $1000 from Bob Smith - Ref: EDU-PURCHASE"
    },
    "id": 3
  }'
```

### Example 3: Bulk Sales with Price Update

```bash
# Raise price due to high demand
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_setPrice",
    "params": [50],
    "id": 1
  }'

# New price: $0.50 per EDU

# Sell 200 EDU to Carol for $100
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "treasury_sellCoins",
    "params": {
      "buyer_address": "edu1qCarol000000000000000000000",
      "amount": 200,
      "payment_method": "check",
      "payment_proof": "Check #5678 - $100 cleared 2025-12-11"
    },
    "id": 2
  }'
```

---

## Security Considerations

### Production Recommendations

1. **Protect RPC Access**
   ```bash
   # Use firewall to restrict RPC access
   # Only allow from trusted IPs
   iptables -A INPUT -p tcp --dport 8545 -s TRUSTED_IP -j ACCEPT
   iptables -A INPUT -p tcp --dport 8545 -j DROP
   ```

2. **Add Authentication**
   - Implement API key authentication
   - Use HTTPS/TLS for RPC
   - Consider VPN for admin access

3. **Multi-Signature for Large Sales**
   - Require multiple approvals for sales > $10,000
   - Keep cold storage for majority of treasury

4. **Payment Verification**
   - Always verify payment before executing sale
   - Keep receipts and proof of payment
   - Document everything for accounting

5. **Rate Limiting**
   - Limit sales per day/hour
   - Flag suspicious patterns
   - Monitor for fraud

---

## Accounting & Compliance

### Record Keeping

Every sale creates a `SaleRecord` with:
- Unique sale ID
- Buyer address
- Amount sold
- Price at time of sale
- Total payment received
- Payment method
- Payment proof/receipt
- Transaction hash
- Timestamp
- Status

### Tax Reporting

- Track total revenue in `treasury_getStats`
- Export sales records for tax filing
- Maintain separate accounting records
- Consult with tax professional

### Anti-Money Laundering (AML)

- Know Your Customer (KYC) for large purchases
- Document source of funds
- Report suspicious transactions
- Comply with local regulations

---

## Limitations & Future Enhancements

### Current Limitations

1. **No Automatic UTXO Selection**
   - Transaction creation is placeholder
   - Needs proper UTXO selection implementation
   - Manual wallet management required

2. **In-Memory Sale Records**
   - Sales stored in RAM only
   - Lost on restart
   - Should be persisted to database

3. **No Authentication**
   - Anyone with RPC access can call APIs
   - Production needs access control
   - Consider API keys or OAuth

4. **Basic Price Model**
   - Single global price
   - No dynamic pricing
   - No volume discounts

### Future Enhancements (Phase 3+)

1. **Automated Transaction Creation**
   - Implement UTXO selection algorithm
   - Automatic signing with treasury key
   - Handle change outputs

2. **Database Persistence**
   - SQLite for sale records
   - Full audit trail
   - Recovery on restart

3. **Payment Gateway Integration**
   - Stripe for credit cards
   - Coinbase Commerce for crypto
   - Automated payment verification

4. **Smart Contract Sales**
   - On-chain escrow
   - Atomic swaps
   - No manual intervention needed

5. **Advanced Features**
   - Volume discounts
   - Referral program
   - Loyalty rewards
   - Staking incentives

---

## Troubleshooting

### Sale Fails with "Treasury balance insufficient"

**Problem:** Not enough coins in treasury  
**Solution:** Check treasury balance, adjust sales limits

```bash
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "wallet_getBalance", "params": ["edu1qTreasury00000000000000000000"], "id": 1}'
```

### Transaction Not Getting Mined

**Problem:** Transaction stuck in mempool  
**Solution:** Check mining daemon is running, verify transaction fees

```bash
# Check if mining is enabled
ps aux | grep blockchain-node

# Enable mining if needed
./target/debug/blockchain-node --mining --validator-address miner1
```

### Price Changes Don't Apply

**Problem:** Old price still showing  
**Solution:** Verify RPC call, check node logs

```bash
# Check current price
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "treasury_getPrice", "params": [], "id": 1}'
```

---

## Testing

Run the included test script:

```bash
./test_treasury.sh
```

This will:
- Check treasury initialization
- Get current price
- View statistics
- Test price updates
- Show example sale commands

---

## Conclusion

The Treasury Coin Sales System provides a **simple, manual, and flexible** way to sell EDU coins for real money. It's designed for:

✅ **Off-chain payments** (cash, bank, any method)  
✅ **Manual approval** (you control every sale)  
✅ **Full audit trail** (every sale recorded)  
✅ **Regulatory compliance** (document everything)

This bridges the gap between traditional finance and blockchain, allowing you to accept familiar payment methods while providing blockchain-based coins.

**Status:** Production-ready for manual sales operations. Automated features coming in Phase 3.
