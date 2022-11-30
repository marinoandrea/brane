#!/usr/bin/env python3

# CODE.py
#   by Lut99
#
# Created:
#   23 May 2022, 16:23:31
# Last edited:
#   30 Nov 2022, 16:25:02
# Auto updated?
#   Yes
#
# Description:
#   Contains the code for the second tutorial in the Brane: The User Guide
#   (https://wiki.enablingpersonalizedinterventions.nl/user-guide/software-engi
#   neers/base64.md).
#
#   This package implements a simple Base64 encode/decode package.
#


# Imports
import base64
import json
import os
import sys
import yaml


# The functions
def encode(s: str) -> str:
    """
        Encodes a given string as Base64, and returns the result as a string
        again.
    """

    # First, get the raw bytes of the string (to have correct padding and such)
    b = s.encode("utf-8")

    # We simply encode using the b64encode function
    b = base64.b64encode(b)

    # We return the value, but not after interpreting the raw bytes returned by the function as a string
    return b.decode("utf-8")


def decode(b: str) -> str:
    """
        Decodes the given Base64 string back to plain text.
    """

    # Remove any newlines that may be present from line splitting first, as these are not part of the Base64 character set
    b = b.replace("\n", "")

    # Decode using the base64 module again
    s = base64.b64decode(b)

    # Finally, we return the value, once again casting it
    return s.decode("utf-8")


# The entrypoint of the script
if __name__ == "__main__":
    # Make sure that at least one argument is given, that is either 'encode' or 'decode'
    if len(sys.argv) != 2 or (sys.argv[1] != "encode" and sys.argv[1] != "decode"):
        print(f"Usage: {sys.argv[0]} encode|decode")
        exit(1)

    # If it checks out, call the appropriate function
    command = sys.argv[1]
    if command == "encode":
        # Parse the input as JSON, then pass that to the `encode` function
        arg = json.loads(os.environ["INPUT"])
        result = encode(arg)
    else:
        # Parse the input as JSON, then pass that to the `encode` function
        arg = json.loads(os.environ["INPUT"])
        result = decode(arg)

    # Print the result with the YAML package
    print(yaml.dump({ "output": result }))

    # Done!
