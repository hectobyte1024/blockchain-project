use anyhow::Result;
use qrcode::QrCode;
use qrcode::render::svg;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

#[derive(Debug, Deserialize, Serialize)]
struct Voucher {
    code: String,
    amount: f64,
    status: String,
}

#[derive(Debug, Deserialize)]
struct VoucherResponse {
    success: bool,
    count: usize,
    vouchers: Vec<Voucher>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <vouchers.json> [output-dir]", args[0]);
        eprintln!("Example: {} vouchers_30x10.json ./voucher-qr-codes", args[0]);
        std::process::exit(1);
    }
    
    let input_file = &args[1];
    let output_dir = if args.len() > 2 {
        args[2].clone()
    } else {
        "voucher-qr-codes".to_string()
    };
    
    // Create output directory
    std::fs::create_dir_all(&output_dir)?;
    
    // Read vouchers from JSON
    let json_data = std::fs::read_to_string(input_file)?;
    let response: VoucherResponse = serde_json::from_str(&json_data)?;
    
    println!("📄 Generating {} QR codes as SVG files...", response.vouchers.len());
    
    // Generate HTML file with all QR codes for easy printing
    let mut html = String::from(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>EduNet Vouchers</title>
    <style>
        @page {
            size: A4;
            margin: 10mm;
        }
        body {
            font-family: Arial, sans-serif;
            margin: 0;
            padding: 20px;
        }
        .voucher-grid {
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 15px;
            page-break-inside: avoid;
        }
        .voucher {
            border: 2px dashed #ccc;
            padding: 10px;
            text-align: center;
            page-break-inside: avoid;
            background: white;
        }
        .qr-code {
            width: 150px;
            height: 150px;
            margin: 10px auto;
        }
        .voucher-code {
            font-size: 12px;
            font-weight: bold;
            color: #333;
            margin: 5px 0;
            word-break: break-all;
        }
        .voucher-amount {
            font-size: 14px;
            font-weight: bold;
            color: #2196F3;
            margin: 5px 0;
        }
        .instructions {
            font-size: 9px;
            color: #666;
            margin-top: 5px;
        }
        h1 {
            text-align: center;
            color: #2196F3;
        }
        @media print {
            .no-print { display: none; }
        }
    </style>
</head>
<body>
    <h1>🎓 EduNet Vouchers</h1>
    <p class="no-print" style="text-align: center; color: #666;">
        Print this page to get physical voucher cards. Cut along the dashed lines.
    </p>
    <div class="voucher-grid">
"#);
    
    // Generate QR codes
    for (idx, voucher) in response.vouchers.iter().enumerate() {
        // Generate QR code as SVG
        let qr = QrCode::new(voucher.code.as_bytes())?;
        let svg_string = qr.render()
            .min_dimensions(200, 200)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build();
        
        // Save individual SVG file
        let svg_filename = format!("{}/voucher_{:03}_{}.svg", output_dir, idx + 1, voucher.code);
        let mut svg_file = File::create(&svg_filename)?;
        svg_file.write_all(svg_string.as_bytes())?;
        
        // Add to HTML
        html.push_str(&format!(r#"
        <div class="voucher">
            <div class="qr-code">{}</div>
            <div class="voucher-code">{}</div>
            <div class="voucher-amount">{:.1} EDU</div>
            <div class="instructions">Scan to redeem</div>
        </div>
"#, svg_string, voucher.code, voucher.amount));
    }
    
    html.push_str(r#"
    </div>
    <script class="no-print">
        window.onload = function() {
            console.log('Vouchers loaded. Use Ctrl+P or Cmd+P to print.');
        };
    </script>
</body>
</html>
"#);
    
    // Save HTML file
    let html_filename = format!("{}/vouchers.html", output_dir);
    let mut html_file = File::create(&html_filename)?;
    html_file.write_all(html.as_bytes())?;
    
    println!("✅ Generated {} QR codes", response.vouchers.len());
    println!("📂 Output directory: {}", output_dir);
    println!("🌐 HTML file: {}/vouchers.html", output_dir);
    println!("\n💡 To print:");
    println!("   1. Open {}/vouchers.html in your browser", output_dir);
    println!("   2. Press Ctrl+P (or Cmd+P on Mac)");
    println!("   3. Set 'Print backgrounds' to ON");
    println!("   4. Print or Save as PDF");
    
    Ok(())
}
