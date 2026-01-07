# EduNet Blockchain - Home Hosting Guide

## Complete Guide to Hosting Your Blockchain from Home

This guide will help you deploy your EduNet blockchain to run 24/7 from your home internet connection.

---

## 📋 Prerequisites

### Hardware Requirements
- **Minimum:**
  - CPU: 2 cores
  - RAM: 2GB
  - Storage: 20GB SSD
  - Network: 10 Mbps upload

- **Recommended:**
  - CPU: 4+ cores
  - RAM: 4GB+
  - Storage: 50GB+ SSD
  - Network: 25+ Mbps upload

### Software Requirements
- Debian/Ubuntu Linux (20.04+ or 11+)
- Rust toolchain installed
- Git installed
- Sudo/root access

### ISP Requirements
✅ **Will Work:**
- Most residential ISPs (Comcast, Spectrum, AT&T, Verizon, etc.)
- Static or dynamic IP (dynamic works with DuckDNS)
- No port 80/443 blocking

⚠️ **May Not Work:**
- CGNAT (Carrier-Grade NAT) - contact ISP to disable
- Some mobile hotspots
- Networks with strict firewall policies

---

## 🚀 Quick Deployment (30 minutes)

### Automated Setup

1. **Run the deployment script:**
```bash
cd ~/Documents/blockchain\ project
sudo bash deploy-home-server.sh
```

2. **Follow the prompts:**
   - Enter your domain name (or press Enter for IP-only)
   - Wait for installation to complete

3. **Configure your router** (see Router Setup below)

4. **Access your blockchain:**
   - Local: `http://YOUR_LOCAL_IP`
   - Internet: `https://YOUR_DOMAIN.com`

---

## 🌐 Router Configuration

### Port Forwarding Setup

**You MUST forward these ports from your router to your server:**

| Service | Port | Protocol | Forward To |
|---------|------|----------|------------|
| HTTP    | 80   | TCP      | Server's Local IP |
| HTTPS   | 443  | TCP      | Server's Local IP |
| P2P (optional) | 8333 | TCP | Server's Local IP |

### Step-by-Step for Common Routers:

#### **Generic Steps (works for most routers):**

1. **Find your router's IP:**
```bash
ip route | grep default
# Usually 192.168.1.1 or 192.168.0.1
```

2. **Access router admin:**
   - Open browser to `http://192.168.1.1` (or your router IP)
   - Login (common defaults: admin/admin, admin/password)

3. **Navigate to Port Forwarding:**
   - Look for: "Port Forwarding", "NAT", "Virtual Server", or "Applications"

4. **Add forwarding rules:**
   - **Rule 1:**
     - Service Name: EduNet-HTTP
     - External Port: 80
     - Internal Port: 80
     - Internal IP: `YOUR_SERVER_LOCAL_IP` (e.g., 192.168.1.100)
     - Protocol: TCP
   
   - **Rule 2:**
     - Service Name: EduNet-HTTPS
     - External Port: 443
     - Internal Port: 443
     - Internal IP: `YOUR_SERVER_LOCAL_IP`
     - Protocol: TCP

5. **Save and Reboot Router**

#### **Specific Router Instructions:**

**Netgear:**
- Advanced → Port Forwarding / Port Triggering
- Add Custom Service

**TP-Link:**
- Forwarding → Virtual Servers
- Add New

**Linksys:**
- Security → Apps and Gaming
- Port Range Forward

**ASUS:**
- WAN → Virtual Server/Port Forwarding
- Add Profile

---

## 🔐 Dynamic DNS Setup (DuckDNS)

If your ISP changes your IP address (most do), use DuckDNS for free:

### 1. Create Account
- Go to https://www.duckdns.org
- Sign in with GitHub, Google, or Reddit
- No payment required

### 2. Create Subdomain
- Choose a subdomain: `myedunet.duckdns.org`
- Click "add domain"
- Note your token (long string of characters)

### 3. Install Update Client
```bash
mkdir -p ~/duckdns
cd ~/duckdns

# Create update script (replace YOUR_DOMAIN and YOUR_TOKEN)
cat > duck.sh <<'EOF'
echo url="https://www.duckdns.org/update?domains=YOUR_DOMAIN&token=YOUR_TOKEN&ip=" | curl -k -o ~/duckdns/duck.log -K -
EOF

chmod 700 duck.sh
```

