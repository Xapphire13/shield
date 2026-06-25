#!/bin/bash

# Shield Deployment Script
# This script automates the deployment of Shield service to a target Raspberry Pi
# Builds in a Podman container (no host Rust toolchain needed) and deploys via SSH

set -e

# Configuration
PI_HOST=""
SERVICE_NAME="shield-service"
INSTALL_DIR="/var/lib/shield"
BIN_DIR="/usr/bin"
SYSTEMD_DIR="/etc/systemd/system"
WEB_DIR="/var/www/shield"
CADDY_CONF_DIR="/etc/caddy/conf.d"
TARGET="aarch64-unknown-linux-gnu"
BUILDER_IMAGE="shield-builder"
CARGO_CACHE_VOLUME="shield-cargo-registry"

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

# Memory (in MiB) to give the podman machine while building. The native Pi
# binary + web bundle build is memory-hungry, so we temporarily bump the VM to
# 8GB and restore the original value when we're done.
PODMAN_MEMORY_MB=8192

# Tracks whether this script started the podman machine, so we only stop (and
# restore the memory of) a machine we ourselves brought up — an already-running
# machine is left untouched. PODMAN_ORIGINAL_MEMORY holds the MiB value to put
# back on stop.
PODMAN_MACHINE_STARTED=false
PODMAN_ORIGINAL_MEMORY=""

# Stop the podman machine if we started it, restoring its original memory.
# Registered as an EXIT/INT/TERM trap (below) so it runs even when the build
# fails under `set -e` or is Ctrl-C'd.
stop_podman_machine() {
    if [[ "$PODMAN_MACHINE_STARTED" == true ]]; then
        print_step "Stopping podman machine..."
        podman machine stop || print_warning "Failed to stop podman machine"
        PODMAN_MACHINE_STARTED=false

        if [[ -n "$PODMAN_ORIGINAL_MEMORY" ]]; then
            print_step "Restoring podman machine memory to ${PODMAN_ORIGINAL_MEMORY}MiB..."
            podman machine set --memory "$PODMAN_ORIGINAL_MEMORY" \
                || print_warning "Failed to restore podman machine memory"
        fi
    fi
}
trap stop_podman_machine EXIT INT TERM

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
    echo "Note: This script builds in a Podman container and deploys to the target Pi"
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
BINARY_PATH="target/$TARGET/release/$SERVICE_NAME"
WEB_BUILD_PATH="target/dx/shield-app/release/web/public"

if [[ "$SKIP_BUILD" == false ]]; then
    # Build inside a Linux container matching the deploy target (Debian bookworm /
    # glibc 2.36). On an Apple Silicon Mac the podman machine is aarch64 Linux, so
    # the service is a native aarch64 build — no cross toolchain. The repo is
    # bind-mounted, so artifacts land under target/ on the host just like a local
    # build; the cargo registry is cached across runs.
    if ! command -v podman &> /dev/null; then
        print_error "podman not found. Install it with:"
        print_error "  brew install podman && podman machine init && podman machine start"
        exit 1
    fi

    # Bring up the podman machine (the Linux VM the build runs in). `podman info`
    # succeeds only when a container backend is reachable; on a native Linux host
    # it always succeeds, so this is a no-op there. Claim ownership before
    # starting so an interrupt mid-start still triggers cleanup via the trap.
    if ! podman info >/dev/null 2>&1; then
        # Capture the machine's current memory so the trap can restore it, then
        # bump to PODMAN_MEMORY_MB. `set` only takes effect while the machine is
        # stopped, so this must happen before `machine start`.
        PODMAN_ORIGINAL_MEMORY=$(podman machine inspect --format '{{.Resources.Memory}}' 2>/dev/null || true)

        print_step "Setting podman machine memory to ${PODMAN_MEMORY_MB}MiB..."
        podman machine set --memory "$PODMAN_MEMORY_MB"

        print_step "Starting podman machine..."
        PODMAN_MACHINE_STARTED=true
        podman machine start
    fi

    print_step "Building builder image..."
    podman build -t "$BUILDER_IMAGE" -f Containerfile.build .

    print_step "Building shield-service and web application in container..."
    podman run --rm \
        -v "$PWD":/src \
        -v "$CARGO_CACHE_VOLUME":/usr/local/cargo/registry \
        "$BUILDER_IMAGE" \
        bash -c "cargo build --release -p shield-service --target $TARGET && dx bundle --release -p shield-app"

    if [[ ! -f "$BINARY_PATH" ]]; then
        print_error "Binary not found at $BINARY_PATH"
        exit 1
    fi
    print_status "Binary built successfully: $BINARY_PATH"

    if [[ ! -d "$WEB_BUILD_PATH" ]]; then
        print_error "Web build not found at $WEB_BUILD_PATH"
        exit 1
    fi
    print_status "Web application built successfully: $WEB_BUILD_PATH"
else
    print_status "Skipping build step"

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

# Copy Caddy configuration
print_status "Copying Caddy configuration..."
scp deploy/shield.caddy "$PI_HOST:~/"

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

echo "Checking caddy installation..."
if ! command -v caddy &> /dev/null; then
    echo "Installing caddy..."
    sudo apt-get install -y debian-keyring debian-archive-keyring apt-transport-https curl
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
    sudo apt-get update
    sudo apt-get install -y caddy
else
    echo "Caddy already installed"
fi

echo "Installing web application..."
sudo mkdir -p $WEB_DIR
sudo cp -r ~/shield-web/* $WEB_DIR/
sudo chown -R www-data:www-data $WEB_DIR

echo "Installing Caddy site config..."
sudo mkdir -p $CADDY_CONF_DIR
# ensure the main Caddyfile imports conf.d (idempotent)
if ! sudo grep -q 'import /etc/caddy/conf.d/\*.caddy' /etc/caddy/Caddyfile; then
    echo 'import /etc/caddy/conf.d/*.caddy' | sudo tee -a /etc/caddy/Caddyfile
fi
sudo mv ~/shield.caddy $CADDY_CONF_DIR/shield.caddy

# validate before reloading; roll back the snippet on failure
if sudo caddy validate --config /etc/caddy/Caddyfile; then
    sudo systemctl reload caddy
    echo "Caddy configuration updated successfully"
else
    echo "ERROR: Caddy configuration test failed - rolling back"
    sudo rm -f $CADDY_CONF_DIR/shield.caddy
    sudo caddy validate --config /etc/caddy/Caddyfile && sudo systemctl reload caddy
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
echo "Caddy status:"
if sudo systemctl is-active --quiet caddy; then
    echo "✓ Caddy is running"
else
    echo "✗ Caddy is not running"
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
print_status "  Check caddy status:   ssh $PI_HOST 'sudo systemctl status caddy'"
print_status "  View logs:            ssh $PI_HOST 'sudo journalctl -u shield.service -f'"
print_status "  Restart service:      ssh $PI_HOST 'sudo systemctl restart shield.service'"
print_status "  Reload caddy:         ssh $PI_HOST 'sudo systemctl reload caddy'"
print_status ""
print_status "Web interface should be available at:"
print_status "  https://shield.home/ (after adding to hosts file)"
print_status ""
print_status "To redeploy quickly: $0 --skip-build $PI_HOST"
