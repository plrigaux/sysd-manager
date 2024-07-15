#https://belmoussaoui.com/blog/8-how-to-flatpak-a-rust-application/
#https://github.com/squidfarts/rs-example
project(
  'sysd-manager',
  'rust',
#  version: '0.0.2',
  meson_version: '>= 0.59',
  license: 'GPL-3.0+',
)

gnome = import('gnome')

base_id = 'org.tool.sysd-manager'


dependency('glib-2.0', version: '>= 2.66')
dependency('gio-2.0', version: '>= 2.66')
dependency('gtk4', version: '>= 4.0.0')


find_program('glib-compile-resources')
#glib_compile_schemas = find_program('glib-compile-schemas')
desktop_file_validate = find_program('desktop-file-validate', required: false)
appstreamcli = find_program('appstreamcli', required: false)
cargo = find_program('cargo')

version = meson.project_version()
message('version: ', version)

prefix = get_option('prefix')
message('prefix: ', prefix)

bindir = prefix / get_option('bindir')
message('bindir: ', bindir)

localedir = prefix / get_option('localedir')
message('localedir: ', localedir)


datadir = prefix / get_option('datadir')
message('datadir: ', datadir)

pkgdatadir = datadir / meson.project_name()
message('pkgdatadir: ', pkgdatadir)

iconsdir = datadir / 'icons'
message('iconsdir: ', iconsdir)

# for language
#podir = meson.project_source_root() / 'po'
#message('podir: ', podir)

gettext_package = meson.project_name()
message('gettext_package: ', gettext_package)

profile_option = get_option('profile')
message('profile_option: ', profile_option)

if profile_option == 'development'
  profile = 'Devel'
  vcs_tag = run_command('git', 'rev-parse', '--short', 'HEAD', check: false).stdout().strip()
  if vcs_tag == ''
    version_suffix = '-devel'
  else
    version_suffix = '-@0@'.format(vcs_tag)
  endif
  application_id = '@0@.@1@'.format(base_id, profile)
else
  profile = ''
  version_suffix = ''
  application_id = base_id
endif

meson.add_dist_script(
  'build_aux/dist-vendor.sh',
  meson.project_build_root() / 'meson-dist' / meson.project_name() + '-' + version,
  meson.project_source_root()
)

if get_option('profile') == 'development'
  # Setup pre-commit hook for ensuring coding style is always consistent
  message('Setting up git pre-commit hook..')
  run_command('cp', '-f', 'hooks/pre-commit.hook', '.git/hooks/pre-commit', check: false)
endif


subdir('data')
#subdir('po') //for language
subdir('src')

#executable('rust_exe', 'src/main.rs')