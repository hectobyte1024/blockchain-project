# 🏠 Router Port Forwarding - Simple Guide

## **Quick Setup for Most Home Routers**

### 1. **Find Your Router's Admin Page**
```
1. Open a web browser
2. Go to one of these addresses:
   • http://192.168.1.1 
   • http://192.168.0.1
   • http://10.0.0.1
3. Login (usually admin/admin or admin/password)
```

### 2. **Add Port Forwarding Rules**

Look for **"Port Forwarding"** or **"Virtual Server"** in your router settings.

**Add these two rules:**

**Rule 1 - Blockchain P2P:**
```
Service Name: EduNet-P2P
External Port: 8333
Internal IP: [Your server's local IP]
Internal Port: 8333
Protocol: TCP
```

**Rule 2 - Web Interface:**
```
Service Name: EduNet-Web
External Port: 8080
Internal IP: [Your server's local IP]
Internal Port: 8080
Protocol: TCP
```

### 3. **Common Router Brands**

**Netgear:**
- Advanced → Dynamic DNS/Port Forwarding → Port Forwarding

**Linksys:**
- Smart Wi-Fi → Security → Apps and Gaming → Single Port Forwarding

**TP-Link:**
- Network → NAT Forwarding → Virtual Servers

**ASUS:**
- Adaptive QoS → Traditional QoS → Port Forwarding

**D-Link:**
- Advanced → Firewall → Port Forwarding

### 4. **Test Your Setup**

**From inside your network:**
```bash
curl http://[local-ip]:8080
```

**From outside (use your phone's data):**
```
Visit: http://[your-public-ip]:8080
```

### 5. **Get Your Public IP**
```bash
curl ifconfig.me
```

### 6. **Troubleshooting**

**Can't access router admin?**
- Try `ipconfig` (Windows) or `ip route` (Linux) to find gateway
- Reset router with paperclip if needed

**Port forwarding not working?**
- Restart router after adding rules
- Check if router has "UPnP" enabled
- Make sure firewall allows the ports
- Some ISPs block certain ports

**Still having issues?**
- Try different external ports (like 8334 instead of 8333)
- Check if your ISP provides a static IP
- Consider using a VPN service with port forwarding

---

## **💡 Pro Tips**

1. **Static Local IP:** Set your server to use a static IP address so the port forwarding doesn't break
2. **Dynamic DNS:** Use services like NoIP.com for a friendly domain name instead of IP addresses
3. **Security:** Only forward the ports you need (8333 and 8080 for blockchain)
4. **Backup:** Take a screenshot of your router settings before making changes

## **🎯 Success Indicators**

✅ People can visit `http://your-public-ip:8080` and see your blockchain interface  
✅ Other nodes can connect to your P2P network on port 8333  
✅ Your blockchain shows up as a bootstrap server for the network  

**Your blockchain is now live on the internet! 🌍**