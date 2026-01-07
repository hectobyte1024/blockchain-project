#!/bin/bash
# EduNet Blockchain Server Deployment Script

echo "🚀 Deploying EduNet Blockchain to Production Server"
echo "=================================================="

# Install required dependencies
echo "📦 Installing dependencies..."
sudo apt update && sudo apt upgrade -y
sudo apt install -y curl git build-essential cmake pkg-config libssl-dev nginx certbot python3-certbot-nginx

# Install Rust
echo "🦀 Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Clone your blockchain project
echo "📥 Cloning blockchain project..."
cd /opt
sudo mkdir -p edunet && sudo chown $USER:$USER edunet
cd edunet
git clone https://github.com/yourusername/edunet-blockchain.git .

# Build the blockchain
echo "🔨 Building blockchain..."
chmod +x build.sh
./build.sh
cargo build --release

# Create systemd service
echo "⚙️ Setting up system service..."
sudo tee /etc/systemd/system/edunet-blockchain.service > /dev/null << 'EOF'
[Unit]
Description=EduNet Blockchain Bootstrap Server
After=network.target

[Service]
Type=simple
User=blockchain
WorkingDirectory=/opt/edunet
ExecStart=/opt/edunet/target/release/edunet-gui --bootstrap
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Create blockchain user
sudo useradd -r -s /bin/false blockchain
sudo chown -R blockchain:blockchain /opt/edunet

# Configure firewall
echo "🔥 Configuring firewall..."
sudo ufw allow 22      # SSH
sudo ufw allow 80      # HTTP
sudo ufw allow 443     # HTTPS
sudo ufw allow 8080    # Web interface
sudo ufw allow 8333    # Blockchain P2P
sudo ufw --force enable

# Configure nginx reverse proxy
echo "🌐 Setting up nginx..."
sudo tee /etc/nginx/sites-available/edunet > /dev/null << 'EOF'
server {
    listen 80;
    server_name your-domain.com;  # CHANGE THIS
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_cache_bypass $http_upgrade;
    }
    
    location /ws {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header Origin http://localhost:8080;
    }
}
EOF

sudo ln -s /etc/nginx/sites-available/edunet /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx

# Start the blockchain service
echo "🚀 Starting EduNet blockchain..."
sudo systemctl daemon-reload
sudo systemctl enable edunet-blockchain
sudo systemctl start edunet-blockchain

echo ""
echo "✅ EduNet Blockchain deployed successfully!"
echo ""
echo "Next steps:"
echo "1. Point your domain to this server's IP: $(curl -s ifconfig.me)"
echo "2. Update nginx config with your domain name"
echo "3. Run: sudo certbot --nginx -d your-domain.com"
echo "4. Test: curl http://$(curl -s ifconfig.me):8080"
echo ""
echo "Your blockchain is now live at:"
echo "- Blockchain P2P: $(curl -s ifconfig.me):8333"
echo "- Web Interface: http://$(curl -s ifconfig.me):8080"
echo ""
echo "Check status: sudo systemctl status edunet-blockchain"
echo "View logs: sudo journalctl -u edunet-blockchain -f"