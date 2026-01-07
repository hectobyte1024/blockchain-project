# Deploying EduNet to Remote Debian Server

## 🚀 Quick Start

### From Your Current Computer:

```bash
./deploy-to-remote-server.sh
```

This interactive script will:
1. Ask for your server's IP address
2. Copy all files to the server
3. Build everything on the server
4. Set up systemd services
5. Configure firewall rules

---

## 📋 Manual Deployment Steps

If you prefer manual control:

### 1. Prepare Your Server

SSH into your Debian server:
```bash
ssh user@your-server-ip
```

Install dependencies:
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install build tools
sudo apt install -y build-essential cmake git sqlite3 libsqlite3-dev pkg-config libssl-dev

# Optional: Install nginx for HTTPS
sudo apt install -y nginx certbot python3-certbot-nginx
```

### 2. Copy Project to Server

From your current computer:
```bash
# Option A: Using rsync (recommended)
rsync -avz --progress \
    --exclude 'target' \
    --exclude 'cpp-core/build' \
    --exclude '*.log' \
    --exclude 'blockchain-data' \
    ./ user@your-server-ip:~/edunet/

# Option B: Using scp
scp -r ./ user@your-server-ip:~/edunet/

# Option C: Using git (if you have a repo)
# On server:
git clone your-repo-url ~/edunet
```

### 3. Build on Server

SSH into server and build:
```bash
ssh user@your-server-ip
cd ~/edunet

# Build C++ core
cd cpp-core/build
cmake ..
make -j$(nproc)
cd ../..

# Build Rust binaries
cargo build --release --bin blockchain-node
cargo build --release --bin edunet-web
```

### 4. Configure Firewall

```bash
sudo apt install -y ufw

# Allow necessary ports
sudo ufw allow 22/tcp      # SSH (don't forget this!)
sudo ufw allow 8080/tcp    # Web interface
sudo ufw allow 9000/tcp    # P2P network
# sudo ufw allow 8545/tcp  # RPC (only if exposing publicly)

# Enable firewall
sudo ufw enable
sudo ufw status
```

### 5. Create Systemd Services

Create blockchain node service:
```bash
sudo nano /etc/systemd/system/edunet-node.service
```

Paste this:
```ini
[Unit]
Description=EduNet Blockchain Node
After=network.target

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME/edunet
ExecStart=/home/YOUR_USERNAME/edunet/target/release/blockchain-node \
    --rpc-port 8545 \
    --p2p-port 9000 \
    --data-dir /home/YOUR_USERNAME/edunet/blockchain-data \
    --mining true \
    --validator-address YOUR_WALLET_ADDRESS
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Create web service:
```bash
sudo nano /etc/systemd/system/edunet-web.service
```

Paste this:
```ini
[Unit]
Description=EduNet Web Interface
After=network.target edunet-node.service
Requires=edunet-node.service

[Service]
Type=simple
User=YOUR_USERNAME
WorkingDirectory=/home/YOUR_USERNAME/edunet
ExecStart=/home/YOUR_USERNAME/edunet/target/release/edunet-web \
    --port 8080 \
    --node-rpc http://localhost:8545 \
    --database /home/YOUR_USERNAME/edunet/edunet-web.db
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### 6. Start Services

```bash
# Reload systemd
sudo systemctl daemon-reload

# Start services
sudo systemctl start edunet-node
sudo systemctl start edunet-web

# Enable auto-start on boot
sudo systemctl enable edunet-node
sudo systemctl enable edunet-web

# Check status
sudo systemctl status edunet-node
sudo systemctl status edunet-web
```

### 7. Monitor Logs

```bash
# Real-time logs
sudo journalctl -u edunet-node -f
sudo journalctl -u edunet-web -f

