#!/bin/sh

BOLD='\033[1m'
ITALIC='\033[3m'
NC='\033[0m'
BBCYAN='\033[1;96m'

PROGRAM="${BBCYAN}SysD Manager${NC}"


if [ $# -eq 0 ]; then
    TARGET="release"
    BUILD_ARG="--release"
fi

if [[ "$1" == "debug" ]]; then
    echo Debug installation
    TARGET="debug"
fi

REL_PATH=".."

echo -e Compiling $PROGRAM program
echo ""
cargo build $BUILD_ARG || exit 1
echo ""
echo Installing files
echo ""
sudo install -Dm755 "${REL_PATH}/target/${TARGET}/sysd-manager" -t "/usr/bin"
sudo install -Dm644 "${REL_PATH}/target/locale/io.github.plrigaux.sysd-manager.desktop" -t "/usr/share/applications"
sudo install -Dm644 "${REL_PATH}/data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg" -t "/usr/share/icons/hicolor/scalable/apps/"
sudo install -Dm644 "${REL_PATH}/data/schemas/io.github.plrigaux.sysd-manager.gschema.xml" -t "/usr/share/glib-2.0/schemas"   
sudo cp -r -v "${REL_PATH}/target/locale"  "/usr/share/" 
#sudo install -Dm644 "data/applications/org.freedesktop.policykit.sysd-manager.policy" -t "/usr/share/polkit-1/actions/"

echo -e Compiling ${BOLD}Schemas${NC} 
echo ""
sudo glib-compile-schemas "/usr/share/glib-2.0/schemas"

echo -e Installation of $PROGRAM completed, enjoy.

COMPILE_SIZE=$(du -sh "${REL_PATH}/target")
COMPILE_SIZE_ARR=($COMPILE_SIZE)

echo ""
echo -e "${ITALIC}${BOLD}Hint:${NC} ${ITALIC}run the command line ${BOLD}cargo clean${NC} ${ITALIC}to remove compiled files and save ${ITALIC}${BOLD}${COMPILE_SIZE_ARR}${NC} ${ITALIC}of disk space."
