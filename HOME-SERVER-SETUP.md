# Router Configuration Guide for Home Server

## 1. Find Your Second Computer's Local IP
On your second computer:
```bash
# Get local IP address
ip addr show | grep "inet 192.168" | awk '{print $2}' | cut -d'/' -f1
# OR
hostname -I | awk '{print $1}'

# Example output: 192.168.1.150
```

## 2. Configure Port Forwarding on Your Router
Access your router admin panel (usually http://192.168.1.1 or http://192.168.0.1):

### Router Settings to Add:
```
Service Name: EduNet Blockchain P2P
Protocol: TCP
External Port: 8333
Internal IP: 192.168.1.150  (your second computer's IP)
Internal Port: 8333
Enable: Yes

Service Name: EduNet Web Interface  
Protocol: TCP
External Port: 8080
Internal IP: 192.168.1.150
Internal Port: 8080
Enable: Yes
```

### Common Router Interfaces:
- **Netgear**: Advanced > Dynamic DNS/Port Forwarding
- **Linksys**: Smart Wi-Fi Tools > Port Range Forwarding
- **TP-Link**: Advanced > NAT Forwarding > Port Forwarding
- **ASUS**: WAN > Virtual Server/Port Forwarding
- **D-Link**: Advanced > Port Forwarding

## 3. Find Your Public IP Address
```bash
curl ifconfig.me
# Example output: 73.45.123.89
```

## 4. Test Port Forwarding
From outside your network (or use online tools):
```bash
# Test if ports are accessible
nmap -p 8333,8080 YOUR_PUBLIC_IP

# Or use online port checker:
# https://www.yougetsignal.com/tools/open-ports/
```

## 5. Dynamic DNS (Recommended)
Since home IP addresses change, set up a free dynamic DNS:

### NoIP.com Setup:
1. Create account at https://www.noip.com/
2. Create hostname: `yourname.ddns.net`
3. Install NoIP client on your server computer:
```bash
cd /usr/local/src/
sudo wget https://www.noip.com/client/linux/noip-duc-linux.tar.gz
sudo tar xf noip-duc-linux.tar.gz
cd noip-2.1.9-1/
sudo make install
sudo /usr/local/bin/noip2 -C  # Configure
sudo /usr/local/bin/noip2     # Start
```

### Alternative: Cloudflare DNS
1. Use Cloudflare as your DNS provider
2. Use cloudflared tunnel for secure access
3. No port forwarding needed!

## 6. Firewall Configuration
On your second computer:
```bash
# Allow blockchain ports
sudo ufw allow 8333/tcp comment "EduNet P2P"
sudo ufw allow 8080/tcp comment "EduNet Web"
sudo ufw enable

# Check status
sudo ufw status
```

## 7. Security Considerations

### ✅ Good Practices:
- Use strong passwords for router admin
- Enable WPA3/WPA2 on WiFi
- Keep router firmware updated
- Monitor who connects to your blockchain
- Use SSH keys instead of passwords

### ⚠️ Things to Consider:
- Your home IP will be visible to blockchain users
- Increased internet traffic on your connection
- Need reliable power and internet connection
- Router restarts may change port forwarding settings

## 8. Testing Your Setup

### From Your Local Network:
```bash
# Test from your main computer
curl http://192.168.1.150:8080
./launch.sh client 192.168.1.150:8333
```

### From the Internet:
```bash
# Test from outside (ask a friend, or use VPN)
curl http://YOUR_PUBLIC_IP:8080
./launch.sh client YOUR_PUBLIC_IP:8333
```

## 9. Monitoring Your Home Server

### System Resources:
```bash
# Check CPU/RAM usage
htop

# Check disk space
df -h

# Check network traffic
sudo nethogs

# Check blockchain logs
tail -f blockchain.log
```

### Uptime Monitoring:
```bash
# Install monitoring tools
sudo apt install uptimed

# Create simple status page
echo "EduNet Status: $(date)" > /var/www/html/status.txt
```

Your home server setup is perfect for:
- Development and testing
- Small friend/family networks
- University projects
- Learning blockchain operations
- Building community before scaling to VPS