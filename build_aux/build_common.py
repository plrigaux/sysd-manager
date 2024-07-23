import subprocess

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

def cmd_run(cmd : list, shell=False):
    
    cmd_str = " ".join(cmd)
    print(f"{color.DARKCYAN}{cmd_str}{color.END}")
    
    ret = subprocess.run(cmd, shell=shell)
    ret.check_returncode()

def clean_gschema():
    cmd_run(["rm", "-f", "~/.local/share/glib-2.0/schemas/io.github.plrigaux.sysd-manager.gschema.xml"])