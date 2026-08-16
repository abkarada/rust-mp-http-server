#!/usr/bin/env sh
set -e

echo "==> Installing High-Performance Multi-Protocol HTTP Server..."

REPO_URL="https://github.com/abkarada/rust-mp-http-server"
BIN_NAME="high-performance-http-server"
INSTALL_DIR="/usr/local/bin"
SERVICE_PATH="/etc/systemd/system/mp-http-server.service"

if command -v cargo >/dev/null 2>&1; then
    echo "==> Building release binary with Cargo..."
    cargo build --release
    sudo cp target/release/"$BIN_NAME" "$INSTALL_DIR/mp-http-server"
else
    echo "Error: Cargo is required to build from source. Please install Rust via https://rustup.rs"
    exit 1
fi

if [ -d /etc/systemd/system ]; then
    echo "==> Registering systemd service..."
    cat <<EOF | sudo tee "$SERVICE_PATH" > /dev/null
[Unit]
Description=High-Performance Multi-Protocol HTTP Server
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/mp-http-server --directory /var/www/html
Restart=on-failure
User=nobody

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    echo "==> Installation complete!"
    echo "Start service with: sudo systemctl start mp-http-server"
    echo "Enable service on boot: sudo systemctl enable mp-http-server"
else
    echo "==> Installation complete! Binary available at $INSTALL_DIR/mp-http-server"
fi
