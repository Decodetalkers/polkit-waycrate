project(
  'polkit-waycrate',
  'rust',
  version: '0.1.0',
  meson_version: '>=1.1.0',
)

cargo = find_program('cargo', version: '>= 1.85')
target_name = 'polkit-waycrate-authentication-agent-1'
if get_option('debug')
  command = [
    cargo,
    'build',
    '&&',
    'cp',
    meson.global_source_root() / 'target' / 'debug' / target_name,
    '@OUTPUT@',
  ]

else
  command = [
    cargo,
    'build',
    '--release', '&&',
    'cp',
    meson.global_source_root() / 'target' / 'release' / target_name,
    '@OUTPUT@',
  ]

endif

prefix = get_option('prefix')

bindir = prefix / get_option('bindir')

custom_target(
  target_name,
  output: target_name,
  build_by_default: true,
  install: true,
  install_dir: bindir,
  console: true,
  command: command,
)
