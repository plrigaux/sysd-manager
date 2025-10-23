import build_aux.build_common as bc
from build_aux.build_common import color
import os
import subprocess
import argparse

APP_IMAGE_DIR = "../AppImage"

APP_DIR = f"{APP_IMAGE_DIR}/SysDManager.AppDir"


def build_cargo():
    print(f"{color.CYAN}{color.BOLD}Compiling{color.END} ")

    bc.cmd_run(["cargo", "build", "--release", "--features", "default"])


def generating_translation_files():
    print(f"{color.CYAN}{color.BOLD}Generating translation files{color.END} ")

    bc.cmd_run(["cargo", "run", "-p", "transtools", "--", "packfiles"])


def create_appdir():
    print(f"{color.CYAN}{color.BOLD}Create AppDir{color.END} ")

    bc.cmd_run(["rm", "-fr", APP_IMAGE_DIR])
    bc.cmd_run(["mkdir", "-p", APP_DIR])
    # bc.cmd_run(["mkdir", "-p", f"{APP_DIR}/bin"])

    bc.cmd_run(["cp", "target/release/sysd-manager", f"{APP_DIR}/bin"])

    bc.cmd_run(
        [
            "install",
            "-Dm755",
            "./target/release/sysd-manager",
            "-t",
            f"{APP_DIR}/usr/bin",
        ]
    )
    bc.cmd_run(
        [
            "install",
            "-Dm644",
            "./data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg",
            "-t",
            APP_DIR,
        ]
    )
    bc.cmd_run(
        [
            "install",
            "-Dm644",
            "./data/schemas/io.github.plrigaux.sysd-manager.gschema.xml",
            "-t",
            f"{APP_DIR}/usr/share/glib-2.0/schemas",
        ]
    )
    bc.cmd_run(
        [
            "install",
            "-Dm644",
            "./target/loc/io.github.plrigaux.sysd-manager.desktop",
            "-t",
            f"{APP_DIR}/usr/share/applications",
        ]
    )
    
    bc.cmd_run(
        [
            "ln",
            "-s",
            "-v",
            f"usr/share/applications/io.github.plrigaux.sysd-manager.desktop",
            f"{APP_DIR}/io.github.plrigaux.sysd-manager.desktop",
        ]
    )
    bc.cmd_run(
        [
            "install",
            "-Dm644",
            "./target/loc/io.github.plrigaux.sysd-manager.metainfo.xml",
            "-T",
            f"{APP_DIR}/usr/share/metainfo/io.github.plrigaux.sysd-manager.appdata.xml",
        ]
    )
    bc.cmd_run(["cp", "-r", "./target/locale", f"{APP_DIR}/usr/share/"])

    print(f"{color.CYAN}{color.BOLD}Compile schemas{color.END} ")
    bc.cmd_run(["glib-compile-schemas", f"{APP_DIR}/usr/share/glib-2.0/schemas"])

    bc.cmd_run(["ln", "-s", "./usr/bin/sysd-manager", f"{APP_DIR}/AppRun"])

def app_image_file_name(version=None) -> str:
    if version is None:
        version = bc.get_version()
    file_name = f"SysDManager-{version}-x86_64.AppImage"
    return file_name

def make_appimage():
    os.environ["ARCH"] = "x86_64"
    bc.cmd_run(
        [
            "appimagetool-x86_64.AppImage",
            APP_DIR,
            f"{APP_IMAGE_DIR}/{app_image_file_name()}",
        ]
    )


import re


def pack_libs():
    print(f"{color.CYAN}{color.BOLD}Parse libs{color.END} ")

    """     os.chdir("..")

        curdir = os.getcwd()
        print(f"{color.BLUE}{color.BOLD}current working dir:{color.END} ", curdir) 
    """

    output = subprocess.check_output(["ldd", "target/release/sysd-manager"])
    result = {}

    output = output.decode("utf-8")

    valid = re.compile(r"([\S]+) => (\S+)")
    i = 0
    for row in output.split("\n"):

        m = valid.search(row)
        print(i)
        i += 1
        print(row, m)
        if m:
            result[m[1]] = m[2]

    # print(result)
    
#WARNING: Blacklisted file ld-linux-x86-64.so.2 found
#WARNING: Blacklisted file libm.so.6 found
#WARNING: Blacklisted file libz.so.1 found
#WARNING: Blacklisted file libfribidi.so.0 found
    exclude = {
        "libc",
        "libicudata",
        "libstdc++",
        # because essential on the disto
        #"libsystemd",

        #Blacklisted
        "ld-linux-x86-64",
        "/lib64/ld-linux-x86-64",
        "libm",
        "libresolv",
        "libEGL",
        "libGLdispatch",
        "libGLX",
        "libdrm",
        "libgbm",
        "libxcb",
        "libX11",
        "libX11-xcb",
        "libwayland-client",
        "libfontconfig",
        "libfreetype",
        "libharfbuzz",
        "libcom_err",
        "libexpat",
        "libgcc_s",
        "libz",
        "libfribidi",
        "libgmp",
    }
    for key, value in result.items():

        lib_name = key.split(".", 1)[0]
        # print(lib_name)
        if lib_name in exclude:
            print(f"{color.YELLOW}Excludes {lib_name}{color.END}")
        else:
            bc.cmd_run(["install", "-Dm644", value, "-t", f"{APP_DIR}/usr/lib"])


def build():
    print(f"color {color.RED}{color.BOLD}Creating an AppImage{color.END}")

    build_cargo()

    generating_translation_files()

    create_appdir()

    pack_libs()

    make_appimage()


def publish():
    version = bc.get_version()
    print(f"{color.CYAN}Publishing version {color.BOLD}{version}{color.END}")

    title = f"Release {version}"

    cmd = [
        "gh",
        "release",
        "create",
        version,
        "--title",
        title,
        "--notes",
        "See https://github.com/plrigaux/sysd-manager/blob/main/CHANGELOG.md",
        f"../AppImage/{app_image_file_name(version)}",
    ]

    print(cmd)

    bc.cmd_run(cmd)


def main():

    parser = argparse.ArgumentParser(
        description="Appimage builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "action",
        choices=[
            "build",
            "publish",
        ],
        help="action to perform",
    )

    args = parser.parse_args()

    os.chdir("..")

    curdir = os.getcwd()
    print(f"{color.BLUE}{color.BOLD}current working dir:{color.END} ", curdir)

    match args.action:
        case "build":
            build()
        case "publish":
            build()
            publish()
        case "publish_only":
            publish()
