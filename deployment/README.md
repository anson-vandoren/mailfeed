# Mailfeed Deployment Guide

This directory contains deployment configuration for Mailfeed as a standalone binary on a single machine (1-10 users).

## Quick Start

1. **Build the application:**
   ```bash
   # Build the complete application (backend + HTMX templates + static assets)
   cargo build --release
   ```

2. **Install:**
   ```bash
   sudo ./deployment/install.sh
   ```

3. **Configure:**
   ```bash
   sudo nano /opt/mailfeed/config/environment
   # Set SMTP settings, secrets, etc.
   sudo systemctl restart mailfeed
   ```

## Files Overview

### Installation & Updates
- `install.sh` - Initial installation script
- `update.sh` - Update binary and restart service
- `systemd/mailfeed.service` - Systemd service configuration

### Operational Scripts
- `scripts/backup.sh` - Database and configuration backup
- `scripts/monitor.sh` - Health monitoring and alerting
- `scripts/maintenance.sh` - Routine maintenance tasks

### Automation
- `cron/mailfeed-cron` - Automated monitoring and maintenance

## Installation Details

The installation script creates:
- Service user: `mailfeed`
- Install directory: `/opt/mailfeed/`
  - `bin/` - Application binary
  - `data/` - SQLite database
  - `config/` - Configuration files
  - `public/` - Static assets (favicon, fonts)
  - `logs/` - Application logs
- Systemd service: `mailfeed.service`
- Log rotation: `/etc/logrotate.d/mailfeed`
- Optional nginx configuration

## Configuration

### Environment Variables

Edit `/opt/mailfeed/config/environment`:

```bash
# Database
MF_DATABASE_URL=/opt/mailfeed/data/mailfeed.db
MF_PUBLIC_PATH=/opt/mailfeed/public
MF_PORT=8080

# Logging
LOG_LEVEL=info
LOG_FORMAT=json
RUST_LOG=mailfeed=info

# SMTP (Required for email sending)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_NAME=Mailfeed
SMTP_FROM_EMAIL=your-email@gmail.com

# Security
SESSION_SECRET=generate-a-secure-random-string

# Optional: Telegram alerts
TELEGRAM_BOT_TOKEN=your-bot-token
TELEGRAM_CHAT_ID=your-chat-id
```

### Security Notes

- Service runs as dedicated `mailfeed` user
- Restricted filesystem access via systemd
- Configuration file has 600 permissions
- No network capabilities beyond HTTP server

## Service Management

```bash
# Service status
sudo systemctl status mailfeed

# Start/stop/restart
sudo systemctl start mailfeed
sudo systemctl stop mailfeed
sudo systemctl restart mailfeed

# View logs
sudo journalctl -u mailfeed -f

# Health check
curl http://localhost:8080/health
```

## Operational Scripts

### Backup
```bash
# Manual backup
sudo /opt/mailfeed/deployment/scripts/backup.sh

# Backups stored in: /opt/mailfeed-backups/
```

### Monitoring
```bash
# Manual health check
sudo /opt/mailfeed/deployment/scripts/monitor.sh

# Configure alerts via TELEGRAM_BOT_TOKEN in environment
```

### Maintenance
```bash
# Full maintenance
sudo /opt/mailfeed/deployment/scripts/maintenance.sh

# Specific tasks
sudo /opt/mailfeed/deployment/scripts/maintenance.sh --sessions
sudo /opt/mailfeed/deployment/scripts/maintenance.sh --database
sudo /opt/mailfeed/deployment/scripts/maintenance.sh --report
```

### Updates
```bash
# Build new version
cargo build --release

# Deploy update
sudo /opt/mailfeed/deployment/scripts/update.sh
```

## Automation Setup

Install automated monitoring and maintenance:

```bash
# Install cron jobs
sudo cp deployment/cron/mailfeed-cron /etc/cron.d/
sudo systemctl restart cron
```

This sets up:
- Health monitoring every 5 minutes
- Daily maintenance at 2 AM
- Weekly backups on Sundays at 3 AM
- Hourly session cleanup

## Troubleshooting

### Service Won't Start
```bash
# Check logs
sudo journalctl -u mailfeed -n 50

# Check configuration
sudo -u mailfeed /opt/mailfeed/bin/mailfeed --help

# Verify permissions
ls -la /opt/mailfeed/
```

### Database Issues
```bash
# Check database
sudo -u mailfeed sqlite3 /opt/mailfeed/data/mailfeed.db ".tables"

# Repair database
sudo systemctl stop mailfeed
sudo -u mailfeed sqlite3 /opt/mailfeed/data/mailfeed.db "PRAGMA integrity_check;"
sudo systemctl start mailfeed
```

### High Memory Usage
```bash
# Check memory
ps aux | grep mailfeed

# Check for memory leaks in logs
sudo journalctl -u mailfeed | grep -i memory

# Restart service
sudo systemctl restart mailfeed
```

### Disk Space Issues
```bash
# Check space
df -h /opt/mailfeed

# Clean up old backups
sudo /opt/mailfeed/deployment/scripts/maintenance.sh --backups

# Clean up old feed items
sudo /opt/mailfeed/deployment/scripts/maintenance.sh --feed-items
```

## Nginx Setup (Optional)

If you installed nginx during setup:

```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/mailfeed /etc/nginx/sites-enabled/

# Edit domain name
sudo nano /etc/nginx/sites-available/mailfeed

# Test and reload
sudo nginx -t
sudo systemctl reload nginx
```

## Security Considerations

- Change default ports if exposed to internet
- Use HTTPS in production (Let's Encrypt recommended)
- Regularly update system packages
- Monitor logs for suspicious activity
- Keep backups in secure location
- Use strong SMTP credentials

## Performance Tuning

For higher loads (approaching 10 users):
- Consider connection pooling tuning
- Monitor database performance
- Implement feed update intervals based on usage
- Consider nginx caching for static assets

## Migration from Development

If migrating from development setup:
1. Export development database
2. Run installation script
3. Import database to `/opt/mailfeed/data/mailfeed.db`
4. Update configuration
5. Restart service