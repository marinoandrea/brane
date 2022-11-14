#!/usr/bin/env python3
# START.py
#   by Lut99
#
# Created:
#   14 Nov 2022, 13:43:51
# Last edited:
#   14 Nov 2022, 14:22:05
# Auto updated?
#   Yes
#
# Description:
#   Companion of the `make.py` script that takes starting into account once
#   the framework has been compiled.
#   
#   Mostly exists to spawn the framework somewhere where there are only
#   compiled images.
#

import argparse
import os
import sys


##### CONSTANTS #####
# The version of Brane for which this make script is made
# Only relevant when downloading files
VERSION = "0.6.3"





##### HELPER FUNCTIONS #####
def supports_color():
    """
        Returns True if the running system's terminal supports color, and False
        otherwise.

        From: https://stackoverflow.com/a/22254892
    """
    plat = sys.platform
    supported_platform = plat != 'Pocket PC' and (plat != 'win32' or
                                                  'ANSICON' in os.environ)
    # isatty is not always implemented, #6223.
    is_a_tty = hasattr(sys.stdout, 'isatty') and sys.stdout.isatty()
    return supported_platform and is_a_tty

def debug(verbose: bool, text: str, end: str = "\n"):
    """
        Prints the given `text` to stdout, with debug formatting.

        Only prints if `verbose` is True.
    """

    # Do the "only prints" part
    if not verbose: return

    # Get colours
    start_col = "\033[90m" if supports_color() else ""
    end_col   = "\033[0m" if supports_color() else ""

    # Print it
    print(f"{start_col}[DEBUG] {text}{end_col}", end=end)

def fatal(text: str, end: str = "\n", code: int = 1):
    """
        Prints the given `text` to stderr, with error formatting.

        Then it `exit()`s with the given error code.
    """

    # Get colours
    start_col = "\033[91;1m" if supports_color() else ""
    end_col   = "\033[0m" if supports_color() else ""
    start_text_col = "\033[1m" if supports_color() else ""
    end_text_col   = "\033[0m" if supports_color() else ""

    # Print it
    print(f"{start_col}[ERROR]{end_col}{start_text_col} {text}{end_text_col}", end=end)
    exit(code)





##### ENTRYPOINT #####
def main(cmd: str, node: str | None, location_id: str | None, central_images: dict[str, str], worker_images: dict[str, str], central_ports: dict[str, int], worker_ports: dict[str, int], file: str, verbose: bool) -> int:
    """
        The main function of this script.
    """

    debug(verbose,  "Starting start.py")
    debug(verbose, f"  - command        {'      ' if node == 'central' else ''}: '{cmd}'")
    debug(verbose, f"  - node           {'      ' if node == 'central' else ''}: '{node}'")
    if cmd == "start" and node == "central":
        debug(verbose, f"  - API image            : '{central_images['api']}'")
        debug(verbose, f"  - driver image         : '{central_images['drv']}'")
        debug(verbose, f"  - planner image        : '{central_images['plr']}'")
        debug(verbose, f"  - Docker registry port : {central_ports['con']}")
        debug(verbose, f"  - API port             : {central_ports['api']}")
        debug(verbose, f"  - driver port          : {central_ports['drv']}")
    elif cmd == "start" and node == "worker":
        debug(verbose, f"  - Location ID    : '{location_id}'")
        debug(verbose, f"  - Registry image : '{worker_images['reg']}'")
        debug(verbose, f"  - Delegate image : '{worker_images['job']}'")
        debug(verbose, f"  - Registry port  : {worker_ports['reg']}")
        debug(verbose, f"  - Delegate port  : {worker_ports['job']}")
    debug(verbose, f"  - file           {'      ' if node == 'central' else ''}: '{file}'")
    debug(verbose, f"  - verbose        {'      ' if node == 'central' else ''}: {'yes' if verbose else 'no'}")

    return 0