# Last 100 lines
sudo journalctl -u edunet-node -n 100
sudo journalctl -u edunet-web -n 100
```

---

## 🌐 Setup Domain & HTTPS (Optional but Recommended)

### 1. Point Domain to Your Server

In your domain registrar (e.g., Namecheap, GoDaddy):
```
A Record:  @  →  YOUR_SERVER_IP
A Record:  www  →  YOUR_SERVER_IP
```

Wait for DNS propagation (5-30 minutes).

### 2. Configure Nginx as Reverse Proxy

```bash
sudo nano /etc/nginx/sites-available/edunet
```

Paste this:
```nginx
server {
    listen 80;
    server_name yourdomain.com www.yourdomain.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

Enable the site:
```bash
sudo ln -s /etc/nginx/sites-available/edunet /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl restart nginx
```

### 3. Get Free SSL Certificate

```bash
sudo certbot --nginx -d yourdomain.com -d www.yourdomain.com
```

Certbot will automatically configure HTTPS! 🎉

---

## 🔧 Service Management Commands

```bash
# Start services
sudo systemctl start edunet-node
sudo systemctl start edunet-web

# Stop services
sudo systemctl stop edunet-node
sudo systemctl stop edunet-web

# Restart services
sudo systemctl restart edunet-node
sudo systemctl restart edunet-web

# Enable auto-start
sudo systemctl enable edunet-node
sudo systemctl enable edunet-web

# Disable auto-start
sudo systemctl disable edunet-node
sudo systemctl disable edunet-web

# Check status
sudo systemctl status edunet-node
sudo systemctl status edunet-web

# View logs
sudo journalctl -u edunet-node -f
sudo journalctl -u edunet-web -f
```

---

## 📊 Accessing Your Services

Once deployed:

- **Web Interface:** http://YOUR_SERVER_IP:8080
- **With Domain:** https://yourdomain.com
- **RPC Endpoint:** http://YOUR_SERVER_IP:8545 (internal use)
- **P2P Network:** YOUR_SERVER_IP:9000 (for friends' nodes)

---

## 👥 Allowing Friends to Join Network

Friends should run:
```bash
export VALIDATOR_ADDRESS=their_wallet_address
export BOOTSTRAP_PEERS=YOUR_SERVER_IP:9000
./run-node.sh
```

Or manually:
```bash
./target/release/blockchain-node \
    --p2p-port 9000 \
    --rpc-port 8545 \
    --validator-address their_wallet \
    --bootstrap-peers YOUR_SERVER_IP:9000 \
    --mining true
```

---

## 🐛 Troubleshooting

### Service won't start
```bash
# Check logs
sudo journalctl -u edunet-node -n 50
sudo journalctl -u edunet-web -n 50

# Check if port is in use
sudo netstat -tlnp | grep 8080
sudo netstat -tlnp | grep 8545
sudo netstat -tlnp | grep 9000
```

### Can't access from outside
```bash
# Check firewall
sudo ufw status

# Test if ports are open
# From another computer:
telnet YOUR_SERVER_IP 8080
telnet YOUR_SERVER_IP 9000
```

### Web client can't connect to node
```bash
# Check if node is running
sudo systemctl status edunet-node

# Check RPC endpoint
curl http://localhost:8545 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"blockchain_getBlockHeight","params":[],"id":1}'
```

### Rebuild after changes
```bash
cd ~/edunet

# Pull latest code (if using git)
git pull

# Rebuild
cargo build --release --bin blockchain-node
cargo build --release --bin edunet-web

# Restart services
sudo systemctl restart edunet-node
sudo systemctl restart edunet-web
```

---

## 💡 Production Tips

1. **Use a Domain:** Much easier than remembering IP addresses
2. **Enable HTTPS:** Essential for security
3. **Monitor Logs:** Set up log rotation and monitoring
4. **Backups:** Regularly backup `blockchain-data/` and `edunet-web.db`
5. **Updates:** Keep server updated (`sudo apt update && sudo apt upgrade`)
6. **Security:** Use SSH keys instead of passwords
7. **Monitoring:** Consider Grafana/Prometheus for metrics

---

## 🎉 Success Checklist

- [ ] Server dependencies installed
- [ ] Project files copied to server
- [ ] C++ core built successfully
- [ ] Rust binaries built successfully
- [ ] Firewall configured
- [ ] Systemd services created
- [ ] Services started and running
- [ ] Web interface accessible
- [ ] P2P network listening
- [ ] Domain configured (optional)
- [ ] HTTPS enabled (optional)

---

## 📞 Quick Reference

| Component | Port | Purpose |
|-----------|------|---------|
| Web Interface | 8080 | HTTP access for users |
| RPC API | 8545 | Blockchain node API |
| P2P Network | 9000 | Peer-to-peer connections |
| SSH | 22 | Server administration |
| HTTPS | 443 | Secure web (with nginx) |

**Your server becomes:**
- A blockchain node (mining & earning rewards)
- A web server (hosting the UI for users)
- A bootstrap peer (for friends' nodes to connect)

All in one! 🚀
