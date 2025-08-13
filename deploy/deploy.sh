#!/bin/bash

# Shield Deployment Script
# This script automates the deployment of Shield service to a target Raspberry Pi
# Builds natively on ARM Ubuntu and deploys via SSH

set -e

# Configuration
PI_HOST=""
SERVICE_NAME="shield-service"
INSTALL_DIR="/var/lib/shield"
BIN_DIR="/usr/bin"
SYSTEMD_DIR="/etc/systemd/system"
WEB_DIR="/var/www/shield"
NGINX_DIR="/etc/nginx/sites-available"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS] <pi_host>"
    echo ""
    echo "Options:"
    echo "  -h, --help         Show this help message"
    echo "  --skip-build       Skip the build step (use existing binaries)"
    echo ""
    echo "Examples:"
    echo "  $0 pi@192.168.1.100"
    echo "  $0 --skip-build pi@192.168.1.100"
    echo ""
    echo "Note: This script builds natively on ARM Ubuntu and deploys to target Pi"
}

# Variables
SKIP_BUILD=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        -*)
            print_error "Unknown option $1"
            show_usage
            exit 1
            ;;
        *)
            if [[ -z "$PI_HOST" ]]; then
                PI_HOST="$1"
            else
                print_error "Multiple hosts specified"
                show_usage
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate arguments
if [[ -z "$PI_HOST" ]]; then
    print_error "Target Raspberry Pi host not specified"
    show_usage
    exit 1
fi

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]] || [[ ! -d "service" ]]; then
    print_error "Please run this script from the shield project root directory"
    exit 1
fi

# Build the project (unless skipped)
if [[ "$SKIP_BUILD" == false ]]; then
    print_step "Building shield-service..."
    cargo build --release -p shield-service

    # Check if binary was created
    BINARY_PATH="target/release/$SERVICE_NAME"
    if [[ ! -f "$BINARY_PATH" ]]; then
        print_error "Binary not found at $BINARY_PATH"
        exit 1
    fi

    print_status "Binary built successfully: $BINARY_PATH"

    # Build the web application
    print_step "Building web application..."

    # Check if dioxus-cli is installed
    if ! command -v dx &> /dev/null; then
        print_error "Dioxus CLI (dx) not found. Please install it first:"
        print_error "cargo install dioxus-cli"
        exit 1
    fi

    dx bundle --release -p shield-app

    # Check if web build was created
    WEB_BUILD_PATH="app/dist"
    if [[ ! -d "$WEB_BUILD_PATH" ]]; then
        print_error "Web build not found at $WEB_BUILD_PATH"
        exit 1
    fi

    print_status "Web application built successfully: $WEB_BUILD_PATH"
else
    print_status "Skipping build step"
    BINARY_PATH="target/release/$SERVICE_NAME"
    WEB_BUILD_PATH="app/dist"

    if [[ ! -f "$BINARY_PATH" ]]; then
        print_error "Binary not found at $BINARY_PATH. Run without --skip-build first."
        exit 1
    fi

    if [[ ! -d "$WEB_BUILD_PATH" ]]; then
        print_error "Web build not found at $WEB_BUILD_PATH. Run without --skip-build first."
        exit 1
    fi
fi

# Copy files to target Raspberry Pi
print_step "Copying files to $PI_HOST..."

# Test SSH connection
if ! ssh -o ConnectTimeout=10 -o BatchMode=yes "$PI_HOST" exit 2>/dev/null; then
    print_error "Cannot connect to $PI_HOST via SSH"
    print_warning "Make sure SSH key authentication is set up"
    exit 1
fi

# Copy binary
print_status "Copying binary..."
scp "$BINARY_PATH" "$PI_HOST:~/"

# Copy web application
print_status "Copying web application..."
scp -r "$WEB_BUILD_PATH" "$PI_HOST:~/shield-web"

# Handle configuration file
print_status "Checking configuration..."
ssh "$PI_HOST" << 'EOF'
if [[ -f ~/shield.config.toml ]]; then
    echo "Existing configuration found, keeping current config"
    touch ~/keep_existing_config
fi
EOF

# Copy template config if no existing config on target
if ssh "$PI_HOST" "[[ ! -f ~/keep_existing_config ]]"; then
    if [[ -f "deploy/shield.config.toml.template" ]]; then
        print_status "Copying configuration template..."
        scp deploy/shield.config.toml.template "$PI_HOST:~/shield.config.toml"

        echo "Configuration template installed - please edit ~/shield.config.toml"
        echo "Update the username and password fields before starting the service"
    else
        print_warning "Configuration template not found"
    fi
else
    print_status "Keeping existing configuration on target"
fi

# Copy systemd service file
print_status "Copying systemd service file..."
scp deploy/shield.service "$PI_HOST:~/"

# Copy nginx configuration
print_status "Copying nginx configuration..."
scp deploy/shield.conf "$PI_HOST:~/"

