#!/usr/bin/env python3
# START.py
#   by Lut99
#
# Created:
#   14 Nov 2022, 13:43:51
# Last edited:
#   16 Nov 2022, 11:43:26
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
import subprocess
import sys
from typing import Optional


##### CONSTANTS #####
# The version of Brane for which this make script is made
# Only relevant when downloading files
VERSION = "1.0.0"

# The list of services that live on the central node. Each of them maps to a short flag, long flag prefix, a more readable name and their default port if any.
CENTRAL_SERVICES = {
    # "aux-registry"  : ("c", "container", "Docker registry", 50050),
    "aux-scylla"    : ("s", "scylla", "Scylla database", None),
    "aux-kafka"     : ("k", "kafka", "Kafka", None),
    "aux-zookeeper" : ("z", "zookeeper", "Kafka Zookeeper", None),
    "aux-xenon"     : ("x", "xenon", "Xenon", None),
    "brane-api"     : ("a", "api", "global registry", 50051),
    "brane-drv"     : ("d", "drv", "driver", 50053),
    "brane-plr"     : ("p", "plr", "planner", None),
}

# The list of services that live on the worker node. Each of them maps to a short flag, long flag prefix, a more readable name and their default port if any.
WORKER_SERVICES = {
    "brane-reg" : ("r", "reg", "local registry", 50051),
    "brane-job" : ("j", "job", "delegate", 50052),
}





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
def main(cmd: str, node: str, location_id: Optional[str], config: str, packages: str, data: str, results: str, certs: str, central_images: dict[str, str], worker_images: dict[str, str], central_ports: dict[str, int], worker_ports: dict[str, int], file: str, verbose: bool) -> int:
    """
        The main function of this script.
    """

    # Print the parameters if in verbose mode
    services = CENTRAL_SERVICES if node == "central" else WORKER_SERVICES
    images   = central_images if node == "central" else worker_images
    ports    = central_ports if node == "central" else worker_ports
    if verbose:
        # Compute the longest thing
        longest = max([ 17 ] + [len(services[svc][2]) + 6 for svc in services])

        # Print the main two
        debug(verbose, f"Starting {sys.argv[0]} with:")
        debug(verbose, f"  - command{' ' * (longest - 7)} : '{cmd}'")
        debug(verbose, f"  - node{' ' * (longest - 4)} : '{node}'")

        # Print any central node relating ones
        if cmd == "start" and node == "central":
            debug(verbose, f"  - Certificates path{' ' * (longest - 17)} : '{certs}'")
            debug(verbose, f"  - Packages path{' ' * (longest - 13)} : '{packages}'")
            for svc, path in central_images.items():
                debug(verbose, f"  - {CENTRAL_SERVICES[svc][2]} image{' ' * (longest - len(CENTRAL_SERVICES[svc][2]) - 6)} : {path}")
            for svc, port in central_ports.items():
                debug(verbose, f"  - {CENTRAL_SERVICES[svc][2]} port{' ' * (longest - len(CENTRAL_SERVICES[svc][2]) - 5)} : {port}")

        # Print any worker node relating ones
        if cmd == "start" and node == "worker":
            debug(verbose, f"  - Location ID{' ' * (longest - 11)} : '{location_id}'")
            debug(verbose, f"  - Config path{' ' * (longest - 11)} : '{config}'")
            debug(verbose, f"  - Packages path{' ' * (longest - 13)} : '{packages}'")
            debug(verbose, f"  - Data path{' ' * (longest - 9)} : '{data}'")
            debug(verbose, f"  - Results path{' ' * (longest - 12)} : '{results}'")
            debug(verbose, f"  - Certificates path{' ' * (longest - 17)} : '{certs}'")
            for svc, path in worker_images.items():
                debug(verbose, f"  - {WORKER_SERVICES[svc][2]} image{' ' * (longest - len(WORKER_SERVICES[svc][2]) - 6)} : {path}")
            for svc, port in worker_ports.items():
                debug(verbose, f"  - {WORKER_SERVICES[svc][2]} port{' ' * (longest - len(WORKER_SERVICES[svc][2]) - 5)} : {port}")

        # Print the final two + newline
        debug(verbose, f"  - file{' ' * (longest - 4)} : '{file}'")
        debug(verbose, f"  - verbose{' ' * (longest - 7)} : {'yes' if verbose else 'no'}")
        print()



    # Switch on what we need to do
    if cmd == "start":
        # Attempt to load all of the images we need
        for svc in images:
            # Load the image into the daemon
            debug(verbose, f"Loading image '{images[svc]}' for service '{svc}'...")
            handle = subprocess.Popen(["docker", "load", "--input", images[svc]], stdout=subprocess.PIPE, stderr=sys.stderr)
            stdout, _ = handle.communicate()
            # if handle.returncode != 0: fatal(f"Command 'docker load --input {images[svc]}' failed with exit code {handle.returncode} (see above for stderr)")
            if handle.returncode != 0: continue

            # Extract the image tag
            stdout = stdout.decode("utf-8")
            if stdout[:24] == "Loaded image ID: sha256:":
                # It's an untagged image
                tag = stdout[24:].strip()

                # Tag the image
                debug(verbose, f"Tagging image '{tag}' as '{svc}'...")
                handle = subprocess.Popen(["docker", "tag", tag, svc], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                stdout, stderr = handle.communicate()
                if handle.returncode != 0:
                    print(f"\nstdout:\n{'-' * 79}\n{stdout}\n{'-' * 79}\n")
                    print(f"stderr:\n{'-' * 79}\n{stderr}\n{'-' * 79}\n")
                    fatal(f"Command 'docker tag {tag} {svc}' failed with exit code {handle.returncode} (see above)")

            elif stdout[:14] == "Loaded image: ":
                # It's a tagged image
                parts = stdout[14:].strip().split(":")
                if len(parts) != 2:
                    fatal(f"Failed to split '{stdout}' into a name:version pair.")
                (name, version) = parts

                # Tag it as the appropriate name for us
                debug(verbose, f"Tagging image '{name}:{version}' as '{svc}'...")
                handle = subprocess.Popen(["docker", "tag", f"{name}:{version}", svc], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                stdout, stderr = handle.communicate()
                if handle.returncode != 0:
                    print(f"\nstdout:\n{'-' * 79}\n{stdout}\n{'-' * 79}\n")
                    print(f"stderr:\n{'-' * 79}\n{stderr}\n{'-' * 79}\n")
                    fatal(f"Command 'docker tag {name}:{version} {svc}' failed with exit code {handle.returncode} (see above)")

            else:
                fatal(f"Failed to retrieve image tag or name from '{stdout}'")
            

        # Generate the environment string
        debug(verbose, f"Preparing environment variables...")
        env = []
        for svc in ports:
            env.append(f"{services[svc][1].upper().replace('-', '_')}_PORT=\"{ports[svc]}\"")
        if node == "central":
            env.append(f"PACKAGES=\"{packages}\"")
        elif node == "worker":
            env.append(f"CONFIG=\"{config}\"")
            env.append(f"PACKAGES=\"{os.path.abspath(packages)}\"")
            env.append(f"DATA=\"{os.path.abspath(data)}\"")
            env.append(f"RESULTS=\"{os.path.abspath(results)}\"")
        env.append(f"CERTS=\"{certs}\"")
        if node == "worker":
            env.append(f" LOCATION_ID=\"{location_id}\"")
        debug(verbose, f"Environment variables: '{' '.join(env)}'")

        # Finally, call docker compose with that
        debug(verbose, f"Calling docker-compose on '{file}'...")
        senv = ' '.join(env)
        handle = subprocess.Popen(["bash", "-c", f"{senv} docker-compose -p brane-{node} -f \"{file}\" up -d"])
        stdout, _ = handle.communicate()
        if handle.returncode != 0:
            print(f"\nstdout:\n{'-' * 79}\n{stdout}\n{'-' * 79}\n")
            senv = ' '.join(env).replace('\"', '\\\"')
            fatal(f"Command 'bash -c \"{senv} docker-compose -p brane-{node} -f \\\"{file}\\\" up -d\"' failed with exit code {handle.returncode} (see above)")

    elif cmd == "stop":
        debug(verbose, f"Calling docker-compose on '{file}'...")
        handle = subprocess.Popen(["docker-compose", "-p", f"brane-{node}", "-f", file, "down"])
        stdout, _ = handle.communicate()
        if handle.returncode != 0:
            print(f"\nstdout:\n{'-' * 79}\n{stdout}\n{'-' * 79}\n")
            fatal(f"Command 'bash -c \"docker-compose -p brane-{node} -f \\\"{file}\\\" down\"' failed with exit code {handle.returncode} (see above)")



    # Done!
    debug(verbose, "Success")
    return 0



# The actual entrypoint
if __name__ == "__main__":
    # Parse the arguments first
    parser = argparse.ArgumentParser()
    parser.add_argument("COMMAND", choices=["start", "stop"], help="The action to perform in this script. Can be 'start' or 'stop'.")
    parser.add_argument("NODE", choices=["central", "worker"], help="The type of node to start/stop. Can be 'central' or 'worker'. Only relevant (and required) if we're starting a node.")
    parser.add_argument("LOCATION_ID", nargs='?', help="The location ID for this worker. Only relevant (and required) if we're starting a worker node.")

    parser.add_argument("--config", default="./config", help="Points to the directory with the configuration files for this node.")
    parser.add_argument("--packages", default="./packages", help="Points to the directory that stores/gives access to this node's package registry. You can use '$CONFIG' to replace that part with the '--config' value.")
    parser.add_argument("--data", default="./data", help="Points to the directory that stores/gives access to this node's data. Only relevant if we're starting a worker node. You can use '$CONFIG' to replace that part with the '--config' value.")
    parser.add_argument("--results", default="/tmp/results", help="Points to the directory that stores intermediate results for this node. Only relevant if we're starting a worker node. You can use '$CONFIG' to replace that part with the '--config' value.")
    parser.add_argument("--certs", default="$CONFIG/certs", help="Points to the directory with certificates to use for this node. You can use '$CONFIG' to replace that part with the '--config' value.")

    # Generate image flags for all services
    parser.add_argument("-i", "--image-dir", default="./target/release", help="Provides a base path for all the image files. Only used as a common path for them, no other files are loaded from this directory.")
    for service in sorted(CENTRAL_SERVICES):
        short, long, name, _ = CENTRAL_SERVICES[service]
        parser.add_argument(f"-{short}", f"--{long}-image", default=f"$IMAGE_DIR/{service}.tar", help=f"Points to the {name} image file to load in the Docker daemon. You can use '$IMAGE_DIR' to refer to the '--image-dir' value. Only relevant if we're starting a central node.")
    for service in sorted(WORKER_SERVICES):
        short, long, name, _ = WORKER_SERVICES[service]
        parser.add_argument(f"-{short}", f"--{long}-image", default=f"$IMAGE_DIR/{service}.tar", help=f"Points to the {name} image file to load in the Docker daemon. You can use '$IMAGE_DIR' to refer to the '--image-dir' value. Only relevant if we're starting a worker node.")

    # Generate port flags for all services
    for service in sorted(CENTRAL_SERVICES):
        short, long, name, port = CENTRAL_SERVICES[service]
        if port is None: continue
        parser.add_argument(f"-{short.upper()}", f"--{long}-port", type=int, default=port, help=f"Determines the port on which the {name} service runs. Only relevant if we're starting a central node.")
    for service in sorted(WORKER_SERVICES):
        short, long, name, port = WORKER_SERVICES[service]
        if port is None: continue
        parser.add_argument(f"-{short.upper()}", f"--{long}-port", type=int, default=port, help=f"Determines the port on which the {name} service runs. Only relevant if we're starting a worker node.")

    parser.add_argument("-f", "--file", default="./docker-compose-$NODE.yml", help="The docker-compose file to execute. You can use '$NODE' to use the current node identifier (i.e., 'central' or 'worker').")
    parser.add_argument("-v", "--verbose", action="store_true", help="If given, provides additional updates on what's happening.")
    parser.add_argument("-V", "--version", action="store_true", help="If given, returns the version of Brane for which this script works.")

    # Parse the arguments
    args = parser.parse_args()
    if args.COMMAND == "start" and args.NODE == "worker" and args.LOCATION_ID is None: fatal("Missing LOCATION_ID parameter (use --help to learn more)")
    args.packages = args.packages.replace("$CONFIG", args.config)
    args.data     = args.data.replace("$CONFIG", args.config)
    args.results  = args.results.replace("$CONFIG", args.config)
    args.certs    = args.certs.replace("$CONFIG", args.config)
    for service in CENTRAL_SERVICES:
        _, long, _, _ = CENTRAL_SERVICES[service]
        setattr(args, f"{long}_image", getattr(args, f"{long}_image").replace("$IMAGE_DIR", args.image_dir))
    for service in WORKER_SERVICES:
        _, long, _, _ = WORKER_SERVICES[service]
        setattr(args, f"{long}_image", getattr(args, f"{long}_image").replace("$IMAGE_DIR", args.image_dir))
    args.file = args.file.replace("$NODE", args.NODE)

    # Do a potential version write
    if args.version:
        print(f"{VERSION}")
        exit(0)

    # Run main
    exit(main(
        args.COMMAND, args.NODE, args.LOCATION_ID,
        args.config, args.packages, args.data, args.results, args.certs,
        { svc: getattr(args, f"{CENTRAL_SERVICES[svc][1]}_image") for svc in CENTRAL_SERVICES },
        { svc: getattr(args, f"{WORKER_SERVICES[svc][1]}_image") for svc in WORKER_SERVICES },
        { svc: getattr(args, f"{CENTRAL_SERVICES[svc][1]}_port") for svc in CENTRAL_SERVICES if CENTRAL_SERVICES[svc][3] is not None },
        { svc: getattr(args, f"{WORKER_SERVICES[svc][1]}_port") for svc in WORKER_SERVICES if WORKER_SERVICES[svc][3] is not None },
        args.file, args.verbose
    ))
