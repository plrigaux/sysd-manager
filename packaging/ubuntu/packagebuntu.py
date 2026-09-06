import argparse
import shutil
from email.utils import formatdate
from pathlib import Path

import build_aux.build_common as bc
from build_aux.build_common import color

BUILD_DIR = "sysd-manager-deb"
TEMPLATE_DIR = "packaging/ubuntu"
DEFAULT_NIX = "default.nix"
PACKAGE_DIR = f"/tmp/{BUILD_DIR}"
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
        choices=[
            "create",
            "changelog",
            "control",
            "rules",
            "copysource",
            "upload",
            "build",
        ],
        help="action to perform",
    )

    parser.add_argument("-r", "--release", help="Set the package release", type=int)

    args = parser.parse_args()

    release = None
    if args.release:
        release = args.release

    if not isinstance(release, int):
        release = 1

    match args.action:
        case "create":
            create(release)
        case "changelog":
            write_changelog(release)
        case "control":
            write_control(release)
        case "rules":
            write_rules()
        case "copysource":
            copy_source()
        case "upload":
            upload_package()
        case "build":
            build_package()


def create(release):
    print(f"{color.BOLD}{color.DARK_ORANGE}Create Unbuntu Package{color.END}")
    set_up_dir()
    copy_source()
    vendor_dep()
    write_changelog(release)
    write_control(release)
    write_rules()
    build_package()
    upload_package(release)


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

    bc.cmd_run(
        ["tar", "-cJf", "vendor.tar.xz", "../vendor"],
        cwd=DEB_DIR,
        env={"XZ_OPT": str(-9)},
    )
    bc.cmd_run(["rm", "-r", "vendor"], cwd=PACKAGE_DIR)


def ubuntu_version(version_raw, release):
    # return f"{version_raw}-{release}ubuntu{release}"
    return f"{version_raw}-{release}"


def write_changelog(release=None):
    print(f"Write {color.BOLD}changelog{color.END} file")

    urgency = "medium"
    distribution = "resolute"
    package = "sysd-manager"

    if not isinstance(release, int):
        release = 1

    version = bc.get_version_cargo()
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


def write_control(release):
    version = bc.get_version_cargo()
    print(f"Write {color.BOLD}control{color.END} file. Version {version}")

    with open("packaging/ubuntu/control", "r") as pkgbuild_file:
        pkgbuild_text = pkgbuild_file.read()

    version = bc.get_version_cargo()

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

    content = "3.0 (native)"

    with open(format_file, "w") as config_file:
        print(f"Write {color.DARK_ORANGE}format{color.END}")
        config_file.write(content)


def copy_source():
    print(f"Copy {color.BOLD}source{color.END} files")

    deb_dir = Path(PACKAGE_DIR)

    if not deb_dir.exists():
        deb_dir.mkdir()

    dirs = [
        "src",
        "data",
        "po",
        "tiny_daemon",
        "transtools",
        "sysd-manager-proxy",
        "sysd-manager-translating",
        "sysd-manager-comcontroler",
        "sysd-manager-test-base",
        "sysd-manager-base",
        "tool",
        # "vendor",
    ]

    for dir in dirs:
        shutil.copytree(dir, f"{PACKAGE_DIR}/{dir}", dirs_exist_ok=True)

    files = ["Cargo.lock", "CHANGELOG.md", "README.md", "Cargo.toml", "build.rs"]
    for file in files:
        shutil.copy(file, PACKAGE_DIR)


def build_package():
    print(f"Build {color.BOLD}Package{color.END}")
    bc.cmd_run(
        [
            "dpkg-buildpackage",
            "-S",
            "-sa",
            "-d",
            "-nc",
            "-kplrigaux@gmail.com",
            BUILD_DIR,
        ],
        cwd=PACKAGE_DIR,
    )


def upload_package(release):
    print(
        f"Upload {color.BOLD}Package{color.END} to {color.BOLD}{color.DARK_ORANGE}PPA{color.END}"
    )
    version = bc.get_version_cargo()
    version = ubuntu_version(version, release)
    bc.cmd_run(
        [
            "dput",
            "ppa::plrigaux/ppa",
            f"sysd-manager_{version}_source.changes",
        ],
        cwd="/tmp",
    )
