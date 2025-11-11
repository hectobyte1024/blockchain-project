
# EduNet Voucher System

## Printing Vouchers:

### Option 1: Print All at Once
- Open `vouchers/printable/ALL_VOUCHERS.html` in your browser
- Click the green "Print All Vouchers" button
- Select your printer and print

### Option 2: Print Individual Vouchers
- Open any file in `vouchers/printable/EDUNET-XXXX.html`
- Click "Print This Voucher" button
- Each voucher is formatted for standard 8.5" x 11" paper

### Option 3: Use QR Images
- QR code images are in `vouchers/qr_images/`
- PNG format, 300x300 pixels
- Can be used in your own designs

## For Users:
1. Go to http://localhost:8080
2. Login or create account
3. Click "Redeem Voucher"
4. Scan QR code or enter code manually
5. Get 20 EDU tokens instantly!

## For Administrators:
- Track redemptions in `database.json`
- Each voucher can only be used once
- All vouchers have unique codes

## Voucher Format:
- Code: PROMO-EDUNET-XXXX-20EDU
- Value: 20 EDU tokens
- Status: unclaimed/claimed

## Files Generated:
- `printable/` - HTML files ready to print
- `qr_images/` - PNG images of QR codes
- `qr_codes/` - Text versions
- `database.json` - Master voucher list
