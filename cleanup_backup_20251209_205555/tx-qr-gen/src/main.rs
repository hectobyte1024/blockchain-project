// EduNet Voucher Generator - Creates 100 QR codes with 20 EDU each
// These work like promotional codes - users scan and redeem tokens

use qrcode::QrCode;
use serde::{Serialize, Deserialize};
use std::fs;
use image::{ImageBuffer, Luma, DynamicImage};

#[derive(Serialize, Deserialize, Clone)]
struct Voucher {
    id: String,
    code: String,
    amount: u64,
    status: String,
}

fn main() -> anyhow::Result<()> {
    println!("\n🎫 EduNet Voucher Generator");
    println!("==========================\n");
    println!("Generating 100 promotional vouchers with 20 EDU each...\n");

    let mut vouchers = Vec::new();
    fs::create_dir_all("vouchers/qr_codes")?;
    fs::create_dir_all("vouchers/qr_images")?;
    fs::create_dir_all("vouchers/printable")?;

    // Create master HTML page with all vouchers
    let mut html_all = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>EduNet Vouchers - Print All</title>
    <style>
        @media print {
            .voucher { page-break-after: always; }
            .no-print { display: none; }
        }
        body { 
            font-family: Arial, sans-serif; 
            margin: 0; 
            padding: 20px;
            background: #f5f5f5;
        }
        .voucher {
            width: 8.5in;
            height: 11in;
            margin: 0 auto 20px;
            padding: 40px;
            background: white;
            border: 2px dashed #333;
            box-sizing: border-box;
            position: relative;
        }
        .header {
            text-align: center;
            border-bottom: 3px solid #4CAF50;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }
        .header h1 {
            color: #4CAF50;
            margin: 0;
            font-size: 36px;
        }
        .header h2 {
            color: #666;
            margin: 10px 0 0 0;
            font-size: 24px;
        }
        .qr-container {
            text-align: center;
            margin: 30px 0;
        }
        .qr-container img {
            width: 300px;
            height: 300px;
            border: 3px solid #4CAF50;
            padding: 10px;
            background: white;
        }
        .code-box {
            background: #f9f9f9;
            border: 2px solid #4CAF50;
            padding: 20px;
            margin: 20px 0;
            text-align: center;
            font-size: 24px;
            font-weight: bold;
            font-family: 'Courier New', monospace;
            color: #333;
        }
        .instructions {
            margin-top: 30px;
            padding: 20px;
            background: #e8f5e9;
            border-radius: 10px;
        }
        .instructions h3 {
            color: #4CAF50;
            margin-top: 0;
        }
        .instructions ol {
            font-size: 18px;
            line-height: 1.8;
        }
        .warning {
            background: #fff3cd;
            border: 2px solid #ffc107;
            padding: 15px;
            margin-top: 20px;
            border-radius: 5px;
            text-align: center;
            font-weight: bold;
            color: #856404;
        }
        .footer {
            text-align: center;
            margin-top: 30px;
            padding-top: 20px;
            border-top: 2px solid #ddd;
            color: #999;
        }
        .no-print {
            position: fixed;
            top: 10px;
            right: 10px;
            background: #4CAF50;
            color: white;
            padding: 15px 30px;
            border-radius: 5px;
            cursor: pointer;
            font-size: 18px;
            border: none;
            box-shadow: 0 2px 5px rgba(0,0,0,0.2);
        }
        .no-print:hover {
            background: #45a049;
        }
    </style>
</head>
<body>
    <button class="no-print" onclick="window.print()">🖨️ Print All Vouchers</button>
"#);

    for i in 1..=100 {
        let id = format!("EDUNET-{:04}", i);
        let code = format!("PROMO-{}-20EDU", id);

        let v = Voucher {
            id: id.clone(),
            code: code.clone(),
            amount: 20,
            status: "unclaimed".to_string(),
        };

        // Generate QR code as image PNG
        let qr = QrCode::new(&code)?;
        
        // Convert QR code to pixels
        let colors = qr.to_colors();
        let size = qr.width();
        let scale = 10; // 10 pixels per module for clear printing
        let img_size = size * scale;
        
        let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(img_size as u32, img_size as u32);
        
        for y in 0..size {
            for x in 0..size {
                let color = if colors[y * size + x] == qrcode::Color::Dark {
                    Luma([0u8]) // Black
                } else {
                    Luma([255u8]) // White
                };
                
                // Scale up each module
                for dy in 0..scale {
                    for dx in 0..scale {
                        img.put_pixel(
                            (x * scale + dx) as u32,
                            (y * scale + dy) as u32,
                            color
                        );
                    }
                }
            }
        }
        
        img.save(format!("vouchers/qr_images/{}.png", id))?;

        // Generate QR code as text (for txt files)
        let qr_txt = qr.render::<qrcode::render::unicode::Dense1x2>()
            .dark_color(qrcode::render::unicode::Dense1x2::Light)
            .light_color(qrcode::render::unicode::Dense1x2::Dark)
            .build();

        // Save text version
        let content = format!(
            "╔════════════════════════════════╗\n\
             ║  EduNet Promotional Voucher   ║\n\
             ║  Code: {}            ║\n\
             ║  Value: 20 EDU Tokens         ║\n\
             ╚════════════════════════════════╝\n\
             \n\
             Scan this QR code:\n\
             \n{}\n\
             \n\
             Or enter code manually: {}\n\
             \n\
             Redeem at: http://localhost:8080\n\
             \n\
             Instructions:\n\
             1. Create EduNet account\n\
             2. Click \"Redeem Voucher\"\n\
             3. Scan QR or enter code\n\
             4. Get 20 EDU instantly!\n\
             \n\
             ⚠️  Can only be used once\n",
            id, qr_txt, code
        );
        fs::write(format!("vouchers/qr_codes/{}.txt", id), content)?;

        // Create individual HTML voucher
        let html_single = format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>EduNet Voucher - {}</title>
    <style>
        @media print {{
            .no-print {{ display: none; }}
            body {{ margin: 0; }}
        }}
        body {{ 
            font-family: Arial, sans-serif; 
            margin: 0; 
            padding: 20px;
            background: #f5f5f5;
        }}
        .voucher {{
            width: 8.5in;
            height: 11in;
            margin: 0 auto;
            padding: 40px;
            background: white;
            border: 2px dashed #333;
            box-sizing: border-box;
        }}
        .header {{
            text-align: center;
            border-bottom: 3px solid #4CAF50;
            padding-bottom: 20px;
            margin-bottom: 30px;
        }}
        .header h1 {{
            color: #4CAF50;
            margin: 0;
            font-size: 36px;
        }}
        .header h2 {{
            color: #666;
            margin: 10px 0 0 0;
            font-size: 24px;
        }}
        .qr-container {{
            text-align: center;
            margin: 30px 0;
        }}
        .qr-container img {{
            width: 300px;
            height: 300px;
            border: 3px solid #4CAF50;
            padding: 10px;
            background: white;
        }}
        .code-box {{
            background: #f9f9f9;
            border: 2px solid #4CAF50;
            padding: 20px;
            margin: 20px 0;
            text-align: center;
            font-size: 24px;
            font-weight: bold;
            font-family: 'Courier New', monospace;
            color: #333;
        }}
        .instructions {{
            margin-top: 30px;
            padding: 20px;
            background: #e8f5e9;
            border-radius: 10px;
        }}
        .instructions h3 {{
            color: #4CAF50;
            margin-top: 0;
        }}
        .instructions ol {{
            font-size: 18px;
            line-height: 1.8;
        }}
        .warning {{
            background: #fff3cd;
            border: 2px solid #ffc107;
            padding: 15px;
            margin-top: 20px;
            border-radius: 5px;
            text-align: center;
            font-weight: bold;
            color: #856404;
        }}
        .footer {{
            text-align: center;
            margin-top: 30px;
            padding-top: 20px;
            border-top: 2px solid #ddd;
            color: #999;
        }}
        .no-print {{
            position: fixed;
            top: 10px;
            right: 10px;
            background: #4CAF50;
            color: white;
            padding: 15px 30px;
            border-radius: 5px;
            cursor: pointer;
            font-size: 18px;
            border: none;
            box-shadow: 0 2px 5px rgba(0,0,0,0.2);
        }}
        .no-print:hover {{
            background: #45a049;
        }}
    </style>
</head>
<body>
    <button class="no-print" onclick="window.print()">🖨️ Print This Voucher</button>
    <div class="voucher">
        <div class="header">
            <h1>🎓 EduNet</h1>
            <h2>Promotional Voucher</h2>
        </div>
        
        <div class="qr-container">
            <img src="../qr_images/{}.png" alt="QR Code">
        </div>
        
        <div class="code-box">
            Voucher Code: {}<br>
            <small style="font-size: 18px;">PROMO-{}-20EDU</small>
        </div>
        
        <div style="text-align: center; font-size: 28px; color: #4CAF50; font-weight: bold; margin: 20px 0;">
            ✨ VALUE: 20 EDU TOKENS ✨
        </div>
        
        <div class="instructions">
            <h3>📱 How to Redeem:</h3>
            <ol>
                <li>Visit <strong>http://localhost:8080</strong></li>
                <li>Create an account or login</li>
                <li>Click <strong>"Redeem Voucher"</strong></li>
                <li>Scan the QR code above OR enter the code manually</li>
                <li>Receive 20 EDU tokens instantly!</li>
            </ol>
        </div>
        
        <div class="warning">
            ⚠️ This voucher can only be used ONCE • Keep it secure until redemption
        </div>
        
        <div class="footer">
            <p>EduNet - Empowering Students with Blockchain Technology</p>
            <p style="font-size: 12px;">For support, contact: support@edunet.edu</p>
        </div>
    </div>
</body>
</html>"#, id, id, id, id);

        fs::write(format!("vouchers/printable/{}.html", id), html_single)?;

        // Add to master HTML
        html_all.push_str(&format!(r#"
    <div class="voucher">
        <div class="header">
            <h1>🎓 EduNet</h1>
            <h2>Promotional Voucher</h2>
        </div>
        
        <div class="qr-container">
            <img src="../qr_images/{}.png" alt="QR Code">
        </div>
        
        <div class="code-box">
            Voucher Code: {}<br>
            <small style="font-size: 18px;">PROMO-{}-20EDU</small>
        </div>
        
        <div style="text-align: center; font-size: 28px; color: #4CAF50; font-weight: bold; margin: 20px 0;">
            ✨ VALUE: 20 EDU TOKENS ✨
        </div>
        
        <div class="instructions">
            <h3>📱 How to Redeem:</h3>
            <ol>
                <li>Visit <strong>http://localhost:8080</strong></li>
                <li>Create an account or login</li>
                <li>Click <strong>"Redeem Voucher"</strong></li>
                <li>Scan the QR code above OR enter the code manually</li>
                <li>Receive 20 EDU tokens instantly!</li>
            </ol>
        </div>
        
        <div class="warning">
            ⚠️ This voucher can only be used ONCE • Keep it secure until redemption
        </div>
        
        <div class="footer">
            <p>EduNet - Empowering Students with Blockchain Technology</p>
            <p style="font-size: 12px;">For support, contact: support@edunet.edu</p>
        </div>
    </div>
"#, id, id, id));

        vouchers.push(v);

        if i % 20 == 0 {
            println!("  ✅ Generated {}/100 vouchers", i);
        }
    }

    // Close master HTML
    html_all.push_str("</body>\n</html>");
    fs::write("vouchers/printable/ALL_VOUCHERS.html", html_all)?;

    // Save database
    fs::write(
        "vouchers/database.json",
        serde_json::to_string_pretty(&vouchers)?
    )?;

    // Create instructions
    let instructions = r#"
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
"#;

    fs::write("vouchers/INSTRUCTIONS.md", instructions)?;

    println!("\n╔════════════════════════════════════════════╗");
    println!("║      Generation Complete!                  ║");
    println!("╚════════════════════════════════════════════╝");
    println!();
    println!("📊 Summary:");
    println!("  ✅ Vouchers: 100");
    println!("  ✅ Total value: 2,000 EDU");
    println!("  ✅ Per voucher: 20 EDU");
    println!();
    println!("📁 Files Created:");
    println!("  📄 vouchers/database.json");
    println!("  📄 vouchers/printable/ALL_VOUCHERS.html (Print all at once!)");
    println!("  📄 vouchers/printable/EDUNET-*.html (100 individual pages)");
    println!("  🖼️  vouchers/qr_images/*.png (100 QR code images)");
    println!("  📝 vouchers/qr_codes/*.txt (100 text versions)");
    println!("  � vouchers/INSTRUCTIONS.md");
    println!();
    println!("🖨️  Ready to Print:");
    println!("  → Open vouchers/printable/ALL_VOUCHERS.html");
    println!("  → Click 'Print All Vouchers' button");
    println!("  → Each voucher prints on one page");
    println!();
    println!("✨ Professional, printer-ready vouchers!\n");

    Ok(())
}
