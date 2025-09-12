
import build_aux.build_common as bc
from  build_aux.build_common import color
import argparse
import os

def main():
    os.chdir("..")

    parser = argparse.ArgumentParser(
        description="Tag git version",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "-f",
        "--force",
        action="store_true",
        help="force the tag",
    )

    parser.add_argument(
        "-d",
        "--allow_dirty",
        action="store_true",       
        help="allow not commited file",
        default=False,
    )

    args = parser.parse_args()

    if (not args.allow_dirty):
        bc.exit_if_dirty()


    #check git changes
    if (bc.cmd_run("[[ -z \"$(git status -s)\" ]]", shell=True, on_fail_exit=False) != 0):   
        bc.cmd_run(["git", "add", "CHANGELOG.md"], on_fail_exit=False)
        tag_name = bc.get_version_tag()
        bc.cmd_run(["git", "commit", "-m", f"change log {tag_name}"], on_fail_exit=False)
        bc.cmd_run(["git", "push"], on_fail_exit=False)

    bc.version(False, None, args.force)