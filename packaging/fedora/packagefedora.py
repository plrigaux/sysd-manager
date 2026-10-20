import argparse
import shutil
from pathlib import Path

import build_aux.build_common as bc
from build_aux.build_common import color


def main():
    # os.chdir("..")
    bc.position_on_root()

    parser = argparse.ArgumentParser(
        description="Deb builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "action",
        choices=[
            "create",
            "package",
            "control",
            "rules",
            "copysource",
            "upload",
            "build",
            "generate",
            "logs",
            "cargologs",
            "cargo",
            "publish",
            "create_publish",
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
        case "package":
            build_package()
        case "generate":
            generate_files()
        case "publish":
            publish()
        case "create_publish":
            create_publish(release)


def create(release):
    print(f"{color.BOLD}{color.DARK_BLUE}Create Fedora Package{color.END}")
    cargo_build()
    generate_files()
    build_package()
    # upload_package(release)


def build_package():
    print(f"Build {color.BLUE}{color.BOLD}Rpm Package{color.END}")
    bc.cmd_run(
        [
            "cargo",
            "run",
            "-p",
            "sysd-manager-buildtool",
            "--",
            "rpm",
        ]
    )


def cargo_build():
    print(f"{color.BOLD}{color.BLUE}Generate Build & Package{color.END}")

    bc.cmd_run(
        [
            "cargo",
            "build",
            "--release",
            "-p",
            "sysd-manager-proxy",
        ],
    )

    bc.cmd_run(
        [
            "cargo",
            "build",
            "--release",
        ],
    )


def generate_files():
    bc.generate_files()


def publish():
    print(f"{color.BOLD}Uploading to Release{color.END}")

    dir_path = Path("target")

    version = bc.get_version_cargo()
    arch = "x86_64"
    file = dir_path / f"sysd-manager_{version}_{arch}.rpm"

    if file:
        print(f"Rpm file: {file}")

        bc.release_upload_file(file)
    else:
        print(f"{color.RED}No files found in the directory: {dir_path}.{color.RED}")


def create_publish(release):
    create(release)
    publish()


if __name__ == "__name__":
    main()
