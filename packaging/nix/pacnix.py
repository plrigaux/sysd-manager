import argparse
import os
import tempfile
import shutil
import build_aux.build_common as bc
from build_aux.build_common import color

TEMPLATE_DIR = "packaging/nix"
TEMPLATE_FILE = "package_template.nix"
DEFAULT_NIX = "default.nix"


def main():
    os.chdir("..")

    parser = argparse.ArgumentParser(
        description="Nix builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "action",
        choices=[
            "create",
        ],
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


def create():
    print(f"{color.BOLD}{color.CYAN}Create Nix Package{color.END}")
    # version
    version = bc.get_version_cargo()
    print(f"Version {color.BOLD}{color.CYAN}{version}{color.END}")

    secure_temp = tempfile.mkdtemp(prefix="sysd-manager-nix")
    print(secure_temp)

    shutil.copy(f'{TEMPLATE_DIR}/{TEMPLATE_FILE}', f'{secure_temp}/{DEFAULT_NIX}')
    
    with open(f"{secure_temp}/{DEFAULT_NIX}", "r") as pkgbuild_file:
        print("WRITE Version ")
        pkgbuild_text = pkgbuild_file.read()
        # set the version

    pkgbuild_text = pkgbuild_text.replace("{VERSION}", version)

    with open(f"{secure_temp}/{DEFAULT_NIX}", "w") as pkgbuild_file:
        print(f"WRITE VERSION {version}")
        
        pkgbuild_file.write(pkgbuild_text)


