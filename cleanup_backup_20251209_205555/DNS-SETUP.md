# DNS Configuration Guide

## 1. Buy a Domain Name
- Go to Namecheap, GoDaddy, or Cloudflare
- Buy a domain like: edunet-blockchain.com, myedunet.io, etc.

## 2. Point Domain to Your Server
Add these DNS records:

```
Type    Name    Value               TTL
A       @       YOUR_SERVER_IP      300
A       www     YOUR_SERVER_IP      300
CNAME   api     @                   300
```

## 3. SSL Certificate Setup
On your server:

```bash
# Install certbot if not done already
sudo apt install certbot python3-certbot-nginx

# Get SSL certificate
sudo certbot --nginx -d yourdomain.com -d www.yourdomain.com

# Test auto-renewal
sudo certbot renew --dry-run
```

## 4. Update Your Blockchain Config
Edit your nginx config to use your actual domain:

```bash
sudo nano /etc/nginx/sites-available/edunet
# Change "your-domain.com" to your actual domain
sudo systemctl reload nginx
```

## 5. Verify Everything Works
```bash
# Test HTTP redirect to HTTPS
curl -I http://yourdomain.com

# Test blockchain API
curl https://yourdomain.com/api/blockchain/network-status

# Test WebSocket connection
curl -I -N -H "Connection: Upgrade" -H "Upgrade: websocket" https://yourdomain.com/ws
```