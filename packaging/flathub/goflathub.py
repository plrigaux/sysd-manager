import argparse
import logging
import json
import oyaml as yaml
import os
import build_aux.build_common as bcommon
import build_aux.flatpak_cargo_generator as cargo_gen


color = bcommon.color

from os import walk

G_RPM_VERSION = None
G_SRPM_PATH = None
G_MOCK_SRPM_ROOT = None
G_MOCK_RPM_ROOT = None
SPEC_FILE_PATH = None
G_LOCAL_RPM_DIR = "tmp"
G_SOURCES_DIR = f"{G_LOCAL_RPM_DIR}/SOURCES"
G_SPECS_DIR = f"{G_LOCAL_RPM_DIR}/SPECS"
COPR_REPO = "sysd-manager"

APP_ID = "io.github.plrigaux.sysd-manager"
MANIFEST = f"{APP_ID}.yaml"
MANIFEST_LOC = "packaging/flathub"
# FLATHUB_DIR = "../flathub"
FLATHUB_DIR = "../io.github.plrigaux.sysd-manager"
FLATPACK_BUILD_DIR = "../flatpak_sysdm"
CARGO_LOCK = "Cargo.lock"
CARGO_SOURCES = "cargo-sources.json"


def main():

    os.chdir("..")

    parser = argparse.ArgumentParser(
        description="Flathub builder",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument(
        "action",
        choices=[
            "clean",
            "build",
            "run",
            "lint",
            "repair",
            "compose",
            "validate",
            "flathub",
            "generate",
            "copy",
            "cleanf",
            "clone",
            "deploy",
            "manifest",
            "tag",
        ],
        help="action to perform",
    )

    parser.add_argument("--logbus", action="store_true", help="log dbus message on run")
    parser.add_argument(
        "-g",
        "--from_git",
        action="store_true",
        help="make the flatpack manifest source come from git",
    )

    parser.add_argument(
        "-d",
        "--allow_dirty",
        action="store_true",
        dest="allow_dirty",
        help="allow not commited file",
        default=False,
    )
    args = parser.parse_args()

    match args.action:
        case "build":
            build(args.from_git)
        case "clean":
            clean()
        case "run":
            run(args.logbus)
        case "lint":
            lint()
        case "compose":
            compose()
        case "validate":
            validate()
        case "flathub":
            add_required_files()
        case "generate":
            generate()
        case "repair":
            repair()
        case "copy":
            set_required_files(args.from_git)
        case "manifest":
            set_manifest(args.from_git, FLATPACK_BUILD_DIR)
        case "cleanf":
            clean_flathub_dir()
        case "clone":
            clone_the_fork()
        case "deploy":
            deploy()
        case "tag":
            tag()
        case _:
            print("No actions performed")


def clean():
    list_dir = [
        "builddir",
        ".flatpak-builder",
        CARGO_SOURCES,
        "repo",
        "tmp",
        FLATPACK_BUILD_DIR,
    ]

    for dir in list_dir:
        print(f"{color.BOLD}Deleting{color.END} {color.YELLOW}{dir}{color.END}")
        bcommon.cmd_run(["rm", "-fr", dir])


def build(from_git: bool):

    set_required_files(from_git)

    add_flathub_repo()

    print(f"{color.BOLD}{color.CYAN}Building for flatpak{color.END}")

    # Info https://docs.flathub.org/docs/for-app-authors/submission/
    # flatpak run org.flatpak.Builder --force-clean --sandbox --user --install --install-deps-from=flathub --ccache --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=repo builddir <manifest>

    bcommon.cmd_run(
        [
            "flatpak",
            "run",
            "org.flatpak.Builder",
            "--force-clean",
            "--sandbox",
            "--user",
            "--install",
            "--install-deps-from=flathub",
            "--ccache",
            "--mirror-screenshots-url=https://dl.flathub.org/media",
            "--repo=repo",
            "builddir",
            MANIFEST,
        ],
        cwd=FLATPACK_BUILD_DIR,
    )


"""     cmd_run(
        [
            "flatpak-builder",
            "--force-clean",
            "--sandbox",
            "--user",
            "--install",
            "--install-deps-from=flathub",
            "--ccache",
            "--mirror-screenshots-url=https://dl.flathub.org/media",
            "--repo=repo",
            "builddir",
            MANIFEST,
        ],
        cwd=FLATHUB_DIR,
    ) """


def add_flathub_repo():
    print(f"{color.BOLD}{color.CYAN}Add the Flathub repo user-wide{color.END}")

    bcommon.cmd_run(
        [
            "flatpak",
            "remote-add",
            "--if-not-exists",
            "--user",
            "flathub",
            "https://dl.flathub.org/repo/flathub.flatpakrepo",
        ],
        cwd=FLATHUB_DIR,
    )


def repair():
    bcommon.cmd_run(["flatpak", "-v", "--user", "repair"])


def run(logbus=False, log="info"):
    print("Try to run the Flatpack")

    cmd = ["flatpak", "run", APP_ID]
    if logbus:
        cmd.insert(2, "--log-session-bus")

    env = {**os.environ, "RUST_LOG" : log}
    
    try:
        bcommon.cmd_run(cmd, env=env)
    except KeyboardInterrupt as ki:
        print("Program closed by Keyboard Interrupt")


def lint():
    print(f"{color.BOLD}{color.CYAN}Lint manifest{color.END}")
    bcommon.cmd_run(
        [
            "flatpak",
            "run",
            "--command=flatpak-builder-lint",
            "org.flatpak.Builder",
            "manifest",
            f"{MANIFEST_LOC}/{MANIFEST}",
        ]
    )

    print(f"{color.BOLD}{color.CYAN}Lint repo{color.END}")
    bcommon.cmd_run(
        [
            "flatpak",
            "run",
            "--command=flatpak-builder-lint",
            "org.flatpak.Builder",
            "repo",
            "repo",
        ],
        cwd=FLATPACK_BUILD_DIR,
    )


def compose():
    print(f"{color.BOLD}{color.CYAN}appstreamcli compose{color.END}")
    bcommon.cmd_run(["appstreamcli", "compose", "builddir/files"])


def validate():
    print(f"{color.BOLD}{color.CYAN}Validating {APP_ID}.metainfo.xml{color.END}")
    bcommon.cmd_run(
        [
            "flatpak",
            "run",
            "--command=flatpak-builder-lint",
            "org.flatpak.Builder",
            "appstream",
            f"data/metainfo/{APP_ID}.metainfo.xml",
        ]
    )

    lint()


def add_required_files():
    print(
        f"{color.BOLD}{color.CYAN}Add the required files for the submission{color.END}"
    )
    # https://docs.flathub.org/docs/for-app-authors/requirements/#required-files
    bcommon.cmd_run(["cp", "-v", MANIFEST, FLATHUB_DIR])


def generate(source_dir=FLATHUB_DIR):
    print(f"{color.BOLD}{color.CYAN}Generate cargo sources{color.END}")

    loglevel = logging.INFO
    logging.basicConfig(level=loglevel)
    git_tarballs = False

    toml = cargo_gen.load_toml(CARGO_LOCK)
    generated_toml = cargo_gen.generate_sources(toml, git_tarballs=git_tarballs)
    generated_sources = cargo_gen.asyncio.run(generated_toml)

    bcommon.cmd_run(["mkdir", "-p", source_dir])

    cargo_source_file = f"{source_dir}/{CARGO_SOURCES}"
    with open(cargo_source_file, "w") as out:
        json.dump(generated_sources, out, indent=4, sort_keys=False)
        print(f"{color.CYAN}New cargo sources{color.END} to {cargo_source_file}")


def diag():
    """flatpak remotes -d
    flatpak update -v --ostree-verbose
    flatpak list"""


def ln(file: str):

    bcommon.cmd_run(
        [
            "ln",
            "-svfn",
            f"../sysd-manager/{file}",
            f"{FLATPACK_BUILD_DIR}/{file}",
        ]
    )


def set_required_files(from_git: bool):
    # https://docs.flathub.org/docs/for-app-authors/requirements/#required-files
    print(f"{color.BOLD}Add required files for {color.CYAN}Flathub{color.END}")

    bcommon.cmd_run(["mkdir", "-p", FLATPACK_BUILD_DIR])

    generate(FLATPACK_BUILD_DIR)

    bcommon.cmd_run(
        [
            "cp",
           # "-u",
            "-r",
            "Cargo.toml",
            CARGO_LOCK,
            "src",
            "data",
            "build.rs",
            "screenshots",
            "po",
            "sysd-manager-translating",
            "sysd-manager-proxy",
            "sysd-manager-comcontroler",
            "transtools",
            "tiny_daemon",
            "sysd-manager-proxy",
            "sysd-manager-comcontroler",
            "sysd-manager-test-base",
            "sysd-manager-base",
            f"{FLATPACK_BUILD_DIR}",
        ]
    )

    """     ln("src")
    ln("data")
    ln("screenshots")
    ln("po")
    ln("sysd-manager-translating")
    ln("sysd-manager-proxy")
    ln("sysd-manager-comcontroler")
    ln("transtools")
    ln("tiny_daemon") """

    set_manifest(from_git, FLATPACK_BUILD_DIR)

    # print(f"{color.CYAN}Generate a .gitignore{color.END}")
    # ignored = ["/target/", "/.flatpak/", "/.flatpak-builder", "/repo", "/builddir"]
    # with open(f"{FLATHUB_DIR}/.gitignore", "w") as file:
    #    content = "\n".join(ignored)
    #    file.write(content)


def set_manifest(from_git: bool, out_dir):

    out_file = f"{out_dir}/{MANIFEST}"
    print(f"set manifest to file: {out_file}")
    manifest_obj = None

    with open(f"{MANIFEST_LOC}/{MANIFEST}", "r") as manifest:
        manifest_obj = yaml.safe_load(manifest)

    if from_git:
        print("set source from git repo")
        modules_obj_0 = manifest_obj["modules"][0]

        tag_name = bcommon.get_version_tag()

        commit = bcommon.get_tag_commit(tag_name)

        if not commit:
            print(f"{color.RED}commit not found for tag {tag_name}")
            exit(0)

        print(
            f"Set tag {color.CYAN}{tag_name}{color.END} and commit {color.CYAN}{commit}{color.END} to manifest"
        )

        modules_obj_0["sources"][0] = {
            "type": "git",
            "url": "https://github.com/plrigaux/sysd-manager.git",
            "tag": tag_name,
            "commit": commit,
        }

    else:
        print("set source from local")

    with open(out_file, "w") as file:
        yaml.dump(manifest_obj, file)


def clone_the_fork():

    print(f"{color.CYAN}Clone the Flathub repository fork{color.END}")
    bcommon.cmd_run(
        [
            "git",
            "clone",
            "--branch=new-pr",
            "git@github.com:plrigaux/flathub.git",
        ],
        cwd="..",
    )


def clean_flathub_dir():

    file_list = []
    for dirpath, dirnames, filenames in walk(FLATHUB_DIR):
        dirnames = [x for x in dirnames if x != ".git"]
        file_list.extend(dirnames)
        file_list.extend(filenames)
        print(f"Path {dirpath}")
        break

    files_to_clean = []
    for file_or_dir in file_list:
        file_or_dir = f"{FLATHUB_DIR}/{file_or_dir}"
        files_to_clean.append(file_or_dir)

    print(files_to_clean)

    for dir in files_to_clean:
        print(f"{color.BOLD}Deleting{color.END} {dir}")
        bcommon.cmd_run(["rm", "-fr", dir])


def deploy():
    print(f"{color.BOLD}Set files for deployment on {color.CYAN}Flathub{color.END}")

    generate(FLATHUB_DIR)

    bcommon.cmd_run(
        ["git", "config", "pull.rebase", "true"], cwd=FLATHUB_DIR, on_fail_exit=False
    )
    bcommon.cmd_run(["git", "pull"], cwd=FLATHUB_DIR, on_fail_exit=False)

    set_manifest(True, FLATHUB_DIR)

    version = bcommon.get_version_tag()

    bcommon.cmd_run(
        ["git", "commit", "-a", "-m", version], cwd=FLATHUB_DIR, on_fail_exit=False
    )

    bcommon.cmd_run(["git", "push"], cwd=FLATHUB_DIR)


def tag():
    commit = bcommon.get_tag_commit(None)
    print("commit", commit)
