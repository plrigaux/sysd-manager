{ lib,
  pkgs,
  rustPlatform,
  pkg-config,
  gtk4,
  libadwaita,
  gtksourceview5,
  systemd,
  gettext,
  glib,
  gsettings-desktop-schemas,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "sysd-manager";
  version = "2.20.8";

  src = fetchFromGitHub {
    owner = "plrigaux";
    repo = "sysd-manager";
    tag = "v${finalAttrs.version}"; #or commit
    hash = "sha256-GYMLV4hffaIbqUp1b5ERo2QQqiKRlHe9oXfq+wNH/hM=";
  };
  
  #cargoHash = "sha256-CsmcXlkOec/KJ59Ng7MyGsfjWQ80YyV6MztRFULmvDA=";
  cargoLock.lockFile = ./Cargo.lock;


  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    gtk4
    libadwaita
    gtksourceview5
    systemd
    gettext
    glib
    gsettings-desktop-schemas
  ];

  doCheck = false;

  cargoBuildFlags = [
    "--release"
    "--features"
    "default"
  ];

  postBuild = ''
    cargo run -p transtools -- packfiles
  '';

  postInstall = ''
    install -Dm755 target/release/sysd-manager "$out/bin/sysd-manager"
    install -Dm755 target/release/sysd-manager-proxy "$out/bin/sysd-manager-proxy"

    install -Dm644 data/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg \
      "$out/share/icons/hicolor/scalable/apps/io.github.plrigaux.sysd-manager.svg"

    install -Dm644 data/schemas/io.github.plrigaux.sysd-manager.gschema.xml \
      "$out/share/glib-2.0/schemas/io.github.plrigaux.sysd-manager.gschema.xml"

    install -Dm644 target/loc/io.github.plrigaux.sysd-manager.desktop \
      "$out/share/applications/io.github.plrigaux.sysd-manager.desktop"
    install -Dm644 target/loc/io.github.plrigaux.sysd-manager.metainfo.xml \
      "$out/share/metainfo/io.github.plrigaux.sysd-manager.metainfo.xml"

    cp -r target/locale "$out/share/"

    install -Dm644 sysd-manager-proxy/data/io.github.plrigaux.SysDManager.conf \
      "$out/share/dbus-1/system.d/io.github.plrigaux.SysDManager.conf"
    install -Dm644 target/loc/io.github.plrigaux.SysDManager.policy \
      "$out/share/polkit-1/actions/io.github.plrigaux.SysDManager.policy"
    install -Dm644 sysd-manager-proxy/data/50-io.github.plrigaux.SysDManager.rules \
      "$out/share/polkit-1/rules.d/50-io.github.plrigaux.SysDManager.rules"
    install -Dm644 sysd-manager-proxy/data/sysd-manager-proxy.service \
      "$out/lib/systemd/system/sysd-manager-proxy.service"
  '';

  meta = with lib; {
    description = "A systemd GUI to manage service, timer, socket and other units.";
    homepage = "https://github.com/plrigaux/sysd-manager";
    license = licenses.gpl3Plus;
    maintainers = [ ];
    platforms = platforms.linux;
    mainProgram = "sysd-manager";
  };
}
