entry = configure_file(
    copy: true,
    input: 'entry.py',
    output: 'squeekboard-entry',
    install: true,
    install_dir: bindir,
)

test_layout = custom_target('squeekboard-test-layout',
    build_by_default: true,
    # meson doesn't track all inputs, cargo does
    build_always_stale: true,
    output: ['squeekboard-test-layout'],
    console: true,
    command: [cargo_build, '--rename', 'test_layout', '@OUTPUT@', '--bin', 'test_layout']
        + cargo_build_flags,
    install: true,
    install_dir: bindir,
    depends: cargo_toml,
)
