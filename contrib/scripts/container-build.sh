#!/bin/bash
# CONTAINER BUILD.sh
#   by Lut99
#
# Created:
#   18 May 2022, 11:20:02
# Last edited:
#   22 May 2022, 15:57:41
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
                echo "  <target>               The target to build. Can be 'branelet'."
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
# Switch on the target
if [[ "$target" == "branelet" ]]; then
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
    echo "       --target \"$arch-unknown-linux-musl\""
    echo "       $rls_flag \\"
    echo "       --target-dir ./target/containers \\"
    echo "       --package brane-let"
    cargo build \
        --target "$arch-unknown-linux-musl" \
        $rls_flag \
        --target-dir ./target/containers \
        --package brane-let \
        || exit $?

    # Done
    echo "Compiled branelet ($arch) to '/build/target/containers/$arch-unknown-linux-musl/$rls_dir/branelet"

elif [[ "$target" == "openssl" ]]; then
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

else
    echo "Unknown target '$target'"
    exit 1

fi
