#subdir('icons/hicolor/scalable/apps')
#subdir('resources')

# Desktop file
desktop_conf = configuration_data()
message('Desktop file desktop_conf: ', desktop_conf.keys())
#desktop_conf.set('icon', application_id)
#
#desktop_file = i18n.merge_file(
#  type: 'desktop',
#  input: configure_file(
#    input: '@0@.desktop.in.in'.format(base_id),
#    output: '@BASENAME@',
#    configuration: desktop_conf
#  ),
#  output: '@0@.desktop'.format(application_id),
#  po_dir: podir,
#  install: true,
#  install_dir: datadir / 'applications'
#)

# Validate Desktop file
if desktop_file_validate.found()
  message('TODO test desktop file')
  #test(
  #  'validate-desktop',
  #  desktop_file_validate,
  #  args: [
  #    desktop_file.full_path()
  #  ],
  #  depends: desktop_file,
  #)
endif

# Appdata
appdata_conf = configuration_data()
appdata_conf.set('app-id', application_id)
appdata_conf.set('gettext-package', gettext_package)
#appdata_file = i18n.merge_file(
#  input: configure_file(
#    input: '@0@.metainfo.xml.in.in'.format(base_id),
#    output: '@BASENAME@',
#    configuration: appdata_conf
#  ),
#  output: '@0@.metainfo.xml'.format(application_id),
#  po_dir: podir,
#  install: true,
#  install_dir: datadir / 'metainfo'
#)

# Validate Appdata
if appstreamcli.found()
  message('TODO Validate Appdata')
  #test(
  #  'validate-appdata', appstreamcli,
  #  args: [
  #    'validate', '--no-net', '--explain', appdata_file.full_path()
  #  ],
  #  depends: appdata_file,
  #)
endif

# GSchema
message('No gschema to manipulate')
#gschema_conf = configuration_data()
#gschema_conf.set('app-id', application_id)
#gschema_conf.set('gettext-package', gettext_package)
#configure_file(
#  input: '@0@.gschema.xml.in'.format(base_id),
#  output: '@0@.gschema.xml'.format(application_id),
#  configuration: gschema_conf,
#  install: true,
#  install_dir: datadir / 'glib-2.0' / 'schemas'
#)
#
## Validata GSchema
#test(
#  'validate-gschema', glib_compile_schemas,
#  args: [
#    '--strict', '--dry-run', meson.current_build_dir()
#  ],
#)

# Resources
resources = gnome.compile_resources(
  'resources',
  'resources/resources.gresource.xml',
  gresource_bundle: true,
  source_dir: meson.current_build_dir(),
  install: true,
  install_dir: pkgdatadir,
)

message('Gnome Resources package dir :', pkgdatadir)

datadir = get_option('prefix') / get_option('datadir')
application_id = 'io.github.plrigaux.sysd-manager'

message('Data dir :', datadir)
message('Application id:', application_id)

install_data(
  'icons/hicolor/scalable/apps/' / 'io.github.plrigaux.sysd-manager.svg',
  install_dir: datadir / 'icons/hicolor/scalable/apps/',
)

install_data(
  'applications' / 'io.github.plrigaux.sysd-manager.desktop',
  install_dir: '/app/share/applications',
)

install_data(
  'metainfo' / 'io.github.plrigaux.sysd-manager.metainfo.xml',
  install_dir: datadir / 'metainfo',
)