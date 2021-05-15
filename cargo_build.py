#!/usr/bin/env python3

"""This script manages Cargo builds
while keeping the artifact directory within the build tree
instead of the source tree.
"""

from pathlib import Path
import shlex
import subprocess
import sys

source_dir = Path(__file__).absolute().parent

args = sys.argv[1:]
binary_dir = "debug"

if '--release' in args:
    binary_dir = "release"

# The file produced by Cargo will have a special name
try:
    i = args.index('--rename')
except ValueError:
    filename = None
else:
    args.pop(i)
    filename = args.pop(i)

# The target destination of the produced file is a positional argument
out_path = [arg for arg in args if not arg.startswith('--')]
if out_path:
    out_path = out_path[0]
    i = args.index(out_path)
    args.pop(i)    

subprocess.run(['sh', "{}/cargo.sh".format(source_dir.as_posix()), 'build']
    + args,
    check=True)

if out_path:
    out_path = Path(out_path).absolute()
    out_basename = out_path.name
    filename = filename or out_basename
    subprocess.run(['cp', '-a',
        './{}/{}'.format(binary_dir, filename),
        out_path],
        check=True)

