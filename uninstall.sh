#!/bin/bash

# Function to uninstall the project using Cargo
uninstall_project() {
    echo "Uninstalling krithon-prover..."
    cargo uninstall krithon-prover
}

# Function to remove JSON manifest
remove_manifest() {
    local manifest_path="$1"
    if [ -f "$manifest_path" ]; then
        echo "Removing JSON manifest at $manifest_path..."
        sudo rm "$manifest_path"
    else
        echo "Manifest file not found at $manifest_path."
    fi
}

# Detect the operating system
OS="$(uname -s)"
case "$OS" in
    Linux*)
        uninstall_project
        MANIFEST_PATH="/etc/opt/chrome/native-messaging-hosts/com.notary.krithon.json"
        remove_manifest "$MANIFEST_PATH"
        ;;
    Darwin*)
        uninstall_project
        MANIFEST_PATH="/Library/Google/Chrome/NativeMessagingHosts/com.notary.krithon.json"
        remove_manifest "$MANIFEST_PATH"
        ;;
    CYGWIN*|MINGW32*|MSYS*|MINGW*)
        echo "Windows detected. Please manually uninstall krithon-prover using Cargo."
        echo "Then, remove the registry entry for the manifest file."
        ;;
    *)
        echo "Unknown OS: $OS. Exiting."
        ;;
esac 