### 4. Test It
```bash
./duck.sh
cat duck.log  # Should say "OK"
```

### 5. Automate Updates
```bash
crontab -e
# Add this line (updates every 5 minutes):
*/5 * * * * ~/duckdns/duck.sh >/dev/null 2>&1
```

### 6. Update Caddy Configuration
```bash
sudo nano /etc/caddy/Caddyfile
```

Change the domain to your DuckDNS domain:
```
myedunet.duckdns.org {
    reverse_proxy localhost:8080
    # ... rest of config
}
```

Restart Caddy:
```bash
sudo systemctl restart caddy
```

---

## 🔒 SSL/HTTPS Certificate

### Automatic (with Domain)
Caddy automatically gets Let's Encrypt certificates! Just:
1. Point your domain to your public IP
2. Wait 2-3 minutes
3. Access `https://YOUR_DOMAIN.com` ✅

### Manual (without Domain)
If using IP-only, you can use a self-signed certificate:
```bash
caddy trust  # Make Caddy a trusted CA
```

But browsers will still show warnings. **Use a domain for production!**

---

## 📊 Service Management

### Start/Stop/Restart
```bash
# Start blockchain
sudo systemctl start edunet-blockchain

# Stop blockchain
sudo systemctl stop edunet-blockchain

# Restart blockchain
sudo systemctl restart edunet-blockchain

# Check status
sudo systemctl status edunet-blockchain

# View live logs
sudo journalctl -u edunet-blockchain -f
```

### Caddy (Web Server)
```bash
# Restart Caddy
sudo systemctl restart caddy

# Check Caddy status
sudo systemctl status caddy

# Test configuration
sudo caddy validate --config /etc/caddy/Caddyfile
```

---

## 💾 Database Backups

### Manual Backup
```bash
# Backup now
cp ~/Documents/blockchain\ project/edunet-gui/edunet.db \
   ~/edunet-backup-$(date +%Y%m%d-%H%M%S).db
```

### Automated Daily Backups
```bash
# Create backup script
mkdir -p ~/backups
cat > ~/backups/backup-edunet.sh <<'EOF'
#!/bin/bash
BACKUP_DIR=~/backups
PROJECT_DIR=~/Documents/blockchain\ project
DATE=$(date +%Y%m%d-%H%M%S)

# Backup database
cp "$PROJECT_DIR/edunet-gui/edunet.db" "$BACKUP_DIR/edunet-$DATE.db"

# Keep only last 7 days
find $BACKUP_DIR -name "edunet-*.db" -mtime +7 -delete

echo "Backup completed: edunet-$DATE.db"
EOF

chmod +x ~/backups/backup-edunet.sh

# Schedule daily at 2 AM
crontab -e
# Add: 0 2 * * * ~/backups/backup-edunet.sh
```

### Restore from Backup
```bash
# Stop blockchain
sudo systemctl stop edunet-blockchain

# Restore database
cp ~/backups/edunet-YYYYMMDD-HHMMSS.db \
   ~/Documents/blockchain\ project/edunet-gui/edunet.db

# Restart blockchain
sudo systemctl start edunet-blockchain
```

---

## 🔍 Troubleshooting

### Can't Access from Internet

**1. Check if services are running:**
```bash
sudo systemctl status edunet-blockchain
sudo systemctl status caddy
```

**2. Check firewall:**
```bash
sudo ufw status
# Should show 80/tcp and 443/tcp as ALLOW
```

**3. Check port forwarding:**
```bash
# From another device on same network:
curl http://YOUR_LOCAL_IP

# From internet (use phone data):
curl http://YOUR_PUBLIC_IP
```

**4. Find your public IP:**
```bash
curl ifconfig.me
```

**5. Test port accessibility:**
- Go to https://www.yougetsignal.com/tools/open-ports/
- Enter your public IP and port 80
- Should say "Open"

### CGNAT Issue (No Public IP)

Some ISPs use CGNAT, which prevents port forwarding. Check:
```bash
# Your public IP as seen by internet
curl ifconfig.me

# Your router's WAN IP (login to router and check)
```

If these don't match, you have CGNAT. Solutions:
1. **Contact ISP:** Request a public IP (may cost $5-10/month)
2. **Use VPN:** Set up WireGuard or Tailscale tunnel
3. **Use Cloudflare Tunnel:** Free, but more complex

