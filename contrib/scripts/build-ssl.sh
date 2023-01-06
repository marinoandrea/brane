#!/bin/bash
# CONTAINER BUILD.sh
#   by Lut99
#
# Created:
#   18 May 2022, 11:20:02
# Last edited:
#   08 Dec 2022, 14:16:00
# Auto updated?
#   Yes
#
# Description:
#   Builds OpenSSL in a container.
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
# Read the CLI
arch="x86_64"

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
            if [[ "$arg" == "-a" || "$arg" == "--arch" ]]; then
                # PArse the value next iteration
                state="arch"

            elif [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
                # Show the help string
                echo ""
                echo "Usage: $0 [opts]"
                echo ""
                echo "This script builds an OpenSSL instance for this machine."
                echo ""
                echo "Note that it's designed to be run in a container (see 'Dockerfile.ssl')."
                echo ""
                echo "Options:"
                echo "  -a,--arch <ARCH>       Determines the architecture to build for. Can be 'x86_64' or 'aarch64'"
                echo "                         (Default: 'x86_64')"
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
            echo "Unknown positional '$arg' at index $pos_i"
            errored=1

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
# if [[ -z "$mode" ]]; then
#     echo "No mode given; nothing to do."
#     errored=1
# fi

# If an error occurred, go no further
if [[ "$errored" -ne 0 ]]; then
    exit 1
fi





### BUILDING ###
# Create the musl binary directories with links
exec_step ln -s "/usr/include/$arch-linux-gnu/asm" "/usr/include/$arch-linux-musl/asm"
exec_step ln -s "/usr/include/asm-generic" "/usr/include/$arch-linux-musl/asm-generic"
exec_step ln -s "/usr/include/linux" "/usr/include/$arch-linux-musl/linux"
exec_step mkdir /musl

# Get the source
exec_step wget https://github.com/openssl/openssl/archive/OpenSSL_1_1_1f.tar.gz
exec_step tar zxvf OpenSSL_1_1_1f.tar.gz 
exec_step cd openssl-OpenSSL_1_1_1f/

# Configure the project
echo " > CC=\"musl-gcc -fPIE -pie\" ./Configure no-shared no-async --prefix=/musl --openssldir=/musl/ssl \"linux-$arch\""
CC="musl-gcc -fPIE -pie" ./Configure no-shared no-async --prefix=/musl --openssldir=/musl/ssl "linux-$arch" || exit "$?"

# Compile it (but not the docs)
make depend
make -j$(nproc)
make install_sw install_ssldirs

# Done, copy the resulting folder to the build one
mkdir -p "/build/target/openssl/$arch"
cp -r /musl/include "/build/target/openssl/$arch"
cp -r /musl/lib "/build/target/openssl/$arch"