# Install on target Raspberry Pi
print_step "Installing on target Raspberry Pi..."

ssh "$PI_HOST" << EOF
set -e

echo "Creating application directory..."
sudo mkdir -p $INSTALL_DIR
sudo chown \$USER:\$USER $INSTALL_DIR

echo "Installing binary..."
sudo mv ~/$SERVICE_NAME $BIN_DIR/$SERVICE_NAME
sudo chmod +x $BIN_DIR/$SERVICE_NAME

echo "Checking nginx installation..."
if ! command -v nginx &> /dev/null; then
    echo "Installing nginx..."
    sudo apt update
    sudo apt install -y nginx
    sudo systemctl enable nginx
    sudo systemctl start nginx
else
    echo "Nginx already installed"
fi

echo "Installing web application..."
sudo mkdir -p $WEB_DIR
sudo cp -r ~/shield-web/* $WEB_DIR/
sudo chown -R www-data:www-data $WEB_DIR

echo "Installing nginx configuration..."
sudo mv ~/shield.conf $NGINX_DIR/
sudo ln -sf $NGINX_DIR/shield.conf /etc/nginx/sites-enabled/

# Remove default nginx site if it exists to avoid conflicts
if [[ -f /etc/nginx/sites-enabled/default ]]; then
    sudo rm /etc/nginx/sites-enabled/default
    echo "Removed default nginx site"
fi

# Test nginx configuration before reloading
if sudo nginx -t; then
    sudo systemctl reload nginx
    echo "Nginx configuration updated successfully"
else
    echo "ERROR: Nginx configuration test failed - rolling back"
    sudo rm -f /etc/nginx/sites-enabled/shield.conf
    sudo nginx -t && sudo systemctl reload nginx
    exit 1
fi

echo "Installing systemd service..."
sudo mv ~/shield.service $SYSTEMD_DIR/
sudo systemctl daemon-reload

echo "Enabling and managing service..."
sudo systemctl enable shield.service

# Check if service is already running
if sudo systemctl is-active --quiet shield.service; then
    echo "Restarting existing service..."
    sudo systemctl restart shield.service
else
    echo "Starting new service..."
    sudo systemctl start shield.service
fi

# Wait a moment for service to start
sleep 2

echo "Cleaning up temporary files..."
rm -f ~/keep_existing_config
rm -rf ~/shield-web

echo "Installation completed!"
EOF

# Verify installation
print_step "Verifying installation..."
ssh "$PI_HOST" << 'EOF'
echo "Service status:"
if sudo systemctl is-active --quiet shield.service; then
    echo "✓ Service is running"
else
    echo "✗ Service is not running"
fi

sudo systemctl status shield.service --no-pager -l | head -10

echo ""
echo "Recent logs:"
sudo journalctl -u shield.service -n 5 --no-pager

echo ""
echo "Nginx status:"
if sudo systemctl is-active --quiet nginx; then
    echo "✓ Nginx is running"
else
    echo "✗ Nginx is not running"
fi

# Check if service is listening on any ports
echo ""
echo "Network status:"
if command -v netstat &> /dev/null; then
    netstat_output=$(sudo netstat -tlnp 2>/dev/null | grep shield-service || true)
    if [[ -n "$netstat_output" ]]; then
        echo "✓ Service is listening on ports:"
        echo "$netstat_output"
    else
        echo "ℹ Service not currently listening on any ports (may be normal)"
    fi
else
    echo "ℹ netstat not available for port checking"
fi
EOF

print_status ""
print_status "Deployment completed successfully!"
print_status "Service is running on $PI_HOST"
print_status ""

# Check if this was a new config deployment
if ssh "$PI_HOST" "[[ -f ~/shield.config.toml ]] && grep -q 'YOUR_UNIFI_USERNAME' ~/shield.config.toml 2>/dev/null"; then
    print_warning "IMPORTANT: Configuration requires setup!"
    print_warning "Edit ~/shield.config.toml"
    print_warning "Update the username and password fields with your UniFi credentials"
    print_warning "Then restart the service: sudo systemctl restart shield.service"
    print_status ""
fi

print_status "Useful commands:"
print_status "  Check service status: ssh $PI_HOST 'sudo systemctl status shield.service'"
print_status "  Check nginx status:   ssh $PI_HOST 'sudo systemctl status nginx'"
print_status "  View logs:            ssh $PI_HOST 'sudo journalctl -u shield.service -f'"
print_status "  Restart service:      ssh $PI_HOST 'sudo systemctl restart shield.service'"
print_status "  Restart nginx:        ssh $PI_HOST 'sudo systemctl restart nginx'"
print_status ""
print_status "Web interface should be available at:"
print_status "  http://shield.home/ (after adding to hosts file)"
print_status ""
print_status "To redeploy quickly: $0 --skip-build $PI_HOST"
