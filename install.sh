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
echo -e "                          ${CLR}${VIOLET}v0.1.2${CLR}"

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

PROFILES="cinderblock performant balanced limited bounded potato brick"

for P in $PROFILES; do
    TARGET_FILE="$ONYX_DIR/profiles/$P.toml"
    if [ ! -f "$TARGET_FILE" ]; then
        echo "  -> fetching $P..."
        URL="https://raw.githubusercontent.com/arozoid/onyx/main/profiles/$P.toml"
        # ensure directory exists just in case
        mkdir -p "$ONYX_DIR/profiles"
        curl -fsSL "$URL" -o "$TARGET_FILE" || echo -e "${RED}     ! failed to fetch $P${CLR}"
    fi
done

# 3. fetching the soul (v0.1.1 binary)
echo -e "${CYAN}fetching latest binary...${CLR}"

# ensure the 'core' directory exists
mkdir -p "$ONYX_DIR/bin/core"

if [ -n "$TERMUX_VERSION" ]; then
    URL="https://github.com/arozoid/onyx/releases/latest/download/onyx-android-aarch64"
else
    case $(uname -m) in
        x86_64)  URL="https://github.com/arozoid/onyx/releases/latest/download/onyx-x86_64" ;;
        aarch64) URL="https://github.com/arozoid/onyx/releases/latest/download/onyx-aarch64" ;;
        *)       URL="https://github.com/arozoid/onyx/releases/latest/download/onyx-aarch64" ;;
    esac
fi

# download to the specific core bin
curl -fsSL "$URL" -o "$ONYX_DIR/bin/core/onyx"
chmod +x "$ONYX_DIR/bin/core/onyx"

# 4. setting the "world-writable" brute permissions
echo -e "${CYAN}unlocking the gates...${CLR}"
chmod -R 777 "$ONYX_DIR"
chmod +x "$ONYX_DIR/bin/core/onyx"

echo "=================================="
echo -e "${BOLD}onyx is now installed.${CLR}"
echo -e "run: ${CYAN}export PATH=\$PATH:$ONYX_DIR/bin/core${CLR}"
echo -e "then: ${CYAN}onyx box create my-box /path/to/rootfs FALSE${CLR}"

