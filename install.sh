#!/bin/sh
set -e

REPO="museslabs/stochos"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY="stochos"

get_latest_tag() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | cut -d '"' -f 4
}

main() {
    tag="${1:-$(get_latest_tag)}"
    if [ -z "$tag" ]; then
        echo "error: could not determine latest release" >&2
        exit 1
    fi

    arch="$(uname -m)"
    case "$arch" in
        x86_64) arch_name="x86_64" ;;
        *) echo "error: unsupported architecture: $arch" >&2; exit 1 ;;
    esac

    os="$(uname -s)"
    case "$os" in
        Linux) os_name="linux" ;;
        *) echo "error: unsupported OS: $os" >&2; exit 1 ;;
    esac

    tarball="${BINARY}-${os_name}-${arch_name}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${tag}/${tarball}"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    echo "downloading ${BINARY} ${tag}..."
    curl -fsSL "$url" -o "${tmpdir}/${tarball}"
    tar xzf "${tmpdir}/${tarball}" -C "$tmpdir"

    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    else
        echo "installing to ${INSTALL_DIR} (requires sudo)..."
        sudo mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    fi

    chmod +x "${INSTALL_DIR}/${BINARY}"
    echo "installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"
}

main "$@"
