# Shield

A Rust-based security monitoring service designed for deployment on Raspberry Pi systems.

## Overview

Shield is a workspace project consisting of multiple components:
- `service`: The main backend service
- `app`: Frontend web application (Dioxus-based)
- `models`: Shared data models

## Prerequisites

### Build Machine
- [Podman](https://podman.io/docs/installation) (on macOS: `brew install podman && podman machine init && podman machine start`)
- SSH access to target Raspberry Pi

The Rust toolchain and Dioxus CLI are **not** required on the build machine —
they live in the build container defined by `Containerfile.build`.

### Target Device (Raspberry Pi)
- Raspberry Pi running a systemd-based Linux distribution
- Network connectivity
- User account with sudo privileges
- Caddy web server (install via the [official Caddy apt repo](https://caddyserver.com/docs/install#debian-ubuntu-raspbian))

## Building

Shield builds inside a Podman container based on `rust:1-bookworm`, which matches
the Raspberry Pi's OS (Debian bookworm / glibc 2.36). On an Apple Silicon Mac the
Podman machine runs aarch64 Linux, so the service compiles natively for
`aarch64-unknown-linux-gnu` with no cross toolchain. The web app compiles to
architecture-independent WASM.

```bash
# Clone the repository
git clone <repository-url>
cd shield

# Build the builder image (first time, and whenever Containerfile.build changes)
podman build -t shield-builder -f Containerfile.build .

# Build the service binary and web bundle inside the container.
# The repo is bind-mounted, so artifacts land under target/ on the host; the
# cargo registry is cached in a named volume across runs.
podman run --rm \
    -v "$PWD":/src \
    -v shield-cargo-registry:/usr/local/cargo/registry \
    shield-builder \
    bash -c "cargo build --release -p shield-service --target aarch64-unknown-linux-gnu && dx bundle --release -p shield-app"
```

The compiled binary will be located at: `target/aarch64-unknown-linux-gnu/release/shield-service`
The web application will be built in: `target/dx/shield-app/release/web/public`

## Deployment

### 1. Copy Files to Target Raspberry Pi

```bash
# Replace with your target Raspberry Pi's IP address
PI_HOST="pi@192.168.1.100"

# Copy the binary
scp target/aarch64-unknown-linux-gnu/release/shield-service ${PI_HOST}:~/

# Copy the web application
scp -r target/dx/shield-app/release/web/public ${PI_HOST}:~/shield-web/

# Copy configuration template (if no config exists on target)
scp deploy/shield.config.toml.template ${PI_HOST}:~/shield.config.toml

# Copy systemd service file
scp deploy/shield.service ${PI_HOST}:~/

# Copy Caddy configuration
scp deploy/shield.caddy ${PI_HOST}:~/
```

### 2. Install on Target Raspberry Pi

SSH into your target Raspberry Pi and run the following commands:

```bash
# Create application directory
sudo mkdir -p /var/lib/shield
sudo chown $USER:$USER /var/lib/shield

# Install Caddy if not already installed (see https://caddyserver.com/docs/install)

# Install the binary
sudo mv ~/shield-service /usr/bin/shield-service
sudo chmod +x /usr/bin/shield-service

# Install the web application
sudo mkdir -p /var/www/shield
sudo cp -r ~/shield-web/* /var/www/shield/
sudo chown -R www-data:www-data /var/www/shield

# Install Caddy configuration
sudo mkdir -p /etc/caddy/conf.d
# ensure the main Caddyfile imports conf.d (one-time)
grep -q 'import /etc/caddy/conf.d/\*.caddy' /etc/caddy/Caddyfile || \
    echo 'import /etc/caddy/conf.d/*.caddy' | sudo tee -a /etc/caddy/Caddyfile
sudo mv ~/shield.caddy /etc/caddy/conf.d/shield.caddy
sudo caddy validate --config /etc/caddy/Caddyfile && sudo systemctl reload caddy

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

# Check Caddy status
sudo systemctl status caddy
```

## Automated Deployment

Use the provided deployment script for easier deployment:

```bash
# Deploy to target Raspberry Pi
./scripts/deploy.sh pi@192.168.1.100
```

The script will:
1. Build the backend service and web application on your build machine
2. Copy all files to the target Raspberry Pi via SSH
3. Install and configure the service and Caddy
4. Start both services and verify they're running

## Web Interface Access

After successful deployment, the web interface will be available at
`https://shield.home/` (requires adding hostname to your local hosts file)

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

1. Build the new version in the container (see [Building](#building)):
   ```bash
   podman run --rm \
       -v "$PWD":/src \
       -v shield-cargo-registry:/usr/local/cargo/registry \
       shield-builder \
       bash -c "cargo build --release -p shield-service --target aarch64-unknown-linux-gnu && dx bundle --release -p shield-app"
   ```

2. Use the deployment script for automatic update:
   ```bash
   ./scripts/deploy.sh pi@192.168.1.100
   ```

Or manually:

1. Stop the service on the target Raspberry Pi:
   ```bash
   sudo systemctl stop shield.service
   ```
2. Copy the new files and restart:
   ```bash
   # From build machine
   scp target/aarch64-unknown-linux-gnu/release/shield-service ${PI_HOST}:~/
   scp -r target/dx/shield-app/release/web/public/ ${PI_HOST}:~/shield-web/

   # On target Raspberry Pi
   sudo mv ~/shield-service /usr/bin/shield-service
   sudo cp -r ~/shield-web/* /var/www/shield/
   sudo chown -R www-data:www-data /var/www/shield
   sudo systemctl start shield.service
   ```

## Troubleshooting

### Log Locations

- **Service logs**: `sudo journalctl -u shield.service`
- **Caddy logs**: `sudo journalctl -u caddy`
- **System logs**: `/var/log/syslog`

### Configuration Files

- **Service config**: `~/shield.config.toml`
- **Caddy config**: `/etc/caddy/conf.d/shield.caddy`
- **Systemd service**: `/etc/systemd/system/shield.service`

> **Note**: The site uses Caddy's internal CA (`tls internal`). Client devices
> must trust Caddy's local root CA, found on the Pi at
> `/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt`.

## Development

### Local Development

The web app's component styles are scoped CSS modules (`*.module.css` files
co-located with their components), bundled into the gitignored
`app/assets/styles.css` by the [stylance](https://github.com/basro/stylance-rs)
CLI. One-time setup:

```bash
cargo install stylance-cli --locked
```

Run the service, web app, and stylance watcher together in a tmux session
(destroyed on detach/exit):

```bash
./scripts/dev.sh
```

Or run them individually:

```bash
# Run the service locally
cargo run -p shield-service

# Rebundle scoped component CSS on save (or one-shot without --watch)
stylance --watch app

# Run the web app locally
dx serve -p shield-app
```

## License

See `LICENSE` file for license information.
