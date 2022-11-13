#!/usr/bin/env python3
# SERIALIZE.py
#   by Lut99
#
# Created:
#   22 Sep 2022, 15:54:02
# Last edited:
#   22 Sep 2022, 16:21:38
# Auto updated?
#   Yes
#
# Description:
#   Implements the serialize package, that takes one of each data type and
#   then converts it to a string.
#

import json
import os


##### ENTRYPOINT #####
def main(input: str) -> int:
    """
        Entrypoint to the script that performs the actual work.
    """

    # Simply parse it as JSON
    val = json.loads(input)

    # Write to stdout as a json string
    print(f"output: {json.dumps(str(val))}")

    # Done
    return 0



# Actual entrypoint
if __name__ == "__main__":
    # Simply run main with the appropriate environment variable
    exit(main(os.environ["INPUT"]))
