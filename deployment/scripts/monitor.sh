#!/bin/bash
set -euo pipefail

# Mailfeed Monitoring Script
# Simple monitoring for single-machine deployment

SERVICE_NAME="mailfeed"
INSTALL_DIR="/opt/mailfeed"
LOG_FILE="/var/log/mailfeed-monitor.log"
HEALTH_URL="http://localhost:8080/health"

# Configuration
MAX_RESPONSE_TIME=5000  # milliseconds
MIN_DISK_SPACE=1000000  # KB (1GB)
MAX_LOG_SIZE=104857600  # bytes (100MB)

log_message() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

check_service_status() {
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        echo "healthy"
    else
        echo "unhealthy"
    fi
}

check_health_endpoint() {
    local response_time
    local http_code
    
    response_time=$(curl -o /dev/null -s -w "%{time_total}" --connect-timeout 5 --max-time 10 "$HEALTH_URL" 2>/dev/null || echo "timeout")
    http_code=$(curl -o /dev/null -s -w "%{http_code}" --connect-timeout 5 --max-time 10 "$HEALTH_URL" 2>/dev/null || echo "000")
    
    if [[ "$http_code" == "200" ]]; then
        local response_ms=$(echo "$response_time * 1000" | bc 2>/dev/null || echo "0")
        if (( $(echo "$response_ms > $MAX_RESPONSE_TIME" | bc -l) )); then
            echo "slow"
        else
            echo "healthy"
        fi
    else
        echo "unhealthy"
    fi
}

check_disk_space() {
    local available_kb
    available_kb=$(df "$INSTALL_DIR" | awk 'NR==2 {print $4}')
    
    if [[ "$available_kb" -gt "$MIN_DISK_SPACE" ]]; then
        echo "healthy"
    else
        echo "low"
    fi
}

check_memory_usage() {
    local mailfeed_memory
    mailfeed_memory=$(ps -o rss= -p $(pgrep -f mailfeed) 2>/dev/null | awk '{sum+=$1} END {print sum}' || echo "0")
    
    # Convert to MB
    local memory_mb=$((mailfeed_memory / 1024))
    
    if [[ "$memory_mb" -gt 500 ]]; then
        echo "high"
    else
        echo "normal"
    fi
}

check_log_size() {
    local log_size
    log_size=$(journalctl -u "$SERVICE_NAME" --output=json | wc -c)
    
    if [[ "$log_size" -gt "$MAX_LOG_SIZE" ]]; then
        echo "large"
    else
        echo "normal"
    fi
}

check_database() {
    if [[ -f "$INSTALL_DIR/data/mailfeed.db" ]]; then
        # Simple check - ensure database file is readable
        if sqlite3 "$INSTALL_DIR/data/mailfeed.db" "SELECT 1;" >/dev/null 2>&1; then
            echo "healthy"
        else
            echo "corrupted"
        fi
    else
        echo "missing"
    fi
}

restart_service() {
    log_message "🔄 Restarting $SERVICE_NAME service..."
    systemctl restart "$SERVICE_NAME"
    sleep 5
    
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        log_message "✅ Service restarted successfully"
        return 0
    else
        log_message "❌ Service restart failed"
        return 1
    fi
}

send_alert() {
    local message="$1"
    log_message "🚨 ALERT: $message"
    
    # If telegram bot token is configured, send alert
    if [[ -n "${TELEGRAM_BOT_TOKEN:-}" ]] && [[ -n "${TELEGRAM_CHAT_ID:-}" ]]; then
        curl -s -X POST "https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/sendMessage" \
            -d chat_id="$TELEGRAM_CHAT_ID" \
            -d text="🚨 Mailfeed Alert: $message" >/dev/null || true
    fi
}

main() {
    local issues=0
    local critical_issues=0
    
    log_message "🔍 Starting health check..."
    
    # Check service status
    local service_status=$(check_service_status)
    if [[ "$service_status" != "healthy" ]]; then
        send_alert "Service is not running"
        restart_service
        ((critical_issues++))
    fi
    
    # Check health endpoint
    local health_status=$(check_health_endpoint)
    if [[ "$health_status" == "unhealthy" ]]; then
        send_alert "Health endpoint not responding"
        ((critical_issues++))
    elif [[ "$health_status" == "slow" ]]; then
        log_message "⚠️  Health endpoint responding slowly"
        ((issues++))
    fi
    
    # Check disk space
    local disk_status=$(check_disk_space)
    if [[ "$disk_status" == "low" ]]; then
        send_alert "Low disk space available"
        ((issues++))
    fi
    
    # Check memory usage
    local memory_status=$(check_memory_usage)
    if [[ "$memory_status" == "high" ]]; then
        log_message "⚠️  High memory usage detected"
        ((issues++))
    fi
    
    # Check database
    local db_status=$(check_database)
    if [[ "$db_status" != "healthy" ]]; then
        send_alert "Database issue detected: $db_status"
        ((critical_issues++))
    fi
    
    # Summary
    if [[ $critical_issues -eq 0 ]] && [[ $issues -eq 0 ]]; then
        log_message "✅ All checks passed"
    else
        log_message "⚠️  Found $issues warnings and $critical_issues critical issues"
    fi
    
    # Clean up old logs
    if [[ -f "$LOG_FILE" ]] && [[ $(stat -f%z "$LOG_FILE" 2>/dev/null || stat -c%s "$LOG_FILE") -gt $MAX_LOG_SIZE ]]; then
        tail -n 1000 "$LOG_FILE" > "$LOG_FILE.tmp"
        mv "$LOG_FILE.tmp" "$LOG_FILE"
        log_message "📝 Rotated monitor log file"
    fi
}

# Run monitoring check
main "$@"