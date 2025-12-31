#!/bin/sh
# Install script for common-repo
# Usage: curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
#
# Options (via environment variables):
#   VERSION      - Specific version to install (e.g., "v0.20.0"). Default: latest
#   INSTALL_DIR  - Directory to install to. Default: ~/.local/bin (or /usr/local/bin with sudo)
#   GITHUB_TOKEN - Token for GitHub API (optional, helps avoid rate limits)
#   INSTALL_PREK - Set to "1" to also install prek (fast pre-commit alternative)
#                  If unset and running interactively, will prompt
#   SKIP_ALIAS   - Set to "1" to skip creating the 'cr' short alias

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

# Check if prek should be installed
should_install_prek() {
    # Already have prek installed
    if command -v prek >/dev/null 2>&1; then
        return 1
    fi

    # Explicit env var takes precedence
    if [ "${INSTALL_PREK:-}" = "1" ]; then
        return 0
    elif [ -n "${INSTALL_PREK:-}" ]; then
        # Any other non-empty value means no
        return 1
    fi

    # If not interactive, don't prompt
    if [ ! -t 0 ] || [ ! -t 1 ]; then
        return 1
    fi

    # Interactive prompt
    printf "%s" "${BLUE}?${NC} Would you like to install prek (fast pre-commit hooks)? [y/N] "
    read -r response
    case "${response}" in
        [yY]|[yY][eE][sS]) return 0 ;;
        *) return 1 ;;
    esac
}

# Get latest prek version from GitHub
get_prek_version() {
    api_url="https://api.github.com/repos/j178/prek/releases/latest"

    if [ -n "${GITHUB_TOKEN:-}" ]; then
        auth_header="Authorization: token ${GITHUB_TOKEN}"
    else
        auth_header=""
    fi

    if command -v curl >/dev/null 2>&1; then
        if [ -n "${auth_header}" ]; then
            response=$(curl -fsSL -H "${auth_header}" "${api_url}" 2>/dev/null) || return 1
        else
            response=$(curl -fsSL "${api_url}" 2>/dev/null) || return 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if [ -n "${auth_header}" ]; then
            response=$(wget -qO- --header="${auth_header}" "${api_url}" 2>/dev/null) || return 1
        else
            response=$(wget -qO- "${api_url}" 2>/dev/null) || return 1
        fi
    else
        return 1
    fi

    echo "${response}" | grep -o '"tag_name":\s*"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/'
}

# Get prek target triple
get_prek_target() {
    os="$1"
    arch="$2"

    case "${os}-${arch}" in
        linux-x86_64)   echo "x86_64-unknown-linux-gnu" ;;
        linux-aarch64)  echo "aarch64-unknown-linux-gnu" ;;
        macos-aarch64)  echo "aarch64-apple-darwin" ;;
        macos-x86_64)   echo "x86_64-apple-darwin" ;;
        windows-x86_64) echo "x86_64-pc-windows-msvc" ;;
        *) return 1 ;;
    esac
}

# Create cr alias symlink
create_cr_alias() {
    install_dir="$1"
    os="$2"

    # Skip if SKIP_ALIAS is set
    if [ "${SKIP_ALIAS:-}" = "1" ]; then
        info "Skipping cr alias creation (SKIP_ALIAS=1)"
        return 0
    fi

    # Windows doesn't support symlinks easily; create a wrapper script instead
    if [ "${os}" = "windows" ]; then
        cr_path="${install_dir}/cr.cmd"
        if [ -e "${cr_path}" ]; then
            # Check if it's our wrapper
            if [ -f "${cr_path}" ] && grep -q "common-repo" "${cr_path}" 2>/dev/null; then
                info "cr alias already exists"
                return 0
            else
                warn "Skipping cr alias: ${cr_path} already exists (not our file)"
                return 0
            fi
        fi
        # Create Windows batch wrapper
        if [ -w "${install_dir}" ]; then
            printf '@echo off\r\n"%%~dp0common-repo.exe" %%%%*\r\n' > "${cr_path}"
        else
            if command -v sudo >/dev/null 2>&1; then
                printf '@echo off\r\n"%%~dp0common-repo.exe" %%%%*\r\n' | sudo tee "${cr_path}" > /dev/null
            else
                warn "Cannot create cr alias: no write permission to ${install_dir}"
                return 0
            fi
        fi
        success "Created cr alias (Windows batch wrapper)"
        return 0
    fi

    # Unix: create symlink
    cr_path="${install_dir}/cr"
    binary_name="${install_dir}/common-repo"

    if [ -e "${cr_path}" ] || [ -L "${cr_path}" ]; then
        # Check if it's already our symlink
        if [ -L "${cr_path}" ]; then
            link_target=$(readlink "${cr_path}" 2>/dev/null || true)
            if [ "${link_target}" = "common-repo" ] || [ "${link_target}" = "${binary_name}" ]; then
                info "cr alias already exists"
                return 0
            fi
        fi
        warn "Skipping cr alias: ${cr_path} already exists"
        warn "To use the alias, remove the existing file and reinstall"
        return 0
    fi

    # Create the symlink (relative path for portability)
    if [ -w "${install_dir}" ]; then
        ln -s "common-repo" "${cr_path}"
    else
        if command -v sudo >/dev/null 2>&1; then
            sudo ln -s "common-repo" "${cr_path}"
        else
            warn "Cannot create cr alias: no write permission to ${install_dir}"
            return 0
        fi
    fi

    success "Created cr alias (symlink to common-repo)"
}

