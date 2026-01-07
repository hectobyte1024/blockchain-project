#!/bin/bash

# Script to convert voucher HTML to PDF
# Usage: ./generate_voucher_pdf.sh <count> <amount>
# Example: ./generate_voucher_pdf.sh 30 10

set -e

COUNT=${1:-30}
AMOUNT=${2:-10}
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
JSON_FILE="vouchers_${COUNT}x${AMOUNT}_${TIMESTAMP}.json"
OUTPUT_DIR="voucher-qr-codes-${TIMESTAMP}"
PDF_FILE="vouchers_${COUNT}x${AMOUNT}_${TIMESTAMP}.pdf"

echo "🎫 Generating $COUNT vouchers with $AMOUNT EDU each..."
echo ""

# Step 1: Generate vouchers via API
echo "📡 Step 1/3: Calling API to generate vouchers..."
curl -s -X POST http://localhost:8080/api/voucher/generate \
    -H "Content-Type: application/json" \
    -d "{\"count\":$COUNT,\"amount\":$AMOUNT}" \
    > "$JSON_FILE"

# Check if successful
if ! jq -e '.success' "$JSON_FILE" > /dev/null 2>&1; then
    echo "❌ Error: Failed to generate vouchers"
    cat "$JSON_FILE"
    exit 1
fi

ACTUAL_COUNT=$(jq -r '.count' "$JSON_FILE")
echo "✅ Generated $ACTUAL_COUNT vouchers"
echo ""

# Step 2: Generate QR codes as HTML
echo "📄 Step 2/3: Generating QR codes..."
./target/release/voucher-pdf-gen "$JSON_FILE" "$OUTPUT_DIR"
echo ""

# Step 3: Convert HTML to PDF
echo "🖨️  Step 3/3: Converting to PDF..."

# Check if wkhtmltopdf is available
if command -v wkhtmltopdf &> /dev/null; then
    echo "Using wkhtmltopdf..."
    wkhtmltopdf \
        --page-size A4 \
        --margin-top 10mm \
        --margin-bottom 10mm \
        --margin-left 10mm \
        --margin-right 10mm \
        --print-media-type \
        --enable-local-file-access \
        "$OUTPUT_DIR/vouchers.html" \
        "$PDF_FILE"
    echo "✅ PDF created: $PDF_FILE"
    
elif command -v chromium &> /dev/null || command -v chromium-browser &> /dev/null; then
    CHROME_BIN=$(command -v chromium 2>/dev/null || command -v chromium-browser 2>/dev/null)
    echo "Using $CHROME_BIN..."
    "$CHROME_BIN" \
        --headless \
        --disable-gpu \
        --print-to-pdf="$PDF_FILE" \
        --no-pdf-header-footer \
        --no-margins \
        "file://$(pwd)/$OUTPUT_DIR/vouchers.html"
    echo "✅ PDF created: $PDF_FILE"
    
elif command -v google-chrome &> /dev/null; then
    echo "Using Google Chrome..."
    google-chrome \
        --headless \
        --disable-gpu \
        --print-to-pdf="$PDF_FILE" \
        --no-pdf-header-footer \
        --no-margins \
        "file://$(pwd)/$OUTPUT_DIR/vouchers.html"
    echo "✅ PDF created: $PDF_FILE"
    
else
    echo "⚠️  No PDF converter found (wkhtmltopdf, chromium, or chrome)"
    echo "📂 HTML file ready: $OUTPUT_DIR/vouchers.html"
    echo ""
    echo "💡 To create PDF manually:"
    echo "   1. Open $OUTPUT_DIR/vouchers.html in your browser"
    echo "   2. Press Ctrl+P (or Cmd+P on Mac)"
    echo "   3. Select 'Save as PDF'"
    echo "   4. Enable 'Print backgrounds'"
    echo "   5. Save as $PDF_FILE"
    echo ""
    echo "💡 Or install a PDF converter:"
    echo "   sudo apt install wkhtmltopdf  # Debian/Ubuntu"
    echo "   sudo dnf install wkhtmltopdf  # Fedora/RHEL"
    echo "   brew install wkhtmltopdf      # macOS"
    exit 0
fi

echo ""
echo "✅ All done!"
echo "📄 PDF: $PDF_FILE"
echo "📂 QR codes: $OUTPUT_DIR/"
echo "💾 JSON: $JSON_FILE"
echo ""
echo "📊 Summary:"
echo "   - Vouchers: $ACTUAL_COUNT"
echo "   - Amount each: $AMOUNT EDU"
echo "   - Total value: $(echo "$ACTUAL_COUNT * $AMOUNT" | bc) EDU"
