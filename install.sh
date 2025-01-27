#!/bin/bash

# Check if extension ID is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <chrome_extension_id>"
    exit 1
fi

EXTENSION_ID="$1"

# Function to install Rust
install_rust() {
    echo "Checking for Rust installation..."
    echo "Current PATH: $PATH"
    if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
        echo "Rust and Cargo are already installed. Skipping installation."
    else
        echo "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    fi
}

# Function to ensure nightly Rust is installed
ensure_nightly_rust() {
    if rustup toolchain list | grep -q "nightly"; then
        echo "Nightly Rust is already installed."
    else
        echo "Installing nightly Rust..."
        rustup install nightly
    fi
    rustup default nightly
}

# Function to install the project using Cargo
install_project() {
    if cargo install --list | grep -q "krithon-prover"; then
        echo "krithon-prover is already installed. Skipping installation."
    else
        echo "Installing krithon-prover..."
        cargo +nightly install --git https://github.com/Gorocy/krithon-prover
    fi
}

# Function to create JSON manifest
create_manifest() {
    local binary_path="$1"
    local manifest_path="$2"
    echo "Creating JSON manifest at $manifest_path..."
    sudo bash -c "cat <<EOF > \"$manifest_path\"
{
    \"name\": \"com.notary.krithon\",
    \"description\": \"Krithon\",
    \"path\": \"$binary_path\",
    \"type\": \"stdio\",
    \"allowed_origins\": [
        \"chrome-extension://$EXTENSION_ID/\"
    ]
}
EOF"
}

# Detect the operating system
OS="$(uname -s)"
case "$OS" in
    Linux*|Darwin*)
        install_rust
        ensure_nightly_rust
        install_project
        BINARY_PATH="$(echo ~/.cargo/bin/krithon-prover)"
        MANIFEST_PATH="/etc/opt/chrome/native-messaging-hosts/com.notary.krithon.json"
        if [ "$OS" = "Darwin" ]; then
            MANIFEST_PATH="/Library/Google/Chrome/NativeMessagingHosts/com.notary.krithon.json"
        fi
        sudo mkdir -p "$(dirname "$MANIFEST_PATH")"
        create_manifest "$BINARY_PATH" "$MANIFEST_PATH"
        ;;
    CYGWIN*|MINGW32*|MSYS*|MINGW*)
        echo "Windows detected. Please install Rust manually from https://rustup.rs/"
        echo "After installation, run the following command in a new terminal:"
        echo "rustup install nightly && rustup default nightly"
        echo "cargo +nightly install --git https://github.com/Gorocy/krithon-prover"
        echo "Then, create a registry entry for the manifest file."
        ;;
    *)
        echo "Unknown OS: $OS. Exiting."
        ;;
esac