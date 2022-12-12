#!/bin/bash
# CONTAINER BUILD.sh
#   by Lut99
#
# Created:
#   18 May 2022, 11:20:02
# Last edited:
#   12 Dec 2022, 16:28:14
# Auto updated?
#   Yes
#
# Description:
#   Script that builds stuff that has to be build inside containers.
#

# Capture the command line arguments as a separate variable (so we can call the script recursively from within functions)
cli_args=($@)


### HELPER FUNCTIONS ###
# Helper function that executes a build step
exec_step() {
    # Construct a string from the input to show to user
    local cmd=""
    for arg in "$@"; do
        if [[ "$arg" =~ \  ]]; then
            cmd="$cmd \"$arg\""
        else
            cmd="$cmd $arg"
        fi
    done
    echo " >$cmd"

    # Run the call with the error check
    "$@" || exit $?
}





### CLI ###
target=""
arch="x86_64"
development=0

state="start"
pos_i=0
allow_opts=1
errored=0
for arg in "${cli_args[@]}"; do
    # Switch between states
    if [[ "$state" == "start" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            # Match the specific option
            if [[ "$arg" == "-a" || "$arg" == "--arch" ]]; then
                # Go to the arch state
                state="arch"

            elif [[ "$arg" == "--dev" || "$arg" == "--development" ]]; then
                # Simply check it
                development=1

            elif [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
                # Show the help string
                echo ""
                echo "Usage: $0 [opts] <target>"
                echo ""
                echo "Positionals:"
                echo "  <target>               The target to build. Can be any Brane package."
                echo ""
                echo "Options:"
                echo "  -a,--arch <arch>       The architecture for which to compile. Can either be 'x86_64' or 'aarch64'."
                echo "                         Default: 'x86_64'"
                echo "  --dev,--development    If given, compiles the executables in development mode. This includes"
                echo "                         building them in debug mode instead of release and adding '--debug' flags"
                echo "                         to all instance services."
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
            # Match the positional index
            if [[ "$pos_i" -eq 0 ]]; then
                # It's the target
                target="$arg"
            else
                echo "Unknown positional '$arg' at index $pos_i"
                errored=1
            fi

            # Increment the index
            ((pos_i=pos_i+1))
        fi

    elif [[ "$state" == "arch" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            echo "Missing value for '--arch'"
            errored=1

        else
            # Check if any of the allowed values
            if [[ "$arg" != "x86_64" && "$arg" != "aarch64" ]]; then
                echo "Illegal value '$arg' for '--arch' (see '--help')"
                errored=1
                state="start"
                continue
            fi

            # Simply set the value
            arch="$arg"

        fi

        # Reset the state
        state="start"

    else
        echo "ERROR: Unknown state '$state'"
        exit 1

    fi
done

# If we're not in a start state, we didn't exist cleanly (missing values)
if [[ "$state" == "arch" ]]; then
    echo "Missing value for '--arch'"
    errored=1

elif [[ "$state" != "start" ]]; then
    echo "ERROR: Unknown state '$state'"
    exit 1
fi

# Check if mandatory variables are given
if [[ -z "$target" ]]; then
    echo "No target specified; nothing to do."
    exit 0
fi

# If an error occurred, go no further
if [[ "$errored" -ne 0 ]]; then
    exit 1
fi





### BUILDING ###
# Navigate the correct workspace folder
exec_step cd /build

# Make sure there is a target/containers folder
exec_step mkdir -p ./target/containers

# Prepare the release flag or not
rls_flag=""
rls_dir="debug"
if [[ "$development" -ne 1 ]]; then
    rls_flag="--release"
    rls_dir="release"
fi

# Compile with cargo, setting the appropriate workspace folder
echo " > cargo build \\"
echo "       $rls_flag \\"
echo "       --target-dir ./target/containers \\"
echo "       --package $target"
cargo build \
    $rls_flag \
    --target-dir ./target/containers \
    --package "$target" \
    || exit $?

# Done
echo "Compiled $target ($arch) to '/build/target/containers/$arch-unknown-linux-musl/$rls_dir/$target (unless it has a custom binary name)"
