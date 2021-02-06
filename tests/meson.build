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
# and the need to call it manually.

# This is the list of tested builtin layouts.
# Please keep each block alphabetical!
# Please keep shapes (with _) on the same line,
# variants (with +) on separate lines.
foreach layout : [
    # This is the fallback layout,
    # so stays first to make sure it never goes missing.
    'us', 'us_wide',

    # Block: Languages
    'be', 'be_wide',
    'bg',
    'br',
    'cz', 'cz_wide',
    'cz+qwerty', 'cz+qwerty_wide',
    'de', 'de_wide',
    'dk',
    'epo',
    'es',
    'es+cat',
    'fi',
    'fr', 'fr_wide',
    'gr',
    'il',
    'ir',
    'it',
    'it+fur',
    'jp+kana','jp+kana_wide',
    'no',
    'pl', 'pl_wide',
    'ru',
    'se',
    'th',
    'ua',
    'us+colemak', 'us+colemak_wide',
    'us+dvorak', 'us+dvorak_wide',

    # Block: Not languages.
    'emoji',
    'number',
    'terminal', 'terminal_wide',
]
    extra = []
    if layout == 'emoji'
        extra += ['allow_missing_return']
    endif
    
    # Older Cargo seens to be sensitive to something
    # about the RUST_FLAGS env var, and rebuilds all tests when it's set,
    # increasing test time by 2 orders of magnitude.
    # Let it have its way.
    if get_option('legacy') == true
        timeout = 300
    else
        timeout = 30
    endif
    test(
        'test_layout_' + layout,
        cargo_script,
        args: ['run'] + cargo_build_flags
            + ['--example', 'test_layout', '--', layout]
            + extra,
        timeout: timeout,
        workdir: meson.build_root(),
    )
endforeach

endif
