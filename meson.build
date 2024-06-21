#https://belmoussaoui.com/blog/8-how-to-flatpak-a-rust-application/
#https://github.com/squidfarts/rs-example
project(
  'sysd-manager',
  'rust',
  version: '0.0.2',
  meson_version: '>= 0.59',
  license: 'GPL-3.0+',
)

dependency('glib-2.0', version: '>= 2.66')
dependency('gio-2.0', version: '>= 2.66')
dependency('gtk4', version: '>= 4.0.0')

executable('rust_exe', 'src/main.rs')