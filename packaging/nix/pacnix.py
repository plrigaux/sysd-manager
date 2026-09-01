import argparse
import os
import tempfile
import shutil
import build_aux.build_common as bc
from build_aux.build_common import color
import subprocess
import re

TEMPLATE_DIR = "packaging/nix"
TEMPLATE_FILE = "package_template.nix"
DEFAULT_NIX = "default.nix"


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
            "create","path"
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

    secure_temp = tempfile.mkdtemp(prefix="sysd-manager-nix_")
    print(secure_temp)

    shutil.copy(f'{TEMPLATE_DIR}/{TEMPLATE_FILE}', f'{secure_temp}/{DEFAULT_NIX}')
    
    with open(f"{secure_temp}/{DEFAULT_NIX}", "r") as pkgbuild_file:
        pkgbuild_text = pkgbuild_file.read()
        # set the version

    pkgbuild_text = pkgbuild_text.replace("{VERSION}", version)

    with open(f"{secure_temp}/{DEFAULT_NIX}", "w") as pkgbuild_file:
        print(f"Write VERSION {color.YELLOW}{version}{color.END}")
        pkgbuild_file.write(pkgbuild_text)

    file = f"{secure_temp}/{DEFAULT_NIX}"
    replace_in_file(file, "{VERSION}", version)
    
    print(f"{color.BOLD}Build to find SHA{color.END}")
    out_lines = nix_build(secure_temp)

    pattern = r"got:\s+(sha256-\S+)"

    sha = None
    for line in out_lines:
        match = re.search(pattern, line)
        if match:
            sha = match.group(1)
            print(f"git sha: {color.BLUE}{sha}{color.END}")
            break

    if not sha:        
        print(f"{color.RED}Sha not found{color.END}")
        return

    print(f"Write SHA {color.YELLOW}{sha}{color.END}")    
    replace_in_file(file,
        "hash = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        f"hash = \"{sha}")
    
    out_lines = nix_build(secure_temp)
    
    sha = None
    for line in out_lines:
        match = re.search(pattern, line)
        if match:
            sha = match.group(1)
            print(f"cargo sha: {color.BLUE}{sha}{color.END}")
            break

    if not sha:        
        print(f"{color.RED}Sha not found{color.END}")
        return

    print(f"Write Cargo SHA {color.YELLOW}{sha}{color.END}")
    replace_in_file(file,
        "cargoHash = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        f"cargoHash = \"{sha}")

    
    print(f"{color.BOLD}Uploading to Release{color.END}")
    bc.publish_upload(file)
    
def replace_in_file(path, pattern, replace):
    
    with open(path, "r") as pkgbuild_file:
        pkgbuild_text = pkgbuild_file.read()

    pkgbuild_text = pkgbuild_text.replace(pattern, replace)

    with open(path, "w") as pkgbuild_file:
        pkgbuild_file.write(pkgbuild_text)
        
def nix_build(dir):
        
    command = ["nix-build", "-E", 'with import <nixpkgs> {}; callPackage ./default.nix {}']
    
    print(f"{color.GREEN}Change Working Dir to: {dir}{color.END}")
    cmd_str = " ".join(command)

    print(f"{color.DARKCYAN}{cmd_str}{color.END}")
        
    try:
        proc = subprocess.Popen(
            command, 
            #capture_output=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,  
            text=True,            # Returns output as a string instead of bytes
            bufsize=1,
            cwd=dir,
        )

        captured_output = []
    
        # Read line-by-line until EOF
        for line in proc.stdout:
            #line = line.strip()
            #print(line)              # Display in real-time
            captured_output.append(line)  # Capture for later use
    
        proc.wait()
        return captured_output         

    except subprocess.CalledProcessError as e:
        print(f"Command failed with exit code {e.returncode}")
        print("Error output:\n", e.stderr)
    
