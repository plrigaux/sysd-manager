import build_aux.build_common as bc
from  build_aux.build_common import color
import os
import re
import argparse

AUR_DIR="../aur/sysd-manager"
PKGBUILD="PKGBUILD"
INSTALL_FILE="sysd-manager.install" 
AUR_OUT_DIR="../aur/sysd-manager"
TEMPLATE_DIR='packaging/aur'

def main():
    os.chdir("..")

    parser = argparse.ArgumentParser(
        description="Aur builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "action",
        choices=[
            "sum",
            "pkfile",
            "generate",
            "clean",
            "make",
            "genpush"
        ],
        help="action to perform",)

    args = parser.parse_args()

    match args.action:
        case "generate":
            gen_pkfile()
        case "sum":
            do_check_sum()
        case "pkfile":
            create_pkgbuild()
        case "clean":
            clean()
        case "make":
            make()
        case "genpush":
            generate_and_push()


def create_pkgbuild():
    #version
    version = bc.get_version()
    print(f"Version {color.BOLD}{color.CYAN}{version}{color.END}")

    #set commit tag
    tag_name = bc.get_version_tag()
    print(f"Tag name {color.BOLD}{color.CYAN}{tag_name}{color.END}")


    commit = bc.get_tag_commit(tag_name)

    print(f"Commit {color.BOLD}{color.CYAN}{commit}{color.END}")


    pkgbuild_text = ""

    with open(f'{TEMPLATE_DIR}/{PKGBUILD}', "r") as pkgbuild_file:
        pkgbuild_text = pkgbuild_file.read()

    #set the version
    pkgbuild_text = pkgbuild_text.replace("pkgver=\n", f"pkgver={version}\n")

    #put the commit label
    pkgbuild_text = pkgbuild_text.replace("_commit=\n", f"_commit={commit}\n")


    with open(f'{AUR_OUT_DIR}/{PKGBUILD}', "w") as pkgbuild_file:
        pkgbuild_file.write(pkgbuild_text)        
    
    
    #print(pkgbuild_text)
    #sums

    bc.cmd_run(["cp", f"{TEMPLATE_DIR}/{INSTALL_FILE}", f"{AUR_OUT_DIR}"])
    bc.cmd_run(["cp", f"CHANGELOG.md", f"{AUR_OUT_DIR}"])


def do_check_sum():
    cmd = ["makepkg", "-g"]  
    checksum = bc.cmd_run_str(cmd, cwd=f"{AUR_OUT_DIR}")

    print("OUT: ", checksum)

    pkgbuild_text = ""

    with open(f'{AUR_OUT_DIR}/{PKGBUILD}', "r") as pkgbuild_file:
        print("WRITE SUM on ")
        pkgbuild_text = pkgbuild_file.read()
        #set the version
    
    pkgbuild_text = pkgbuild_text.replace("sha256sums=()\n", checksum)

    with open(f'{AUR_OUT_DIR}/{PKGBUILD}', "w") as pkgbuild_file:
        print("WRITE SUM ")

        pkgbuild_file.write(pkgbuild_text)       


def generate_sourceinfo():
    cmd = ["makepkg", "--printsrcinfo"]  
    printsrcinfo = bc.cmd_run_str(cmd, cwd=f"{AUR_OUT_DIR}")

    with open(f'{AUR_OUT_DIR}/.SRCINFO', "w") as srcinfo_file:
        print("WRITE .SRCINFO")
        srcinfo_file.write(printsrcinfo)     


def gen_pkfile(): 
    create_pkgbuild()
    do_check_sum()
    generate_sourceinfo()


def generate_and_push(): 
    gen_pkfile()

    push()

def push():
    tag_name = bc.get_version_tag()

    print(f"Commit {color.BOLD}{color.CYAN}{tag_name}{color.END}")

    bc.cmd_run(["git", "commit", "-a", "-m", f"\"{tag_name}\""], cwd=f"{AUR_OUT_DIR}")

    print(f"{color.BOLD}{color.CYAN}Push on AUR{color.END}")

    bc.cmd_run(["git", "push"], cwd=f"{AUR_OUT_DIR}")


def make(): 
    gen_pkfile()

    bc.cmd_run(["makepkg"], cwd=f"{AUR_OUT_DIR}")

def clean():
    list_dir = [
        "PKGBUILD",
        "src",
        "sysd-manager",
        "pkg",
        "*.zst",
        ".SRCINFO",
        f"{INSTALL_FILE}",
    ]

    for f in list_dir:
        print(f"{color.BOLD}Deleting{color.END} {color.YELLOW}{f}{color.END}")
        #x = " ".join(["rm", "-fr", f])
        bc.cmd_run(["rm", "-fr", f], cwd=f"{AUR_OUT_DIR}", shell=True)

