#!/bin/sh
# onyx v0.1.1 - stylish brute-strap
set -e

# colors: the "onyx" palette
# the bulletproof basics
BOLD='\033[1m'
VIOLET='\033[1;35m'
CYAN='\033[0;36m'
RED='\033[1;31m'
CLR='\033[0m' # No Color

# clear the clutter
clear

# awesome logo
echo -e "${VIOLET}  ___  _  _ __   ____  __"
echo " / _ \| \| |\ \ / /\ \/ /"
echo "| (_) | .  | \ V /  >  < "
echo " \___/|_|\_|  |_|  /_/\_\\"
echo -e "                          ${CLR}${VIOLET}v0.1.1${CLR}"

# environment detection
if [ -n "$TERMUX_VERSION" ]; then
    TARGET="termux"
    ONYX_DIR="$HOME/.onyx"
else
    TARGET="linux"
    ONYX_DIR="/home/onyx"
fi

echo -e "${BOLD}target:${CLR} $TARGET"
echo -e "${BOLD}path:  ${CLR} $ONYX_DIR"
echo ""
echo "=================================="

# 1. the skeleton (brute-force 777)
echo -e "${CYAN}scanning for life signs...${CLR}"
[ -d "$ONYX_DIR" ] || echo -e "${CYAN}initializing new home...${CLR}"

# create the hierarchy
mkdir -p "$ONYX_DIR/sys" \
         "$ONYX_DIR/bin" \
         "$ONYX_DIR/profiles" \
         "$ONYX_DIR/box64" \
         "$ONYX_DIR/glibc" \
         "$ONYX_DIR/tmp"

# 2. install performance profiles
echo -e "${CYAN}grabbing performance profiles...${CLR}"

# the brute list of profiles from your repo
PROFILES="cinderblock performant balanced limited bounded potato brick"

for P in $PROFILES; do
    if [ ! -f "$ONYX_DIR/profiles/$P.toml" ]; then
        echo "  => fetching $P profile..."
        # pull from your github structure
        URL="https://raw.githubusercontent.com/arozoid/onyx/main/profiles/$P.toml"
        curl -sL "$URL" -o "$ONYX_DIR/profiles/$P.toml" || echo "${RED}     ! failed to fetch $P${CLR}"
    fi
done

# 3. fetching the soul (v0.1.1 binary)
echo -e "${CYAN}fetching latest binary...${CLR}"
# detecting architecture for the release header
ARCH=$(uname -m)
case $ARCH in
    x86_64)  URL="https://github.com/arozoid/onyx/releases/latest/download/onyx-x86_64" ;;
    aarch64) URL="https://github.com/arozoid/onyx/releases/latest/download/onyx-aarch64" ;;
    *)       URL="https://github.com/arozoid/onyx/releases/latest/download/onyx-aarch64" ;;
esac

# dummy download (replace with curl -L "$URL" -o "$ONYX_DIR/bin/onyx")
# using -L because redirects are the only way to hit "latest"
if ! curl -sL "$URL" -o "$ONYX_DIR/bin/core/onyx"; then
    echo "${RED}error: brian blocked the download. check connection.${CLR}"
    exit 1
fi

# 4. setting the "world-writable" brute permissions
echo -e "${CYAN}unlocking the gates...${CLR}"
chmod -R 777 "$ONYX_DIR"
chmod +x "$ONYX_DIR/bin/onyx"

# 5. seeding the first profile
if [ ! -f "$ONYX_DIR/profiles/default.toml" ]; then
    cat <<EOF > "$ONYX_DIR/profiles/default.toml"
[profile]
name = "default"
max_memory = "32mb"
cpu_shares = 512
EOF
fi

echo "=================================="
echo -e "${BOLD}onyx is now installed.${CLR}"
echo -e "run: ${CYAN}export PATH=\$PATH:$ONYX_DIR/bin/core${CLR}"
echo -e "then: ${CYAN}onyx box create my-box /path/to/rootfs FALSE${CLR}"

