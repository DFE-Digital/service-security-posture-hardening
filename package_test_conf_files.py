#!/usr/bin/env python3
import os
import glob
import re


def get_conf_files():
    return glob.glob("**/**/savedsearches.conf", recursive=True)


def get_content(filename):
    with open(filename, "r") as f:
        contents =f.read()
        return contents


def test_conf_content(content):
    testable_content = ""

    counter = 0

    for line in content.split("\n"):
        # increment line number
        counter += 1

        # ignore if blank line
        if re.match("^(?:\s+)?$", line):
            continue

        # ignore if comment
        if re.match("^(?:\s+)?#.*$", line):
            continue

        # ignore if stanza []
        if re.match("^(?:\s+)?\[.*$", line):
            continue

        # ignore kv pair line which is not a search
        if re.match("^\s*=\s*", line):
            continue

        # # does the line end with backslash, if so, don't add a newline
        # if line.strip().endswith("\\"):
        #     testable_content += f"LINE-{counter-1}: {line}"
        # else:
        #     testable_content += f"LINE-{counter-1}: {line}\n"

        print(line)
    # # match non-key-value pairs
    # matches = re.findall("^LINE-\d+: (?![\w\.\-\,\s]+\=).*$", testable_content, re.MULTILINE)
    # return matches


def main():
    has_error = False

    print("-"*35)
    print("Testing all .conf files for bad key-value pair lines (missing backslash)")
    print("-"*35)

    files = get_conf_files()
    for f in files:
        content = get_content(f)
        matches = test_conf_content(content)
        if len(matches) != 0:
            print(f"{len(matches)} error(s) found in:", f)
            for m in matches:
                print((m[:70] + '...') if len(m) > 70 else m)
            print("-"*35)
            has_error = True

    if has_error:
        exit(1)
    else:
        print("No issues found! ✔")
        print("-"*35)


if __name__ == "__main__":
    main()