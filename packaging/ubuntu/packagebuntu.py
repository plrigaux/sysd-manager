import argparse
import os
import tempfile
import shutil
import build_aux.build_common as bc
from build_aux.build_common import color
import re
from pathlib import Path
from email.utils import formatdate

TEMPLATE_DIR = "packaging/nix"
TEMPLATE_FILE = "package_template.nix"
DEFAULT_NIX = "default.nix"
DEB_DIR = "/tmp/debian"


def main():
    #os.chdir("..")
    bc.position_on_root()

    parser = argparse.ArgumentParser(
        description="Nix builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "action",
        choices=[
            "create","changelog"
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
        case "changelog":
            write_changelog()


def create():
    print(f"{color.BOLD}{color.DARK_ORANGE}Create Unbuntu Package{color.END}")
    set_up_dir()
    vendor_dep()  
    write_changelog()

def set_up_dir():
    print("setup dir")

    deb_dir = Path(DEB_DIR)

    if deb_dir.exists():
        shutil.rmtree(deb_dir)
    else:
        deb_dir.mkdir()
    
def vendor_dep():
    print("Vendor all dependencies for a project locally")
    
    cargo_dir = Path(DEB_DIR) / ".cargo"

    cargo_dir.mkdir(parents = True)

    cargo_config = cargo_dir / "config.toml"

    config_content = """[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
"""
 
    with open(cargo_config, "w") as config_file:
        print(f"Write {color.DARK_ORANGE}Cargo config{color.END}")
        config_file.write(config_content)

    vendor = Path(DEB_DIR) / "vendor"
    bc.cmd_run(["cargo", "vendor", str(vendor)])

    
def write_changelog():
    print(f"Write {color.BOLD}changelog{color.END} file")

    urgency="medium"
    distribution = "resolute"
    package = "sysd-manager"
    version = bc.get_version_cargo()
    print(f"Version {color.BOLD}{color.DARK_ORANGE}{version}{color.END}")
      
    headerline = f"{package} ({version}) {distribution}; urgency={urgency}"

    rfc2822_date = formatdate()
    trailline = f"-- Pierre-Luc Rigaux <plrigaux@users.noreply.github.com>  {rfc2822_date}"

    content = headerline + "\n\n" + "  * See CHANGELOG.md" + "\n\n" + trailline
    
    with open(f"{DEB_DIR}/changelog", "w") as changelog_file:
        print(f"Write {color.DARK_ORANGE}Cargo config{color.END}")
        changelog_file.write(content)
