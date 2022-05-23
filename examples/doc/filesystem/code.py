#!/usr/bin/env python3

# CODE.py
#   by Lut99
#
# Created:
#   23 May 2022, 16:23:31
# Last edited:
#   23 May 2022, 16:35:01
# Auto updated?
#   Yes
#
# Description:
#   Contains the code for the third tutorial in the Brane: The User Guide
#   (https://wiki.enablingpersonalizedinterventions.nl/user-guide/software-engi
#   neers/filesystem.md).
#
#   This package implements a very simple "filesystem", which can write and
#   read content to the shared `/data` folder.
#


# Imports
import os
import sys
import yaml



# The functions
def write(name: str, contents: str) -> int:
    """
        Writes a given string to the distributed filesystem.
    """

    # We wrap the writing in a try/catch so we may catch any errors
    try:
        # Open the file and write the content
        with open(f"/data/{name}.txt", "w") as f:
            f.write(contents)

        # Return 0 (i.e., "success")
        return 0

    # Catch file errors
    except IOError as e:
        # Return the non-zero exit code that they define
        return e.errno


def read(name: str) -> str:
    """
        Reads the given file in the distributed filesystem and returns its contents.
    """

    # Once again we wrap the reading in a try/catch so we may catch any errors
    try:
        # Open the file and read the content
        with open(f"/data/{name}.txt", "r") as f:
            content = f.read()

        # Return the string
        return content

    # Catch file errors
    except IOError as e:
        # Return the error message
        return f"ERROR: {e} ({e.errno})"



# The entrypoint of the script
if __name__ == "__main__":
    # Make sure that at least one argument is given, that is either 'write' or 'read'
    if len(sys.argv) != 2 or (sys.argv[1] != "write" and sys.argv[1] != "read"):
        print(f"Usage: {sys.argv[0]} write|read")
        exit(1)

    # If it checks out, call the appropriate function
    command = sys.argv[1]
    if command == "write":
        # Write the file and print the error code
        print(yaml.dump({ "code": write(os.environ["NAME"], os.environ["CONTENTS"]) }))
    else:
        # Read the file and print the contents
        print(yaml.dump({ "contents": read(os.environ["NAME"]) }))

    # Done!
