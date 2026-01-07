#!/bin/bash

# Generate vouchers for distribution
# Each voucher is worth 20 EDU

COUNT=${1:-10}

echo "=== Generating $COUNT Vouchers ==="
echo ""

RESPONSE=$(curl -s -X POST http://localhost:8080/api/voucher/generate \
  -H "Content-Type: application/json" \
  -d "{\"count\": $COUNT}")

echo "$RESPONSE" | jq .

# Save vouchers to file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
FILENAME="vouchers/vouchers_$TIMESTAMP.json"

mkdir -p vouchers
echo "$RESPONSE" | jq '.vouchers' > "$FILENAME"

echo ""
echo "✅ Vouchers saved to: $FILENAME"
echo ""

# Display vouchers in readable format
echo "=== Generated Voucher Codes ==="
echo "$RESPONSE" | jq -r '.vouchers[] | .code'
echo ""

echo "=== Distribution Instructions ==="
echo "1. Share voucher codes with users"
echo "2. Users can redeem at: http://your-site.com/api/voucher/redeem"
echo "3. Each voucher is worth 20 EDU"
echo "4. Vouchers are single-use only"
echo ""

# Count total EDU value
TOTAL_EDU=$(echo "$COUNT * 20" | bc)
echo "Total Value: $TOTAL_EDU EDU"
