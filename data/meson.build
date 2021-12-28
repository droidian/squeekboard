gnome = import('gnome')

squeekboard_resources = gnome.compile_resources(
   'squeekboard-resources',
   'squeekboard.gresources.xml',

   c_name: 'squeekboard',
)

desktopconf = configuration_data()
desktopconf.set('bindir', bindir)

desktop_file = 'sm.puri.Squeekboard.desktop'

i18n.merge_file(
    input: configure_file(
      input: desktop_file + '.in.in',
      output: desktop_file + '.in',
      configuration: desktopconf
    ),
    output: desktop_file,
    po_dir: '../po',
    install: true,
    install_dir: desktopdir,
    type: 'desktop'
)
