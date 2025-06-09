#!/bin/sh

BOLD='\033[1m'
ITALIC='\033[3m'
NC='\033[0m'
BBCYAN='\033[1;96m'

PROGRAM="${BBCYAN}SysD Manager${NC}"


echo -e Uninstalling $PROGRAM program
echo ""

echo Removing files
echo ""
sudo rm "/usr/bin/sysd-manager" 
sudo rm "/usr/share/applications/io.github.plrigaux.sysd-manager.desktop" 
sudo rm "/usr/share/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg"
sudo rm "/usr/share/glib-2.0/schemas/io.github.plrigaux.sysd-manager.gschema.xml" 
sudo find /usr/share/locale -name sysd-manager.mo -type f -delete
echo ""
echo -e Uninstallation of $PROGRAM completed.
echo -e We wish we had more time together.