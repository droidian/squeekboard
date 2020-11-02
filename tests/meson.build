if get_option('tests')

test_env = [
  'G_TEST_SRCDIR=@0@'.format(meson.current_source_dir()),
  'G_TEST_BUILDDIR=@0@'.format(meson.current_build_dir()),
  'G_DEBUG=gc-friendly,fatal-warnings',
  'GSETTINGS_BACKEND=memory',
  'MALLOC_CHECK_=2'
]

test_cflags = [
  '-I@0@/../src'.format(meson.current_source_dir()),
  '-I@0@/../src'.format(meson.current_build_dir()),
  '-DEEK_COMPILATION',
  '-DEEKBOARD_COMPILATION'
]

test_link_args = [
  '-fPIC',
]

c_tests = [
]

foreach name : c_tests

    test_sources = [name + '.c']

    t = executable(
        name,
        test_sources,
        squeekboard_resources,
        link_with: libsqueekboard,
        c_args : test_cflags,
        link_args: test_link_args,
        dependencies: deps,       # from src/meson.build
        include_directories: [
            include_directories('..'),
            include_directories('../eek')
        ]
    )

    test(name, t, env: test_env)

endforeach

# The layout test is in the examples directory
# due to the way Cargo builds executables
# and the need to call it manually
foreach layout : [
    'us', 'us+colemak', 'us_wide',
    'br',
    'be', 'be_wide',
    'de', 'de_wide',
    'dk',
    'es',
    'fi',
    'fr', 'fr_wide',
    'it+fur',
    'gr',
    'it',
    'jp+kana','jp+kana_wide',
    'no',
    'number',
    'pl', 'pl_wide',
    'ru',
    'se',
    'ua',
    'terminal', 'terminal_wide',

    'emoji',
]
    extra = []
    if layout == 'emoji'
        extra += ['allow_missing_return']
    endif

    test(
        'test_layout_' + layout,
        cargo_script,
        args: ['run'] + cargo_build_flags
            + ['--example', 'test_layout', '--', layout]
            + extra,
        workdir: meson.build_root(),
    )
endforeach

endif
