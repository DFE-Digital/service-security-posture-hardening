#!/usr/bin/env python3
import glob
import configparser
import re
import sys


class ConfParser:
    # Match a '\' followed by optional spaces then a new line
    SLASH_NEW_LINE = re.compile(r"(\\\s*?\n)", flags=re.M)

    def __init__(self, filename):
        self.filename = filename
        self.load_file()

    def validate(self, new):
        c = configparser.ConfigParser()
        try:
            # Try to load the currently parsed config
            c.read_string("\n".join(new), source=self.filename)
        except (configparser.ParsingError, configparser.DuplicateOptionError) as e:
            self.error = e
            # Is the config valid?
            return False
        return True

    def load_file(self):
        """Read a .conf file"""
        with open(self.filename) as f:
            self.data = f.readlines()

    def parse(self) -> bool:
        new = []
        join_next_line = False

        for line_number, line in enumerate(self.data):
            # Previous line ended with a '\'
            if join_next_line:
                join_next_line = False

                if self.SLASH_NEW_LINE.search(line):
                    # This line ends with a '\'
                    # Remove the '\\\n'
                    line = self.SLASH_NEW_LINE.sub(" ", line)
                    # Append this line to the previous line
                    new[-1] = f"{new[-1]} {line}"
                    # Append the next line to the current one
                    join_next_line = True

            elif self.SLASH_NEW_LINE.search(line):
                # This line ends with a '\' (the previous line didn't)
                # Remove the '\\\n'
                line = self.SLASH_NEW_LINE.sub(" ", line)
                # Append the next line to the current one
                join_next_line = True
                new.append(line)

            else:
                # The previous line didn't end in a '\' and this line doesn't have a '\'
                new.append(line)

            if self.validate(new):
                # Config is valid so far, try the next line
                continue
            else:
                # Config isn't valid. Set up error reporting
                self.line_number = line_number
                self.new = new
                self.result = False
                break
        else:
            # Config must be valid
            self.result = True
        return self.result

    def show_error(self):
        """Show error messages"""
        print("!" * 100)
        print(f"ERROR while parsing: {self.filename}")
        print("COMPUTED:")
        for n, line in enumerate(self.new[-10:]):
            print(f"{len(self.new) -10+n}: ^{line.strip()}$")
        print("\n\n")
        print(self.error)
        print("\n\n")
        print("ORIGINAL:")
        for n, line in enumerate(
            self.data[self.line_number - 10 : self.line_number + 1]
        ):
            print(f"{self.line_number+1 -10+n}: ^{line.strip()}$")
        print("\n\n")
        print(self.error)
        print("" + "!" * 100 + "\n\n")


def conf_files():
    f = glob.glob("*.conf", recursive=True)
    f += glob.glob("**/*.conf", recursive=True)
    return f


def main():
    conffiles = conf_files()
    errors = False
    for conf_file in conffiles:
        parser = ConfParser(conf_file)
        if not parser.parse():
            errors = True
            parser.show_error()
        else:
            print(f"{conf_file} is validish!")
    if errors:
        sys.exit(1)


if __name__ == "__main__":
    main()
