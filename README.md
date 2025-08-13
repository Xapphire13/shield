# Shield

A Rust-based security monitoring service designed for deployment on Raspberry Pi systems.

## Overview

Shield is a workspace project consisting of multiple components:
- `service`: The main backend service
- `app`: Frontend web application (Dioxus-based)
- `models`: Shared data models

## Prerequisites

### Build Machine (ARM Ubuntu)
- ARM-based Ubuntu system (or Raspberry Pi OS running on Raspberry Pi)
- Rust toolchain installed
- Dioxus CLI (`cargo install dioxus-cli`)
- SSH access to target Raspberry Pi

### Target Device (Raspberry Pi)
- Raspberry Pi running a systemd-based Linux distribution
- Network connectivity
- User account with sudo privileges
- Nginx web server (`sudo apt install nginx`)

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

# Install Dioxus CLI for building the web app
cargo install dioxus-cli
```

### Building the Project

```bash
# Clone the repository
git clone <repository-url>
cd shield

# Build the service for release
cargo build --release -p shield-service

# Build the web application
dx bundle --release -p shield-app
```

The compiled binary will be located at: `target/release/shield-service`
The web application will be built in: `app/dist/`

## Deployment

### 1. Copy Files to Target Raspberry Pi

```bash
# Replace with your target Raspberry Pi's IP address
PI_HOST="pi@192.168.1.100"

# Copy the binary
scp target/release/shield-service ${PI_HOST}:~/

# Copy the web application
scp -r app/dist/ ${PI_HOST}:~/shield-web/

# Copy configuration template (if no config exists on target)
scp deploy/shield.config.toml.template ${PI_HOST}:~/shield.config.toml

# Copy systemd service file
scp deploy/shield.service ${PI_HOST}:~/

# Copy nginx configuration
scp deploy/shield.conf ${PI_HOST}:~/
```

### 2. Install on Target Raspberry Pi

SSH into your target Raspberry Pi and run the following commands:

```bash
# Create application directory
sudo mkdir -p /var/lib/shield
sudo chown $USER:$USER /var/lib/shield

# Install nginx if not already installed
sudo apt update
sudo apt install -y nginx

# Install the binary
sudo mv ~/shield-service /usr/bin/shield-service
sudo chmod +x /usr/bin/shield-service

# Install the web application
sudo mkdir -p /var/www/shield
sudo cp -r ~/shield-web/* /var/www/shield/
sudo chown -R www-data:www-data /var/www/shield

# Install nginx configuration
sudo mv ~/shield.conf /etc/nginx/sites-available/
sudo ln -sf /etc/nginx/sites-available/shield.conf /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx

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

# Check nginx status
sudo systemctl status nginx
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
1. Build the backend service and web application on your build machine
2. Copy all files to the target Raspberry Pi via SSH
3. Install and configure the service and nginx
4. Start both services and verify they're running

## Web Interface Access

After successful deployment, the web interface will be available at
`http://shield.home/` (requires adding hostname to your local hosts file)

To set up hostname access on your local machine:
```bash
# On your local machine, add this line to /etc/hosts:
192.168.1.100    shield.home    # Replace with your Pi's actual IP
```

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
   cargo build --release -p shield-service
   dx bundle --release -p shield-app
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
2. Copy the new files and restart:
   ```bash
   # From build machine
   scp target/release/shield-service ${PI_HOST}:~/
   scp -r app/dist/ ${PI_HOST}:~/shield-web/

   # On target Raspberry Pi
   sudo mv ~/shield-service /usr/bin/shield-service
   sudo cp -r ~/shield-web/* /var/www/shield/
   sudo chown -R www-data:www-data /var/www/shield
   sudo systemctl start shield.service
   ```

## Troubleshooting

### Log Locations

- **Service logs**: `sudo journalctl -u shield.service`
- **Nginx access logs**: `/var/log/nginx/access.log`
- **Nginx error logs**: `/var/log/nginx/error.log`
- **System logs**: `/var/log/syslog`

### Configuration Files

- **Service config**: `~/shield.config.toml`
- **Nginx config**: `/etc/nginx/sites-available/shield.conf`
- **Systemd service**: `/etc/systemd/system/shield.service`

## Development

### Local Development
```bash
# Run the service locally
cargo run -p shield-service

# Run the web app locally
dx serve -p shield-app
```

## License

See `LICENSE` file for license information.