# Install prek to the specified directory
install_prek() {
    install_dir="$1"
    os="$2"
    arch="$3"

    info "Installing prek..."

    prek_target=$(get_prek_target "${os}" "${arch}") || {
        warn "No prek binary available for ${os}-${arch}, skipping prek installation"
        return 0
    }

    prek_version=$(get_prek_version) || {
        warn "Could not determine latest prek version, skipping prek installation"
        return 0
    }

    info "Downloading prek ${prek_version}..."

    # prek releases use .tar.gz for all platforms except Windows
    if [ "${os}" = "windows" ]; then
        prek_archive="prek-${arch}-pc-windows-msvc.zip"
        prek_ext="zip"
    else
        prek_archive="prek-${prek_target}.tar.gz"
        prek_ext="tar.gz"
    fi

    prek_url="https://github.com/j178/prek/releases/download/${prek_version}/${prek_archive}"

    prek_tmp=$(mktemp -d)
    prek_archive_path="${prek_tmp}/${prek_archive}"

    if ! download_file "${prek_url}" "${prek_archive_path}"; then
        warn "Failed to download prek, skipping installation"
        rm -rf "${prek_tmp}"
        return 0
    fi

    # Extract prek
    cd "${prek_tmp}"
    if [ "${prek_ext}" = "zip" ]; then
        if command -v unzip >/dev/null 2>&1; then
            unzip -q "${prek_archive_path}"
        else
            warn "unzip not available, skipping prek installation"
            rm -rf "${prek_tmp}"
            return 0
        fi
        prek_binary="${prek_tmp}/prek.exe"
    else
        tar xzf "${prek_archive_path}"
        prek_binary="${prek_tmp}/prek"
    fi

    if [ ! -f "${prek_binary}" ]; then
        warn "prek binary not found in archive, skipping installation"
        rm -rf "${prek_tmp}"
        return 0
    fi

    # Install prek binary
    if [ "${os}" = "windows" ]; then
        prek_install_path="${install_dir}/prek.exe"
    else
        prek_install_path="${install_dir}/prek"
    fi

    if [ -w "${install_dir}" ]; then
        cp "${prek_binary}" "${prek_install_path}"
        chmod +x "${prek_install_path}"
    else
        if command -v sudo >/dev/null 2>&1; then
            sudo cp "${prek_binary}" "${prek_install_path}"
            sudo chmod +x "${prek_install_path}"
        else
            warn "Cannot write to ${install_dir}, skipping prek installation"
            rm -rf "${prek_tmp}"
            return 0
        fi
    fi

    rm -rf "${prek_tmp}"

    # Show version
    if [ -x "${prek_install_path}" ]; then
        prek_installed_version=$("${prek_install_path}" --version 2>/dev/null || echo "unknown")
        success "prek ${prek_installed_version} installed successfully!"
    else
        success "prek installed successfully!"
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

    # Create cr alias (short name)
    create_cr_alias "${install_dir}" "${os}"

    # Optionally install prek
    if should_install_prek; then
        install_prek "${install_dir}" "${os}" "${arch}"
    fi

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
