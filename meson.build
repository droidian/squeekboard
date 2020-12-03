project(
    'squeekboard',
    'c', 'rust',
    version: '1.9.3',
    license: 'GPLv3',
    meson_version: '>=0.51.0',
    default_options: [
        'warning_level=1',
        'buildtype=debugoptimized',
        'c_std=gnu11'
    ]
)

add_project_arguments(
  [
    '-Werror=implicit-function-declaration',
    '-Werror=implicit-fallthrough=3',
    '-Werror=maybe-uninitialized',
    '-Werror=missing-field-initializers',
    '-Werror=incompatible-pointer-types',
    '-Werror=int-conversion',
    '-Wformat-nonliteral',
    '-Wformat-security',
    '-Wformat',
    '-Winit-self',
    '-Wmaybe-uninitialized',
    '-Wold-style-definition',
    '-Wredundant-decls',
    '-Wstrict-prototypes',
    '-Wunused',
  ],
  language: 'c'
)

i18n = import('i18n')

conf_data = configuration_data()

if get_option('buildtype').startswith('debug')
    add_project_arguments('-DDEBUG=1', language : 'c')
endif

if get_option('strict')
    add_project_arguments(
        [
            '-Werror',
        ],
        language: 'c'
    )
endif

if get_option('buildtype') != 'plain'
    add_project_arguments('-fstack-protector-strong', language: 'c')
endif
if get_option('buildtype') == 'release'
    cargo_build_flags = ['--release'] # for artifacts
else
    cargo_build_flags = []
endif

prefix = get_option('prefix')
bindir = join_paths(prefix, get_option('bindir'))
datadir = join_paths(prefix, get_option('datadir'))
desktopdir = join_paths(datadir, 'applications')
pkgdatadir = join_paths(datadir, meson.project_name())
if get_option('depdatadir') == ''
  depdatadir = datadir
else
  depdatadir = get_option('depdatadir')
endif
dbusdir = join_paths(depdatadir, 'dbus-1/interfaces')

summary = [
        '',
        '------------------',
        'squeekboard @0@'.format(meson.project_version()),
        '',
        '------------------',
        ''
]
message('\n'.join(summary))

# Rust deps are changing, depending on compile flags. Cargo can't handle it alone.
cargo_toml_in = files('Cargo.toml.in')
path_data = configuration_data()
path_data.set('path', meson.source_root())
cargo_toml_base = configure_file(
    input: 'Cargo.toml.in',
    output: 'Cargo.toml.base',
    configuration: path_data,
)


cargo_deps = files('Cargo.deps')

if get_option('legacy') == true
    cargo_build_flags += ['--features', 'gtk_v0_5,gio_v0_5']
    cargo_deps = files('Cargo.deps.legacy')
endif
    
cat = find_program('cat')
cargo_toml = custom_target(
    'Cargo.toml',
    output: 'Cargo.toml',
    command: [cat, cargo_toml_base, cargo_deps],
    capture: true,
)

dep_cargo = find_program('cargo')
cargo_script = find_program('cargo.sh')
cargo_build = find_program('cargo_build.sh')

subdir('data')
subdir('protocols')
subdir('src')
subdir('tools')
subdir('tests')
