import subprocess
import pprint
import git
import tomllib

class color:
    PURPLE = '\033[95m'
    CYAN = '\033[96m'
    DARKCYAN = '\033[36m'
    BLUE = '\033[94m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'
    END = '\033[0m'

def cmd_run(cmd : list, shell=False, cwd=None):
    

    if (cwd): 
        print(f"{color.GREEN}Change Working Dir to: {cwd}{color.END}")

    cmd_str = " ".join(cmd)
    print(f"{color.DARKCYAN}{cmd_str}{color.END}")
   
    ret = subprocess.run(cmd, shell=shell, cwd=cwd)
    try: 
        ret.check_returncode()
    except subprocess.CalledProcessError as err: 
        print (f"{color.RED}Called Process Error{color.END}")
        print (f"{color.YELLOW}{cmd_str}{color.END}")
        pprint.pp(err)


def clean_gschema():
    cmd_run(["rm", "-f", "~/.local/share/glib-2.0/schemas/io.github.plrigaux.sysd-manager.gschema.xml"])

def is_repo_dirty() -> bool:
    repo = git.Repo(".")
    return repo.is_dirty(untracked_files=True)

def toml() -> dict[str: any]:
    with open("Cargo.toml", "rb") as f:
        cargo_toml = tomllib.load(f)
    
    return cargo_toml