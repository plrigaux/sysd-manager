import build_aux.build_common as bc
from build_aux.build_common import color
import os

APP_IMAGE_DIR = "../AppImage"

APP_DIR = f"{APP_IMAGE_DIR}/SysD-Manager.AppDir"


def build_cargo():
    print(f"{color.CYAN}{color.BOLD}Compiling{color.END} ")

    bc.cmd_run(["cargo", "build", "--release", "--features", "default"])


def generating_translation_files():
    print(f"{color.CYAN}{color.BOLD}Generating translation files{color.END} ")

    bc.cmd_run(["cargo", "run", "-p", "transtools", "--", "packfiles"])


def generating_translation_files():
    print(f"{color.CYAN}{color.BOLD}Create AppDir{color.END} ")

    bc.cmd_run(["rm", "-fr", APP_DIR])
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
            APP_DIR,
        ]
    )
    bc.cmd_run(
        [
            "install",
            "-Dm644",
            "./target/loc/io.github.plrigaux.sysd-manager.metainfo.xml",
            "-t",
            f"{APP_DIR}/usr/share/metainfo",
        ]
    )
    bc.cmd_run(["cp", "-r", "./target/locale", f"{APP_DIR}/usr/share/"])

    print(f"{color.CYAN}{color.BOLD}Compile schemas{color.END} ")
    bc.cmd_run(["glib-compile-schemas", f"{APP_DIR}/usr/share/glib-2.0/schemas"])

    bc.cmd_run(["ln", "-s", "./usr/bin/sysd-manager", f"{APP_DIR}/AppRun"])

    os.environ["ARCH"] = "x86_64"
    bc.cmd_run(
        [
            "appimagetool-x86_64.AppImage",
            APP_DIR,
            f"{APP_IMAGE_DIR}/SysD-Manager-x86_64.AppImage",
        ]
    )


def main():

    print(f"color {color.RED}{color.BOLD}Creating an AppImage{color.END}")

    os.chdir("..")

    curdir = os.getcwd()
    print(f"{color.BLUE}{color.BOLD}current working dir:{color.END} ", curdir)

    print(f"{color.BLUE}current working dir:{color.END} ", curdir)

    build_cargo()

    generating_translation_files()
