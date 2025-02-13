import subprocess
import pprint
import git
import tomllib
from typing import Optional
from typing import Optional


class color:
    PURPLE = "\033[95m"
    CYAN = "\033[96m"
    DARKCYAN = "\033[36m"
    BLUE = "\033[94m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    RED = "\033[91m"
    BOLD = "\033[1m"
    UNDERLINE = "\033[4m"
    END = "\033[0m"


def cmd_run(cmd: list, shell=False, cwd=None, on_fail_exit=True, verbose=True) -> int:

    if cwd:
        print(f"{color.GREEN}Change Working Dir to: {cwd}{color.END}")

    if verbose:
        cmd_str = ""
        if isinstance(cmd, list):
            cmd_str = " ".join(cmd)
        else:
            cmd_str = cmd

        print(f"{color.DARKCYAN}{cmd_str}{color.END}")


    cmd1 = ""
    if shell:
        cmd1 = " ".join(cmd)
    else:
        cmd1 = cmd

    ret = subprocess.run(cmd1, shell=shell, cwd=cwd)

    try:
        ret.check_returncode()
    except subprocess.CalledProcessError as err:

        if on_fail_exit:
            print(f"{color.RED}Called Process Error! code({ret.returncode}){color.END}")
            cmd_str = " ".join(cmd)
            print(f"{color.YELLOW}{cmd_str}{color.END}")
            pprint.pp(err)
            print(f"{color.RED}Exit program{color.END}")
            exit(ret.returncode)

    return ret.returncode


def cmd_run_str(
    cmd: list, shell=False, cwd=None, on_fail_exit=True, verbose=True
) -> str:

    if cwd:
        print(f"{color.GREEN}Change Working Dir to: {cwd}{color.END}")

    if verbose:
        cmd_str = " ".join(cmd)
        print(f"{color.DARKCYAN}{cmd_str}{color.END}")

    try:
        out = subprocess.check_output(cmd, shell=shell, cwd=cwd)
        out = out.decode("utf-8")
        return out
    except subprocess.CalledProcessError as err:

        if on_fail_exit:
            print(f"{color.RED}Called Process Error! {color.END}")
            cmd_str = " ".join(cmd)
            print(f"{color.YELLOW}{cmd_str}{color.END}")
            pprint.pp(err)
            print(f"{color.RED}Exit program{color.END}")
            exit(1)

    return ""


def clean_gschema():
    cmd_run(
        [
            "rm",
            "-f",
            "~/.local/share/glib-2.0/schemas/io.github.plrigaux.sysd-manager.gschema.xml",
        ]
    )

def exit_if_dirty(allow_dirty = False):    
    if is_repo_dirty() and not allow_dirty:
        print(f"The repo is dirty {color.BOLD}Exit{color.END}")
        exit(101)

def is_repo_dirty() -> bool:
    repo = git.Repo(".")
    return repo.is_dirty(untracked_files=True)


def toml() -> dict:
    with open("Cargo.toml", "rb") as f:
        cargo_toml = tomllib.load(f)

    return cargo_toml


def get_version() -> str:
    cargo_toml = toml()

    version = cargo_toml["package"]["version"]

    return version

def get_version_tag() -> str:
    version = get_version() 
    tag_name = f"v{version}"

    return tag_name


def get_tag_commit(tag_label: Optional[str]) -> Optional[str]:

    if not tag_label:
        tag_label = get_version_tag()
        print("tag", None)


    repo = git.Repo(".")

    """     out1 = None

    for t in repo.tags:
        if tag_label == str(t):
            out1 = str(t.commit)
            print(out1) """

    out2 = cmd_run_str(
        [
            "git",
            "rev-parse",
            tag_label,
        ]
    )


    out2 = out2[:-1]

    print(f"tag {tag_label} commit {out2}")

    """     if out2.find(out1):
        print("get tag error")
        print("out1", out1)
        print("out2", out2)
        exit(1) """

    return out2


def version(allow_dirty: bool, message: str, force: bool):
    print(f"{color.CYAN}Create as git tag and push it{color.END}")

    exit_if_dirty(allow_dirty)

    tag_name = get_version_tag()

    print(f"Program version {color.BOLD}{version}{color.END}")
    print(f"Git tag {color.BOLD}{color.YELLOW}{tag_name}{color.END}")

    if not message:
        print(f'Message needed (-m "a message ...")')
        message = f'version {tag_name}'
        print(f'Message supplied (-m "{message}")')

    git_tag = ["git", "tag", "-m", f'"{message}"', tag_name]
    git_push = ["git", "push", "origin", "tag", tag_name]

    if force:
        git_tag.insert(2, "-f")
        git_push.insert(2, "-f")

    cmd_run(git_tag)
    cmd_run(git_push)


def change_log():
    print(f"{color.BOLD}Generate {color.CYAN}CHANGELOG.md{color.END} file{color.END}")

    import xml.etree.ElementTree as ET
    #https://keepachangelog.com/en/1.1.0/
    tree = ET.parse('./data/metainfo/io.github.plrigaux.sysd-manager.metainfo.xml')
    root = tree.getroot()
    
    
    out = """# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

"""
    change_type = set(['Added', 'Changed', 'Deprecated', 'Removed' ,'Fixed' ,'Security'])

    for release in root.iter('release'):
        version = release.get('version')
        date = release.get('date')
        
        out += f"## [{version}] - {date}\n\n"
        
        for description in release.iter('description'):
            for sub in description.iter('*'):
                if sub.tag == "p":
                    if sub.text in change_type:
                        out += f"### {sub.text}\n\n"
                    else:
                        out += sub.text + "\n\n"
                    
                if sub.tag == "li":
                   out += "- " + sub.text + "\n"
            
            out += "\n"
    
    print (out)        
    
    with open("CHANGELOG.md", "w") as f:
        f.write(out)