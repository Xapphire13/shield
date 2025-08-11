# Shield

A Rust-based security monitoring service designed for deployment on Raspberry Pi systems.

## Overview

Shield is a workspace project consisting of multiple components:
- `service`: The main backend service
- `app`: Frontend application
- `models`: Shared data models

## Prerequisites

### Build Machine (ARM Ubuntu)
- ARM-based Ubuntu system (or Raspberry Pi OS running on Raspberry Pi)
- Rust toolchain installed
- SSH access to target Raspberry Pi

### Target Device (Raspberry Pi)
- Raspberry Pi running a systemd-based Linux distribution
- Network connectivity
- User account with sudo privileges

## Building

### Setting up the Build Environment

On your ARM Ubuntu build machine:

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install build dependencies
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

### Building the Project

```bash
# Clone the repository
git clone <repository-url>
cd shield

# Build the service for release
cargo build --release --bin shield-service
```

The compiled binary will be located at: `target/release/shield-service`

## Deployment

### 1. Copy Files to Target Raspberry Pi

```bash
# Replace with your target Raspberry Pi's IP address
PI_HOST="pi@192.168.1.100"

# Copy the binary
scp target/release/shield-service ${PI_HOST}:~/

# Copy configuration template (if no config exists on target)
scp deploy/shield.config.toml.template ${PI_HOST}:~/shield.config.toml

# Copy systemd service file
scp deploy/shield.service ${PI_HOST}:~/
```

### 2. Install on Target Raspberry Pi

SSH into your target Raspberry Pi and run the following commands:

```bash
# Create application directory
sudo mkdir -p /var/lib/shield
sudo chown $USER:$USER /var/lib/shield

# Install the binary
sudo mv ~/shield-service /usr/bin/shield-service
sudo chmod +x /usr/bin/shield-service

# Install systemd service
sudo mv ~/shield.service /etc/systemd/system/
sudo systemctl daemon-reload

# Enable and start the service
sudo systemctl enable shield.service
sudo systemctl start shield.service
```

### 3. Verify Installation

```bash
# Check service status
sudo systemctl status shield.service

# View logs
sudo journalctl -u shield.service -f
```

## Automated Deployment

Use the provided deployment script for easier deployment:

```bash
# Make the script executable (first time only)
chmod +x deploy/deploy.sh

# Deploy to target Raspberry Pi
./deploy/deploy.sh pi@192.168.1.100
```

The script will:
1. Build the project on your ARM Ubuntu machine
2. Copy files to the target Raspberry Pi via SSH
3. Install and configure the service
4. Start the service and verify it's running

## Configuration

The service uses a TOML configuration file located at `~/shield.config.toml`.

### Initial Configuration

On first deployment, a template configuration file is installed with placeholder values:

```toml
[credentials]
username = "YOUR_UNIFI_USERNAME"
password = "YOUR_UNIFI_PASSWORD"

# [otp]
# secret = "GENERATED_ON_FIRST_RUN"
#
# [jwt]
# secret = "GENERATED_ON_FIRST_RUN"
#
# [notifications]
# topic = "YOUR_NTFY_TOPIC"
```

**Important**: You must configure your UniFi credentials on the target Raspberry Pi.

```bash
# SSH into your Raspberry Pi
ssh pi@192.168.1.100

# Edit the configuration file
nano ~/shield.config.toml

# Update the username and password fields:
# username = "your_actual_unifi_username"
# password = "your_actual_unifi_password"

# The service will automatically generate secrets on first start
```

After editing the configuration, restart the service:

```bash
sudo systemctl restart shield.service
```

### Configuration Updates

The deployment script preserves existing configuration files. If you need to update configuration:

1. SSH into the target Raspberry Pi
2. Edit `~/shield.config.toml`
3. Restart the service: `sudo systemctl restart shield.service`

## Updating

To update the service:

1. Build the new version on your ARM Ubuntu build machine:
   ```bash
   cargo build --release --bin shield-service
   ```

2. Use the deployment script for automatic update:
   ```bash
   ./deploy/deploy.sh pi@192.168.1.100
   ```

Or manually:

1. Stop the service on the target Raspberry Pi:
   ```bash
   sudo systemctl stop shield.service
   ```
2. Copy the new binary and restart:
   ```bash
   # From build machine
   scp target/release/shield-service ${PI_HOST}:~/

   # On target Raspberry Pi
   sudo mv ~/shield-service /usr/bin/shield-service
   sudo systemctl start shield.service
   ```

## Development

### Local Development
```bash
# Run the service locally
cargo run -p shield-service
```

## License

See `LICENSE` file for license information.
