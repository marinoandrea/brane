# SEND BINS.sh
#   by Lut99
#
# Created:
#   06 Nov 2022, 12:28:51
# Last edited:
#   06 Nov 2022, 13:25:33
# Auto updated?
#   Yes
#
# Description:
#   Simple script that uses `rsync` to send compiled parts of the framework
#   over to another server, where they may be run with only Docker load
#   times.
#


### CLI ###
# Read the CLI
what=""
where=""
dev=""
arch=""
no_build=0

state="start"
pos_i=0
allow_opts=1
errored=0
for arg in "$@"; do
    # Switch between states
    if [[ "$state" == "start" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            # Match the specific option
            if [[ "$arg" == "--dev" ]]; then
                # Set that we've seen it
                dev="--dev"

            elif [[ "$arg" == '-a' || "$arg" == "--arch" ]]; then
                # Move to the state to parse it
                state="arch"

            elif [[ "$arg" == "-n" || "$arg" == "--no-build" ]]; then
                # Set that we've seen it
                no_build=1

            elif [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
                # Show the help string
                echo ""
                echo "Usage: $0 [opts] <what> <where>"
                echo ""
                echo "Arguments:"
                echo "  <what>                 The set of binaries to send over. Use 'instance' to send images and stuff"
                echo "                         for a central node (i.e., 'brane-api', 'brane-drv' and 'brane-plr'), and"
                echo "                         use 'worker-instance' to send the set for a worker node (i.e., 'brane-reg'"
                echo "                         and 'brane-job'). In both cases, auxillary Dockerfiles and compose files"
                echo "                         are sent as well."
                echo "  <where>                Some hostname to send the files to. They will be placed under '~/brane' on"
                echo "                         that host. You can use an SSH-compatible hostname (the ones from 'config'"
                echo "                         work as well."
                echo ""
                echo "Options:"
                echo "     --dev               Builds the instance binaries in development mode instead of release mode."
                echo "  -a,--arch <ARCH>       The architecture to build the container binaries for. Can be 'x86_64' or"
                echo "                         'aarch64' (default: native architecture)"
                echo "  -n,--no-build          Does not call upon 'make.py' to built the instance binaries before sending"
                echo "                         them."
                echo "  -h,--help              Shows this help menu, then quits."
                echo "  --                     Any following values are interpreted as-is instead of as options."
                echo ""

                # Done, quit
                exit 0

            elif [[ "$arg" == "--" ]]; then
                # No longer allow options
                allow_opts=0

            else
                echo "Unknown option '$arg'"
                errored=1
            fi
        
        else
            if [[ "$pos_i" -eq 0 ]]; then
                # Store the mode; it's validity will be checked when switching on it
                what="$arg"

            elif [[ "$pos_i" -eq 1 ]]; then
                # Store the location ID - which may also be the file in another setting
                where="$arg"

            else
                echo "Unknown positional '$arg' at index $pos_i"
                errored=1
            fi

            # Increment the index
            ((pos_i=pos_i+1))
        fi

    elif [[ "$state" == "arch" ]]; then
        # Check if the current one is a deny-options; if so, try again
        if [[ "$allow_opts" -eq 1 && "$arg" == "--" ]]; then
            # No longer allow options
            allow_opts=0
            continue
        elif [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            # It's an option
            echo "Missing value for '--$state'"
            errored=1
        fi

        # Otherwise, match on the specific value to find where to store it
        if [[ "$state" == "arch" ]]; then
            arch="$arg"
        fi

        # Regardless, move back to the normal state
        state="start"

    else
        echo "ERROR: Unknown state '$state'"
        exit 1

    fi
done

# If we're not in a start state, we didn't exist cleanly
if [[ "$state" != "start" ]]; then
    echo "ERROR: Unknown state '$state'"
    exit 1
fi

# # Check if mandatory variables are given
if [[ -z "$what" ]]; then
    echo "No 'what' given; nothing to do."
    errored=1
elif [[ -z "$where" ]]; then
    echo "No 'where' given; nothing to do."
    errored=1
fi

# If an error occurred, go no further
if [[ "$errored" -ne 0 ]]; then
    exit 1
fi





### SENDING ###
# Move the config dir to be relative to the general folder, for make
scriptpath="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
cd "$scriptpath/../.."

# Match on what to send
if [[ "$what" == "instance" ]]; then
    # Resolve some arguments
    barch=""
    if [[ ! -z "$arch" ]]; then barch="--arch $arch"; fi
    rdev="release"
    if [[ ! -z "$dev" ]]; then rdev="debug"; fi
    rarch=""
    if [[ ! -z "$arch" ]]; then rarch="$arch/"; fi

    # Build the instance first
    if [[ "$no_build" -ne 1 ]]; then
        echo "Building with './make.py instance $dev $barch'"
        ./make.py instance $dev $barch || exit $?
    fi

    # Copy meta files
    echo "Sending meta files to '$where'..."
    for svc in brane-*; do
        rsync -avr --progress --rsync-path="mkdir -p \"\$HOME/brane/$svc\" && rsync" "./$svc/Cargo.toml" "$where":"brane/$svc/" || exit $?
    done

    # Copy the scripts and junk
    echo "Sending scripts to '$where'..."
    rsync -avr --progress "./make.py" "./docker-compose-central.yml" "$where":"brane" || exit $?

    # Copy all config folders
    echo "Sending config folders to '$where'..."
    rsync -avr --progress ./config* "$where":"brane/" || exit $?

    # Copy the binaries using rsync
    echo "Sending binaries to '$where'..."
    rsync -avr --progress --rsync-path="mkdir -p \"\$HOME/brane/target/${rarch}release\" && rsync" ./target/${rarch}release/brane-xenon.tar "$where":"brane/target/${rarch}release/" || exit $?
    rsync -avr --progress --rsync-path="mkdir -p \"\$HOME/brane/target/$rarch$rdev\" && rsync" ./target/$rarch$rdev/{brane-api,brane-drv,brane-plr}.tar "$where":"brane/target/$rarch$rdev/" || exit $?

elif [[ "$where" == "worker-instance" ]]; then
    # TBD
    echo "TBD"

fi