### SSL Certificate Issues

**Certificate not issuing:**
```bash
# Check Caddy logs
sudo journalctl -u caddy -n 100

# Common issues:
# - Domain not pointing to your IP (check DNS)
# - Port 80/443 not forwarded
# - Firewall blocking Let's Encrypt validation
```

**Force certificate renewal:**
```bash
sudo caddy reload --config /etc/caddy/Caddyfile
```

### Blockchain Won't Start

**Check logs:**
```bash
sudo journalctl -u edunet-blockchain -n 100 --no-pager
```

**Common issues:**
- Database file permissions
- Port 8080 already in use
- Missing dependencies

**Fix permissions:**
```bash
cd ~/Documents/blockchain\ project
sudo chown -R $USER:$USER edunet-gui/
```

### High RAM/CPU Usage

**Monitor resources:**
```bash
htop
# or
top
```

**Limit resource usage:**
Edit `/etc/systemd/system/edunet-blockchain.service`:
```ini
[Service]
MemoryLimit=1G
CPUQuota=50%
```

Then reload:
```bash
sudo systemctl daemon-reload
sudo systemctl restart edunet-blockchain
```

---

## 📈 Monitoring

### View Live Activity
```bash
# Real-time blockchain logs
sudo journalctl -u edunet-blockchain -f

# Real-time web access logs
sudo tail -f /var/log/caddy/edunet-access.log

# Server resources
htop
```

### Check Blockchain Status
```bash
# From command line
curl http://localhost:8080/api/blockchain/network-status | jq

# Expected output:
{
  "block_height": 123,
  "blockchain_type": "PRODUCTION_REAL_ECDSA",
  "node_type": "CLIENT",
  ...
}
```

---

## 🛡️ Security Best Practices

### 1. Change Default Passwords
```bash
# Access dashboard
http://YOUR_IP/dashboard

# Create new accounts with strong passwords
# Delete or disable demo accounts (alice, bob, carol)
```

### 2. Keep System Updated
```bash
# Update OS weekly
sudo apt update && sudo apt upgrade -y

# Update EduNet (when new version available)
cd ~/Documents/blockchain\ project
git pull
cargo build --release --manifest-path edunet-gui/Cargo.toml
sudo systemctl restart edunet-blockchain
```

### 3. Enable Fail2Ban (optional)
```bash
# Install fail2ban
sudo apt install fail2ban

# Protects against brute-force attacks
sudo systemctl enable fail2ban
sudo systemctl start fail2ban
```

### 4. Regular Backups
- Enable automated backups (see Backups section)
- Test restore procedure monthly
- Keep off-site backup (external drive, cloud)

---

## 🌍 Public Access Checklist

Before sharing your blockchain publicly:

- [ ] Port forwarding configured (80, 443)
- [ ] Domain name pointing to your IP
- [ ] HTTPS certificate issued (green lock in browser)
- [ ] Demo accounts disabled/password changed
- [ ] Firewall enabled (`sudo ufw status`)
- [ ] Backups automated and tested
- [ ] Monitoring set up
- [ ] Router has strong password
- [ ] Server has strong password
- [ ] SSH key authentication enabled (disable password login)

---

## 📞 Support Resources

### Useful Commands Reference
```bash
# Service status
sudo systemctl status edunet-blockchain caddy

# View all logs
sudo journalctl -xe

# Disk space
df -h

# Network connections
ss -tulpn | grep LISTEN

# Test local access
curl http://localhost:8080/api/blockchain/network-status

# Test external access (from another device)
curl http://YOUR_PUBLIC_IP/api/blockchain/network-status
```

### Common URLs
- **Local Dashboard:** `http://localhost:8080/dashboard`
- **Blockchain Explorer:** `http://localhost:8080/explorer`
- **Network Status API:** `http://localhost:8080/api/blockchain/network-status`
- **Marketplace:** `http://localhost:8080/marketplace`

---

## 🎉 Success!

Your EduNet blockchain is now:
- ✅ Accessible worldwide
- ✅ Secured with HTTPS
- ✅ Running 24/7 automatically
- ✅ Backed up regularly
- ✅ Production-ready

**Share your blockchain:**
`https://YOUR_DOMAIN.com`

Welcome to the decentralized future! 🚀
