import argparse
import os
import tempfile
import shutil
import build_aux.build_common as bc
from build_aux.build_common import color

TEMPLATE_DIR = "packaging/nix"
TEMPLATE_FILE = "package_template.nix"



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

    shutil.copy(f'{TEMPLATE_DIR}/{TEMPLATE_FILE}', secure_temp)

    
    with open(f"{secure_temp}/{TEMPLATE_FILE}", "r") as pkgbuild_file:
        print("WRITE Version ")
        pkgbuild_text = pkgbuild_file.read()
        # set the version

    pkgbuild_text = pkgbuild_text.replace("{VERSION}", version)

    with open(f"{secure_temp}/{TEMPLATE_FILE}", "w") as pkgbuild_file:
        print(f"WRITE VERSION {version}")
        
        pkgbuild_file.write(pkgbuild_text)


def do_check_sum():

    cmd = ["makepkg", "--geninteg", "--clean", "--cleanbuild"]
    checksum = bc.cmd_run_str(cmd, cwd=f"{AUR_OUT_DIR}")

    # checksum = checksum.replace("'","\"")
    print("OUT: ", checksum)

    pkgbuild_text = ""

    with open(f"{AUR_OUT_DIR}/{PKGBUILD}", "r") as pkgbuild_file:
        print("WRITE SUM on ")
        pkgbuild_text = pkgbuild_file.read()
        # set the version

    pkgbuild_text = pkgbuild_text.replace("sha256sums=()\n", checksum)

    with open(f"{AUR_OUT_DIR}/{PKGBUILD}", "w") as pkgbuild_file:
        print("WRITE SUM ")

        pkgbuild_file.write(pkgbuild_text)


def check_package():
    cmd = ["namcap", "--info", "PKGBUILD"]
    bc.cmd_run_str(cmd, cwd=f"{AUR_OUT_DIR}")


def generate_sourceinfo():
    cmd = ["makepkg", "--printsrcinfo"]
    printsrcinfo = bc.cmd_run_str(cmd, cwd=f"{AUR_OUT_DIR}")

    with open(f"{AUR_OUT_DIR}/.SRCINFO", "w") as srcinfo_file:
        print("WRITE .SRCINFO")
        srcinfo_file.write(printsrcinfo)


def install():
    cmd = ["makepkg", "--install"]
    printsrcinfo = bc.cmd_run_str(cmd, cwd=f"{AUR_OUT_DIR}")

    with open(f"{AUR_OUT_DIR}/.SRCINFO", "w") as srcinfo_file:
        print("WRITE .SRCINFO")
        srcinfo_file.write(printsrcinfo)


def gen_pkfile(release=None):    
    create_pkgbuild(release)
    do_check_sum()
    generate_sourceinfo()


def generate_and_push(release=None):
    gen_pkfile(release)

    push()


def push():
    tag_name = bc.get_version_tag()

    print(f"Commit {color.BOLD}{color.CYAN}{tag_name}{color.END}")

    bc.cmd_run(["git", "commit", "-a", "-m", f'"{tag_name}"'], cwd=f"{AUR_OUT_DIR}")

    print(f"{color.BOLD}{color.CYAN}Push on AUR{color.END}")

    bc.cmd_run(["git", "push"], cwd=f"{AUR_OUT_DIR}")


def make(release=None):
    gen_pkfile(release)

    bc.cmd_run(["makepkg"], cwd=f"{AUR_OUT_DIR}")


def clean():
    list_dir = [
        "*",
    ]

    for f in list_dir:
        print(f"{color.BOLD}Deleting{color.END} {color.YELLOW}{f}{color.END}")
        # x = " ".join(["rm", "-fr", f])
        bc.cmd_run(["rm", "-fr", f], cwd=f"{AUR_OUT_DIR}", shell=True)

def local():
    print(f"{color.BOLD}{color.CYAN}Local install{color.END}")

    bc.cmd_run(["makepkg", "-si"], cwd=f"{AUR_OUT_DIR}")


