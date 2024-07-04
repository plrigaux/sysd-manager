#!/bin/bash
set -e
RPM_VERSION="$(grep '^version = ' Cargo.toml)"
RPM_VERSION="${RPM_VERSION#version = \"}"
RPM_VERSION="${RPM_VERSION%\"}"

bold=$(tput bold)
normal=$(tput sgr0)

function usage {
    echo "${bold}error Usage:${normal} $0 copr-release|copr-testing|mock [x86_64|aarch64] [<fedora-release>]"
}

echo "Hello"

case "$1" in
copr-release)
    CMD=copr
    COPR_REPO=sysd-manager
    ;;

copr-testing)
    CMD=copr
    COPR_REPO=tools-testing
    ;;

mock)
    CMD=mock
    ;;
*)
    usage
    exit 1
    ;;
esac

case "$2" in
x86_64)
    ARCH=x86_64
    ;;
aarch64)
    ARCH=aarch64
    ;;
"")
    ARCH=$(arch)
    ;;
*)
    usage
    exit 1
    ;;
esac

case "$3" in
rawhide)
    VERSION_ID=$3
    ;;

[0-9][0-9])
    VERSION_ID=$3
    ;;
"")
    . /etc/os-release
    ;;
*)
    usage
    exit 1
    ;;
esac

rm -rf tmp
mkdir -p tmp/{SPECS,SOURCES,tmpdir}

echo "${bold}info Info:${normal} make source"
git archive main --format=tar --prefix=sysd-manager-${RPM_VERSION}/ --output=tmp/SOURCES/sysd-manager-${RPM_VERSION}.crate
cd tmp/SOURCES
echo "${bold}info Info:${normal} make specfile"
# rust2rpm fails on one host becuase of some unknown issue with TMPDIR
# using an empty directory works around the failure
TMPDIR=$PWD/tmp/tmpdir rust2rpm ./sysd-manager-${RPM_VERSION}.crate
mv *.spec ../SPECS
cd ../..

# use host's arch for srpm
MOCK_SRPM_ROOT=fedora-${VERSION_ID}-$(arch)
# use user's arch for rpm
MOCK_RPM_ROOT=fedora-${VERSION_ID}-${ARCH}

echo "${bold}info Info:${normal} build SRPM"
ls -l tmp
mock \
    --buildsrpm \
    --root ${MOCK_SRPM_ROOT} \
    --spec tmp/SPECS/rust-sysd-manager.spec \
    --sources tmp/SOURCES

echo "${bold}info Info:${normal} copy SRPM"
ls -l /var/lib/mock/${MOCK_SRPM_ROOT}/result
cp -v /var/lib/mock/${MOCK_SRPM_ROOT}/result/rust-sysd-manager-${RPM_VERSION}-*.src.rpm tmp

SRPM=tmp/rust-sysd-manager-${RPM_VERSION}-*.src.rpm

case "$CMD" in
copr)
    echo "${bold}info Info:${normal} copr build ${bold} $COPR_REPO${normal} of ${bold} ${SRPM}${normal} for ${normal} $ARCH${normal}"
    set -x
    copr-cli build -r ${MOCK_RPM_ROOT} ${COPR_REPO} ${SRPM}
    ;;

mock)
    echo "${bold}info Info:${normal} build RPM for ${bold} $ARCH${normal}"
    set -x
    mock \
        --rebuild \
        --root ${MOCK_RPM_ROOT} \
            tmp/rust-sysd-manager-${RPM_VERSION}-*.src.rpm
    ls -l /var/lib/mock/${MOCK_RPM_ROOT}/result
    cp -v /var/lib/mock/${MOCK_RPM_ROOT}/result/sysd-manager-${RPM_VERSION}*${ARCH}.rpm tmp
    ;;

*)
    echo "${bold}error Error:${normal} bad CMD value of ${bold} $CMD${normal}"
    ;;
esac