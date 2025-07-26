#!/bin/bash
set -euo pipefail

# Mailfeed Installation Script
# For standalone binary deployment on a single machine

INSTALL_DIR="/opt/mailfeed"
SERVICE_USER="mailfeed"
SERVICE_NAME="mailfeed"

echo "🚀 Installing Mailfeed..."

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   echo "❌ This script must be run as root (use sudo)"
   exit 1
fi

# Create service user
if ! id "$SERVICE_USER" &>/dev/null; then
    echo "👤 Creating service user: $SERVICE_USER"
    useradd --system --home-dir "$INSTALL_DIR" --shell /bin/false "$SERVICE_USER"
fi

# Create directory structure
echo "📁 Setting up directory structure..."
mkdir -p "$INSTALL_DIR"/{bin,data,logs,public,config}
chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR"

# Copy binary (assumes it's built and available)
if [[ -f "./target/release/mailfeed" ]]; then
    echo "📦 Installing binary..."
    cp ./target/release/mailfeed "$INSTALL_DIR/bin/"
    chmod +x "$INSTALL_DIR/bin/mailfeed"
    chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/bin/mailfeed"
else
    echo "❌ Binary not found. Please run 'cargo build --release' first."
    exit 1
fi

# Copy frontend files (if they exist)
if [[ -d "./mailfeed-ui/build" ]]; then
    echo "🎨 Installing frontend files..."
    cp -r ./mailfeed-ui/build/* "$INSTALL_DIR/public/"
    chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/public"
fi

# Install systemd service
echo "🔧 Installing systemd service..."
cp ./deployment/systemd/mailfeed.service /etc/systemd/system/
systemctl daemon-reload

# Create environment file
echo "⚙️  Creating environment configuration..."
cat > "$INSTALL_DIR/config/environment" << EOF
# Mailfeed Configuration
MF_DATABASE_URL=$INSTALL_DIR/data/mailfeed.db
MF_PUBLIC_PATH=$INSTALL_DIR/public
MF_PORT=8080

# Logging
LOG_LEVEL=info
LOG_FORMAT=json
RUST_LOG=mailfeed=info

# Security (set these to your actual values)
# JWT_SECRET=your-secret-here
# SMTP_HOST=your-smtp-server
# SMTP_USERNAME=your-email@domain.com
# SMTP_PASSWORD=your-password

# Optional: Telegram Bot
# TELEGRAM_BOT_TOKEN=your-bot-token
EOF

chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/config/environment"
chmod 600 "$INSTALL_DIR/config/environment"

# Create basic nginx config (optional)
if command -v nginx &> /dev/null; then
    echo "🌐 Creating nginx configuration..."
    cat > /etc/nginx/sites-available/mailfeed << EOF
server {
    listen 80;
    server_name mailfeed.yourdomain.com;  # Change this
    
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
    
    location /health {
        proxy_pass http://127.0.0.1:8080/health;
        access_log off;
    }
}
EOF
    echo "ℹ️  Nginx config created at /etc/nginx/sites-available/mailfeed"
    echo "   Enable with: ln -s /etc/nginx/sites-available/mailfeed /etc/nginx/sites-enabled/"
fi

# Create logrotate config
echo "📝 Setting up log rotation..."
cat > /etc/logrotate.d/mailfeed << EOF
/opt/mailfeed/logs/*.log {
    daily
    missingok
    rotate 14
    compress
    delaycompress
    notifempty
    create 0644 mailfeed mailfeed
    postrotate
        systemctl reload mailfeed || true
    endrotate
}
EOF

# Initialize database and create admin user
echo "🗄️  Initializing database..."
sudo -u "$SERVICE_USER" "$INSTALL_DIR/bin/mailfeed" --create-admin || true

# Enable and start service
echo "🔄 Enabling and starting service..."
systemctl enable "$SERVICE_NAME"
systemctl start "$SERVICE_NAME"

# Show status
echo ""
echo "✅ Installation complete!"
echo ""
echo "📊 Service status:"
systemctl status "$SERVICE_NAME" --no-pager -l
echo ""
echo "📋 Next steps:"
echo "   1. Edit configuration: $INSTALL_DIR/config/environment"
echo "   2. Restart service: sudo systemctl restart mailfeed"
echo "   3. View logs: sudo journalctl -u mailfeed -f"
echo "   4. Health check: curl http://localhost:8080/health"
echo ""
echo "🔗 Access your application at: http://your-server:8080"