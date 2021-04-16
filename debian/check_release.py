#!/usr/bin/env python3

"""Checks tag before release.
Feed it the first changelog line, and then all available tags.
"""

import re, sys
version = re.findall("\\((.*)\\)", input())[0]
tag = 'v' + re.findall("([0-9]+\\.[0-9]+\\.[0-9]+).*", version)[0]
if tag not in map(str.strip, sys.stdin.readlines()):
    raise Exception("Changelog's current version doesn't have a tag. Push the tag!")