# The actual entrypoint
if __name__ == "__main__":
    # Parse the arguments first
    parser = argparse.ArgumentParser()
    parser.add_argument("COMMAND", choices=["start", "stop"], help="The action to perform in this script. Can be 'start' or 'stop'.")
    parser.add_argument("NODE", choices=["central", "worker"], help="The type of node to start/stop. Can be 'central' or 'worker'. Only relevant (and required) if we're starting a node.")
    parser.add_argument("LOCATION_ID", nargs='?', help="The location ID for this worker. Only relevant (and required) if we're starting a worker node.")

    parser.add_argument("-c", "--config", default="./config", help="Points to the directory with the configuration files for this node.")
    parser.add_argument("--data", default="./data", help="Points to the directory that stores/gives access to this node's data. Only relevant if we're starting a worker node.")
    parser.add_argument("--results", default="/tmp/results", help="Points to the directory that stores intermediate results for this node. Only relevant if we're starting a worker node.")
    parser.add_argument("--certs", default="$CONFIG/certs", help="Points to the directory with certificates to use for this node.You can use '$CONFIG' to use the part to the configuration folder.")

    parser.add_argument("-a", "--api-image", default="./target/release/brane-api.tar", help="Points to the API service image file to actually load in the Docker daemon. Only relevant if we're starting a central node.")
    parser.add_argument("-d", "--drv-image", default="./target/release/brane-drv.tar", help="Points to the driver service image file to actually load in the Docker daemon. Only relevant if we're starting a central node.")
    parser.add_argument("-p", "--plr-image", default="./target/release/brane-plr.tar", help="Points to the planner service image file to actually load in the Docker daemon. Only relevant if we're starting a central node.")
    parser.add_argument("-r", "--reg-image", default="./target/release/brane-reg.tar", help="Points to the registry service image file to actually load in the Docker daemon. Only relevant if we're starting a worker node.")
    parser.add_argument("-j", "--job-image", default="./target/release/brane-job.tar", help="Points to the delegate service image file to actually load in the Docker daemon. Only relevant if we're starting a worker node.")

    parser.add_argument("-A", "--api-port", type=int, default=50051, help="Determines the port on which the API service runs. Only relevant if we're starting a central node.")
    parser.add_argument("-D", "--drv-port", type=int, default=50053, help="Determines the port on which the driver service runs. Only relevant if we're starting a central node.")
    parser.add_argument("-C", "--con-port", type=int, default=50050, help="Determines the port on which the Docker container registry service runs. Only relevant if we're starting a central node.")
    parser.add_argument("-R", "--reg-port", type=int, default=50051, help="Determines the port on which the registry service runs. Only relevant if we're starting a worker node.")
    parser.add_argument("-J", "--job-port", type=int, default=50052, help="Determines the port on which the delegate service runs. Only relevant if we're starting a worker node.")

    parser.add_argument("-f", "--file", default="./docker-compose-$NODE.yml", help="The docker-compose file to actually run. You can use '$NODE' to use the current node identifier (i.e., 'central' or 'worker').")
    parser.add_argument("-v", "--verbose", action="store_true", help="If given, provides additional updates on what's happening.")

    # Parse the arguments
    args = parser.parse_args()
    if args.COMMAND == "start" and args.NODE == "worker" and args.LOCATION_ID is None: fatal("Missing LOCATION_ID parameter (use --help to learn more)")
    args.certs = args.certs.replace("$CONFIG", args.config)
    args.file  = args.file.replace("$NODE", args.NODE)

    # Run main
    exit(main(args.COMMAND, args.NODE, args.LOCATION_ID, { "api": args.api_image, "drv": args.drv_image, "plr": args.plr_image }, { "reg": args.reg_image, "job": args.job_image }, { "api": args.api_port, "drv": args.drv_port, "con": args.con_port }, { "reg": args.reg_port, "job": args.job_port }, args.file, args.verbose))
