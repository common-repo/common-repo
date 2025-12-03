#!/bin/sh
# Install script for common-repo
# Usage: curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
#
# Options (via environment variables):
#   VERSION      - Specific version to install (e.g., "v0.20.0"). Default: latest
#   INSTALL_DIR  - Directory to install to. Default: ~/.local/bin (or /usr/local/bin with sudo)
#   GITHUB_TOKEN - Token for GitHub API (optional, helps avoid rate limits)

set -e

REPO="common-repo/common-repo"
BINARY_NAME="common-repo"

# Colors for output (disabled if not a terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

info() {
    printf "${BLUE}info:${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn:${NC} %s\n" "$1"
}

error() {
    printf "${RED}error:${NC} %s\n" "$1" >&2
}

success() {
    printf "${GREEN}success:${NC} %s\n" "$1"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)       error "Unsupported operating system: $(uname -s)"; exit 1 ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)"; exit 1 ;;
    esac
}

# Map OS and arch to Rust target triple
get_target() {
    os="$1"
    arch="$2"

    case "${os}-${arch}" in
        linux-x86_64)   echo "x86_64-unknown-linux-gnu" ;;
        linux-aarch64)  echo "aarch64-unknown-linux-gnu" ;;
        macos-aarch64)  echo "aarch64-apple-darwin" ;;
        macos-x86_64)
            # macOS x86_64 is not currently built, suggest using Rosetta on Apple Silicon
            # or building from source
            error "macOS x86_64 binaries are not currently provided."
            error "On Apple Silicon, the aarch64 binary works natively."
            error "For Intel Macs, please build from source: cargo install --git https://github.com/${REPO}"
            exit 1
            ;;
        windows-x86_64) echo "x86_64-pc-windows-msvc" ;;
        *)
            error "No prebuilt binary available for ${os}-${arch}"
            error "Please build from source: cargo install --git https://github.com/${REPO}"
            exit 1
            ;;
    esac
}

# Get the latest release version from GitHub
get_latest_version() {
    api_url="https://api.github.com/repos/${REPO}/releases/latest"

    if [ -n "${GITHUB_TOKEN:-}" ]; then
        auth_header="Authorization: token ${GITHUB_TOKEN}"
    else
        auth_header=""
    fi

    if command -v curl >/dev/null 2>&1; then
        if [ -n "${auth_header}" ]; then
            response=$(curl -fsSL -H "${auth_header}" "${api_url}" 2>/dev/null) || {
                error "Failed to fetch latest release. Check your network connection."
                exit 1
            }
        else
            response=$(curl -fsSL "${api_url}" 2>/dev/null) || {
                error "Failed to fetch latest release. Check your network connection."
                exit 1
            }
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ -n "${auth_header}" ]; then
            response=$(wget -qO- --header="${auth_header}" "${api_url}" 2>/dev/null) || {
                error "Failed to fetch latest release. Check your network connection."
                exit 1
            }
        else
            response=$(wget -qO- "${api_url}" 2>/dev/null) || {
                error "Failed to fetch latest release. Check your network connection."
                exit 1
            }
        fi
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    # Parse tag_name from JSON response (simple grep approach for portability)
    version=$(echo "${response}" | grep -o '"tag_name":\s*"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')

    if [ -z "${version}" ]; then
        error "Failed to determine latest version. Response may indicate rate limiting."
        error "Try setting GITHUB_TOKEN environment variable."
        exit 1
    fi

    echo "${version}"
}

# Download file using curl or wget
download_file() {
    url="$1"
    output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "${output}" "${url}" || return 1
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "${output}" "${url}" || return 1
    else
        error "Neither curl nor wget found."
        exit 1
    fi
}

# Determine install directory
get_install_dir() {
    if [ -n "${INSTALL_DIR:-}" ]; then
        echo "${INSTALL_DIR}"
    elif [ "$(id -u)" = "0" ]; then
        echo "/usr/local/bin"
    else
        echo "${HOME}/.local/bin"
    fi
}

# Main installation logic
main() {
    info "Installing ${BINARY_NAME}..."

    # Detect platform
    os=$(detect_os)
    arch=$(detect_arch)
    target=$(get_target "${os}" "${arch}")

    info "Detected platform: ${os}-${arch} (${target})"

    # Get version
    if [ -n "${VERSION:-}" ]; then
        version="${VERSION}"
        # Ensure version starts with 'v'
        case "${version}" in
            v*) ;;
            *)  version="v${version}" ;;
        esac
        info "Installing specified version: ${version}"
    else
        info "Fetching latest release..."
        version=$(get_latest_version)
        info "Latest version: ${version}"
    fi

    # Determine archive extension
    if [ "${os}" = "windows" ]; then
        archive_ext="zip"
    else
        archive_ext="tar.gz"
    fi

    # Construct download URL
    archive_name="${BINARY_NAME}-${version}-${target}.${archive_ext}"
    download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

    info "Downloading ${archive_name}..."

    # Create temp directory
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "${tmp_dir}"' EXIT

    archive_path="${tmp_dir}/${archive_name}"

    if ! download_file "${download_url}" "${archive_path}"; then
        error "Failed to download ${download_url}"
        error "The release may not exist or the binary for your platform may not be available."
        exit 1
    fi

    # Extract archive
    info "Extracting..."
    cd "${tmp_dir}"

    if [ "${archive_ext}" = "zip" ]; then
        if command -v unzip >/dev/null 2>&1; then
            unzip -q "${archive_path}"
        else
            error "unzip is required to extract the Windows binary."
            exit 1
        fi
        binary_path="${tmp_dir}/${BINARY_NAME}.exe"
    else
        tar xzf "${archive_path}"
        binary_path="${tmp_dir}/${BINARY_NAME}"
    fi

    if [ ! -f "${binary_path}" ]; then
        error "Binary not found in archive. The release may be corrupted."
        exit 1
    fi

    # Determine install location
    install_dir=$(get_install_dir)

    # Create install directory if needed
    if [ ! -d "${install_dir}" ]; then
        info "Creating directory ${install_dir}..."
        mkdir -p "${install_dir}"
    fi

    # Install binary
    if [ "${os}" = "windows" ]; then
        install_path="${install_dir}/${BINARY_NAME}.exe"
    else
        install_path="${install_dir}/${BINARY_NAME}"
    fi

    info "Installing to ${install_path}..."

    # Check if we can write to the install directory
    if [ -w "${install_dir}" ]; then
        cp "${binary_path}" "${install_path}"
        chmod +x "${install_path}"
    else
        # Try with sudo
        if command -v sudo >/dev/null 2>&1; then
            warn "Requires elevated permissions to install to ${install_dir}"
            sudo cp "${binary_path}" "${install_path}"
            sudo chmod +x "${install_path}"
        else
            error "Cannot write to ${install_dir} and sudo is not available."
            error "Try: INSTALL_DIR=~/.local/bin sh install.sh"
            exit 1
        fi
    fi

    success "${BINARY_NAME} ${version} installed successfully!"

    # Check if install directory is in PATH
    case ":${PATH}:" in
        *":${install_dir}:"*)
            info "Run '${BINARY_NAME} --version' to verify the installation."
            ;;
        *)
            warn "${install_dir} is not in your PATH."
            warn "Add it to your shell profile:"
            warn ""
            warn "  For bash:  echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.bashrc"
            warn "  For zsh:   echo 'export PATH=\"${install_dir}:\$PATH\"' >> ~/.zshrc"
            warn "  For fish:  fish_add_path ${install_dir}"
            warn ""
            warn "Then restart your shell or run: export PATH=\"${install_dir}:\$PATH\""
            ;;
    esac
}

main "$@"
