import argparse
import os
import shutil

import build_aux.build_common as bc
from build_aux.build_common import color

DEB_DIR = "../deb/sysd-manager"
PKGBUILD = "PKGBUILD"
# INSTALL_FILE="sysd-manager.install"
AUR_OUT_DIR = "../aur/sysd-manager"
TEMPLATE_DIR = "packaging/aur"


def main():
    os.chdir("..")

    parser = argparse.ArgumentParser(
        description="Aur builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "action",
        choices=[
            "generate",
            "dodeb",
            "build",
            "make",
            "control",
            "install",
            "compile",
            "publish",
            "just_publish",
        ],
        help="action to perform",
    )

    parser.add_argument("-r", "--release", help="Set the package release", type=int)

    args = parser.parse_args()

    release = None
    if args.release:
        release = args.release

    match args.action:
        case "compile":
            compile()
        case "deb":
            build_deb()
        case "dodeb":
            create_deb(release)
        case "control":
            debian_control()
        case "build":
            build_deb()
        case "generate":
            generate()
        case "publish":
            publish()
        case "just_publish":
            just_publish()


def generate():
    compile()
    create_deb()
    debian_control()
    build_deb()


def compile():
    version = bc.get_version()
    print(f"Compile {color.BOLD}{color.CYAN}Version {version}{color.END}")

    bc.cmd_run(
        [
            "cargo",
            "build",
            "--manifest-path",
            "sysd-manager-proxy/Cargo.toml",
            "--release",
        ]
    )
    bc.cmd_run(["cargo", "build", "--release"])
    bc.cmd_run(["cargo", "run", "-p", "transtools", "--", "packfiles"])


def install_to(file, dir):
    os.makedirs(dir, exist_ok=True)
    shutil.copy(file, dir)


def install_tree(source, dir):
    os.makedirs(dir, exist_ok=True)
    shutil.copytree(source, dir, dirs_exist_ok=True)


def create_deb(release=None):
    print(f"Generate {color.BOLD}{color.CYAN}directory structure{color.END}")
    shutil.rmtree(DEB_DIR, ignore_errors=True)
    os.makedirs(DEB_DIR, exist_ok=True)
    bin = f"{DEB_DIR}/usr/bin"

    os.makedirs(bin, exist_ok=True)
    shutil.copy("target/release/sysd-manager", bin)
    shutil.copy("target/release/sysd-manager-proxy", bin)

    install_to(
        "./data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg",
        f"{DEB_DIR}/usr/share/icons/hicolor/scalable/apps",
    )
    install_to(
        "./data/schemas/io.github.plrigaux.sysd-manager.gschema.xml",
        f"{DEB_DIR}/usr/share/glib-2.0/schemas",
    )
    install_to(
        "./target/loc/io.github.plrigaux.sysd-manager.desktop",
        f"{DEB_DIR}/usr/share/applications",
    )
    install_to(
        "./target/loc/io.github.plrigaux.sysd-manager.metainfo.xml",
        f"{DEB_DIR}/usr/share/metainfo",
    )

    install_tree("./target/locale", f"{DEB_DIR}/usr/share/locale")

    install_to(
        "./sysd-manager-proxy/data/io.github.plrigaux.SysDManager.conf",
        f"{DEB_DIR}/usr/share/dbus-1/system.d",
    )
    bc.cmd_run(
        [
            "sed",
            "-i",
            "-e",
            "s/{BUS_NAME}/io.github.plrigaux.SysDManager/",
            "-e",
            "s/{DESTINATION}/io.github.plrigaux.SysDManager/",
            "-e",
            "s/{ENVIRONMENT}//",
            "-e",
            "s/{INTERFACE}/io.github.plrigaux.SysDManager/",
            f"{DEB_DIR}/usr/share/dbus-1/system.d/io.github.plrigaux.SysDManager.conf",
        ]
    )
    install_to(
        "./sysd-manager-proxy/data/io.github.plrigaux.SysDManager.policy",
        f"{DEB_DIR}/usr/share/polkit-1/actions",
    )
    install_to(
        "./sysd-manager-proxy/data/sysd-manager-proxy.service",
        f"{DEB_DIR}/usr/lib/systemd/system",
    )

    bc.cmd_run(
        [
            "sed",
            "-i",
            "-e",
            "s/{BUS_NAME}/io.github.plrigaux.SysDManager/",
            "-e",
            "s/{DESTINATION}/io.github.plrigaux.SysDManager/",
            "-e",
            "s/{ENVIRONMENT}//",
            "-e",
            "s/{EXECUTABLE}/\\/usr\\/bin\\/sysd-manager-proxy/",
            "-e",
            "s/{INTERFACE}/io.github.plrigaux.SysDManager/",
            "-e",
            "s/{SERVICE_ID}/sysd-manager-proxy/",
            f"{DEB_DIR}/usr/lib/systemd/system/sysd-manager-proxy.service",
        ]
    )


def debian_control():
    print(f"Generate {color.BOLD}{color.CYAN}control file{color.END}")

    os.makedirs(f"{DEB_DIR}/DEBIAN", exist_ok=True)

    cargo_toml = bc.toml()

    version = cargo_toml["package"]["version"]
    description = cargo_toml["package"]["description"]
    # print(cargo_toml)
    author = cargo_toml["workspace"]["package"]["authors"][0]
    repository = cargo_toml["workspace"]["package"]["repository"]

    package_info = {
        "Package": "sysd-manager",
        "Version": version,
        "Maintainer": author,
        "Architecture": "amd64",
        "Description": description,
        "Homepage": repository,
        "Depends": "libgtk-4-1 (>=4.20), libadwaita-1-0 (>=1.8), libsystemd0 (>=257), libgtksourceview-5-0 (>=5.16)",
    }

    text = ""

    for key, value in package_info.items():
        text += f"{key}: {value}\n"

    with open(f"{DEB_DIR}/DEBIAN/control", "w") as pkgbuild_file:
        print("WRITE Control ")

        pkgbuild_file.write(text)


def build_deb():
    print(f"Generate {color.BOLD}{color.CYAN}sysd-manager.deb{color.END}")
    bc.cmd_run(
        ["dpkg-deb", "--build", "--root-owner-group", "sysd-manager"],
        cwd=f"{DEB_DIR}/..",
    )


def publish():
    generate()
    just_publish()


def just_publish():
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
        f"{DEB_DIR}/../sysd-manager.deb",
    ]

    print(cmd)

    bc.cmd_run(cmd)
