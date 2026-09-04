import argparse
import shutil
from email.utils import formatdate
from pathlib import Path

import build_aux.build_common as bc
from build_aux.build_common import color

TEMPLATE_DIR = "packaging/ubuntu"
DEFAULT_NIX = "default.nix"
PACKAGE_DIR = "/tmp/sysd-manager-deb"
DEB_DIR = f"{PACKAGE_DIR}/debian"


def main():
    # os.chdir("..")
    bc.position_on_root()

    parser = argparse.ArgumentParser(
        description="Nix builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "action",
        choices=["create", "changelog", "control", "rules", "copysource"],
        help="action to perform",
    )

    parser.add_argument("-r", "--release", help="Set the package release", type=int)

    args = parser.parse_args()

    release = None
    if args.release:
        release = args.release

    match args.action:
        case "create":
            create()
        case "changelog":
            write_changelog(release)
        case "control":
            write_control(release)
        case "rules":
            write_rules()
        case "copysource":
            copy_source()


def create():
    print(f"{color.BOLD}{color.DARK_ORANGE}Create Unbuntu Package{color.END}")
    set_up_dir()
    copy_source()
    vendor_dep()
    write_changelog()
    write_control()
    write_rules()


def set_up_dir():
    print(f"Setup dir {PACKAGE_DIR}")

    pkd_dir = Path(PACKAGE_DIR)

    if pkd_dir.exists():
        shutil.rmtree(pkd_dir)
    else:
        pkd_dir.mkdir()

    deb_dir = Path(DEB_DIR)

    deb_dir.mkdir(parents=True, exist_ok=True)


def vendor_dep():
    print("Vendor all dependencies for a project locally")

    cargo_dir = Path(PACKAGE_DIR) / ".cargo"

    cargo_dir.mkdir(parents=True, exist_ok=True)

    cargo_config = cargo_dir / "config.toml"

    config_content = """[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
"""

    with open(cargo_config, "w") as config_file:
        print(f"Write {color.DARK_ORANGE}Cargo config{color.END}")
        config_file.write(config_content)

    vendor_dir = Path(PACKAGE_DIR) / "vendor"
    bc.cmd_run(["cargo", "vendor", "-q", str(vendor_dir)])


def ubuntu_version(version_raw, release):
    return f"{version_raw}-{release}ubuntu{release}"


def write_changelog(release=None):
    print(f"Write {color.BOLD}changelog{color.END} file")

    urgency = "medium"
    distribution = "resolute"
    package = "sysd-manager"
    version = bc.get_version_cargo()

    if not isinstance(release, int):
        release = 1

    version = ubuntu_version(version, release)
    print(f"Version {color.BOLD}{color.DARK_ORANGE}{version}{color.END}")

    headerline = f"{package} ({version}) {distribution}; urgency={urgency}"

    rfc2822_date = formatdate()
    trailline = (
        f" -- Pierre-Luc Rigaux <plrigaux@users.noreply.github.com>  {rfc2822_date}"
    )

    content = headerline + "\n\n" + "   * See CHANGELOG.md" + "\n\n" + trailline

    with open(f"{DEB_DIR}/changelog", "w") as changelog_file:
        print(f"Write {color.DARK_ORANGE}Cargo config{color.END}")
        changelog_file.write(content)


def write_control(release=1):
    version = bc.get_version_cargo()
    print(f"Write {color.BOLD}control{color.END} file. Version {version}")

    with open("packaging/ubuntu/control", "r") as pkgbuild_file:
        pkgbuild_text = pkgbuild_file.read()

    version = bc.get_version_cargo()
    if not isinstance(release, int):
        release = 1

    version = ubuntu_version(version, release)
    pkgbuild_text = pkgbuild_text.replace("{VERSION}", version)

    with open(f"{DEB_DIR}/control", "w") as pkgbuild_file:
        pkgbuild_file.write(pkgbuild_text)


def write_rules():
    print(f"Write {color.BOLD}rules{color.END} file")
    bc.cmd_run(["cp", f"{TEMPLATE_DIR}/rules", DEB_DIR])

    source_dir = Path(DEB_DIR) / "source"

    source_dir.mkdir(parents=True, exist_ok=True)

    format_file = source_dir / "format"

    content = "3.0 (quilt)"

    with open(format_file, "w") as config_file:
        print(f"Write {color.DARK_ORANGE}format{color.END}")
        config_file.write(content)


def copy_source():
    print(f"Copy {color.BOLD}source{color.END} files")

    deb_dir = Path(PACKAGE_DIR)

    if not deb_dir.exists():
        deb_dir.mkdir()

    shutil.copytree("src", f"{PACKAGE_DIR}/src", dirs_exist_ok=True)
    shutil.copytree(
        "data",
        f"{PACKAGE_DIR}/data",
        ignore=shutil.ignore_patterns("*.mo"),
        dirs_exist_ok=True,
    )
    shutil.copytree(
        "sysd-manager-proxy", f"{PACKAGE_DIR}/sysd-manger-proxy", dirs_exist_ok=True
    )
    shutil.copytree(
        "sysd-manager-base", f"{PACKAGE_DIR}/sysd_manager-base", dirs_exist_ok=True
    )
    shutil.copytree(
        "sysd-manager-comcontroler",
        f"{PACKAGE_DIR}/sysd-manager-comcontroler",
        dirs_exist_ok=True,
    )
    shutil.copytree(
        "sysd-manager-proxy", f"{PACKAGE_DIR}/sysd-manger-proxy", dirs_exist_ok=True
    )

    shutil.copy("Cargo.toml", f"{PACKAGE_DIR}/Cargo.toml")
    shutil.copy("Cargo.lock", f"{PACKAGE_DIR}/Cargo.lock")
    shutil.copy("CHANGELOG.md", f"{PACKAGE_DIR}/CHANGELOG.md")
    shutil.copy("README.md", f"{PACKAGE_DIR}/README.md")
