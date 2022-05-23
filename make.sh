#!/bin/bash
# MAKE.sh
#   by Lut99
#
# Created:
#   03 Mar 2022, 17:03:04
# Last edited:
#   23 May 2022, 21:26:40
# Auto updated?
#   Yes
#
# Description:
#   Custom "Makefile" for the Brane project.
#   Not using GNU Make because it doesn't really understand the concept of
#   not rebuilding images when not needed.
#


##### CONSTANTS #####
# Determines the location of the file state cache
CACHE_DIR=./target/make_cache

# The crates part of the Brane instance source code
BRANE_INSTANCE_SRC=(./brane-api ./brane-bvm ./brane-cfg ./brane-clb ./brane-drv ./brane-dsl ./brane-job ./brane-plr ./brane-shr ./specifications)
# The images part of the Brane instance
BRANE_INSTANCE_IMAGES=(brane-xenon brane-format brane-api brane-clb brane-drv brane-job brane-log brane-plr)
# The services part of the Brane instance
BRANE_INSTANCE_SERVICES=(aux-scylla aux-registry aux-zookeeper aux-kafka brane-xenon aux-minio aux-redis once-format brane-api brane-clb brane-drv brane-job brane-log brane-plr)

# The timeout (in seconds) before we consider a spawned service a failure
BRANE_INSTANCE_SERVICE_TIMEOUT=60

# The to-be-replaced string for the cluster domain name
K8S_DOMAIN_REPLACE="%BRANE_CLUSTER_DOMAIN%"
# The to-be-replaced string for the data volume name
K8S_DATA_SC_REPLACE="%BRANE_DATA_STORAGE%"
# The to-be-replaced string for the config volume name
K8S_CONFIG_SC_REPLACE="%BRANE_CONFIG_STORAGE%"

# The host arch
HOST_ARCH=$(uname -m)
if [[ "$HOST_ARCH" == "amd64" ]]; then HOST_ARCH="x86_64"; fi
if [[ "$HOST_ARCH" == "arm64" ]]; then HOST_ARCH="aarch64"; fi

# Lists the generated targets of OpenSSL
OPENSSL_DIR="$(pwd)/target/openssl/$HOST_ARCH"
OPENSSL_TARGETS=("$OPENSSL_DIR/lib/libcrypto.a" "$OPENSSL_DIR/lib/libssl.a" \
                "$OPENSSL_DIR/lib/pkgconfig/libcrypto.pc" "$OPENSSL_DIR/lib/pkgconfig/libssl.pc" "$OPENSSL_DIR/lib/pkgconfig/openssl.pc"
                "$OPENSSL_DIR/include/openssl/aes.h" "$OPENSSL_DIR/include/openssl/asn1err.h" "$OPENSSL_DIR/include/openssl/asn1.h"
                "$OPENSSL_DIR/include/openssl/asn1_mac.h" "$OPENSSL_DIR/include/openssl/asn1t.h" "$OPENSSL_DIR/include/openssl/asyncerr.h"
                "$OPENSSL_DIR/include/openssl/async.h" "$OPENSSL_DIR/include/openssl/bioerr.h" "$OPENSSL_DIR/include/openssl/bio.h"
                "$OPENSSL_DIR/include/openssl/blowfish.h" "$OPENSSL_DIR/include/openssl/bnerr.h" "$OPENSSL_DIR/include/openssl/bn.h"
                "$OPENSSL_DIR/include/openssl/buffererr.h" "$OPENSSL_DIR/include/openssl/buffer.h" "$OPENSSL_DIR/include/openssl/camellia.h"
                "$OPENSSL_DIR/include/openssl/cast.h" "$OPENSSL_DIR/include/openssl/cmac.h" "$OPENSSL_DIR/include/openssl/cmserr.h"
                "$OPENSSL_DIR/include/openssl/cms.h" "$OPENSSL_DIR/include/openssl/comperr.h" "$OPENSSL_DIR/include/openssl/comp.h"
                "$OPENSSL_DIR/include/openssl/conf_api.h" "$OPENSSL_DIR/include/openssl/conferr.h" "$OPENSSL_DIR/include/openssl/conf.h"
                "$OPENSSL_DIR/include/openssl/cryptoerr.h" "$OPENSSL_DIR/include/openssl/crypto.h" "$OPENSSL_DIR/include/openssl/cterr.h"
                "$OPENSSL_DIR/include/openssl/ct.h" "$OPENSSL_DIR/include/openssl/des.h" "$OPENSSL_DIR/include/openssl/dherr.h"
                "$OPENSSL_DIR/include/openssl/dh.h" "$OPENSSL_DIR/include/openssl/dsaerr.h" "$OPENSSL_DIR/include/openssl/dsa.h"
                "$OPENSSL_DIR/include/openssl/dtls1.h" "$OPENSSL_DIR/include/openssl/ebcdic.h" "$OPENSSL_DIR/include/openssl/ecdh.h"
                "$OPENSSL_DIR/include/openssl/ecdsa.h" "$OPENSSL_DIR/include/openssl/ecerr.h" "$OPENSSL_DIR/include/openssl/ec.h"
                "$OPENSSL_DIR/include/openssl/engineerr.h" "$OPENSSL_DIR/include/openssl/engine.h" "$OPENSSL_DIR/include/openssl/e_os2.h"
                "$OPENSSL_DIR/include/openssl/err.h" "$OPENSSL_DIR/include/openssl/evperr.h" "$OPENSSL_DIR/include/openssl/evp.h"
                "$OPENSSL_DIR/include/openssl/hmac.h" "$OPENSSL_DIR/include/openssl/idea.h" "$OPENSSL_DIR/include/openssl/kdferr.h"
                "$OPENSSL_DIR/include/openssl/kdf.h" "$OPENSSL_DIR/include/openssl/lhash.h" "$OPENSSL_DIR/include/openssl/md2.h"
                "$OPENSSL_DIR/include/openssl/md4.h" "$OPENSSL_DIR/include/openssl/md5.h" "$OPENSSL_DIR/include/openssl/mdc2.h"
                "$OPENSSL_DIR/include/openssl/modes.h" "$OPENSSL_DIR/include/openssl/objectserr.h" "$OPENSSL_DIR/include/openssl/objects.h"
                "$OPENSSL_DIR/include/openssl/obj_mac.h" "$OPENSSL_DIR/include/openssl/ocsperr.h" "$OPENSSL_DIR/include/openssl/ocsp.h"
                "$OPENSSL_DIR/include/openssl/opensslconf.h" "$OPENSSL_DIR/include/openssl/opensslv.h" "$OPENSSL_DIR/include/openssl/ossl_typ.h"
                "$OPENSSL_DIR/include/openssl/pem2.h" "$OPENSSL_DIR/include/openssl/pemerr.h" "$OPENSSL_DIR/include/openssl/pem.h"
                "$OPENSSL_DIR/include/openssl/pkcs12err.h" "$OPENSSL_DIR/include/openssl/pkcs12.h" "$OPENSSL_DIR/include/openssl/pkcs7err.h"
                "$OPENSSL_DIR/include/openssl/pkcs7.h" "$OPENSSL_DIR/include/openssl/rand_drbg.h" "$OPENSSL_DIR/include/openssl/randerr.h"
                "$OPENSSL_DIR/include/openssl/rand.h" "$OPENSSL_DIR/include/openssl/rc2.h" "$OPENSSL_DIR/include/openssl/rc4.h"
                "$OPENSSL_DIR/include/openssl/rc5.h" "$OPENSSL_DIR/include/openssl/ripemd.h" "$OPENSSL_DIR/include/openssl/rsaerr.h"
                "$OPENSSL_DIR/include/openssl/rsa.h" "$OPENSSL_DIR/include/openssl/safestack.h" "$OPENSSL_DIR/include/openssl/seed.h"
                "$OPENSSL_DIR/include/openssl/sha.h" "$OPENSSL_DIR/include/openssl/srp.h" "$OPENSSL_DIR/include/openssl/srtp.h"
                "$OPENSSL_DIR/include/openssl/ssl2.h" "$OPENSSL_DIR/include/openssl/ssl3.h" "$OPENSSL_DIR/include/openssl/sslerr.h"
                "$OPENSSL_DIR/include/openssl/ssl.h" "$OPENSSL_DIR/include/openssl/stack.h" "$OPENSSL_DIR/include/openssl/storeerr.h"
                "$OPENSSL_DIR/include/openssl/store.h" "$OPENSSL_DIR/include/openssl/symhacks.h" "$OPENSSL_DIR/include/openssl/tls1.h"
                "$OPENSSL_DIR/include/openssl/tserr.h" "$OPENSSL_DIR/include/openssl/ts.h" "$OPENSSL_DIR/include/openssl/txt_db.h"
                "$OPENSSL_DIR/include/openssl/uierr.h" "$OPENSSL_DIR/include/openssl/ui.h" "$OPENSSL_DIR/include/openssl/whrlpool.h"
                "$OPENSSL_DIR/include/openssl/x509err.h" "$OPENSSL_DIR/include/openssl/x509.h" "$OPENSSL_DIR/include/openssl/x509v3err.h"
                "$OPENSSL_DIR/include/openssl/x509v3.h" "$OPENSSL_DIR/include/openssl/x509_vfy.h")

# Capture the command line arguments as a separate variable (so we can call the script recursively from within functions)
cli_args=($@)





##### HELPER FUNCTIONS #####
# Helper function that executes a recursive script call
make_target() {
    # Make sure there is at least one target
    if [[ "$#" -lt 1 ]]; then
        echo "Usage: make_target <target> [opts...]"
        exit 1
    fi

    # Run the recursive call with the error check
    ./make.sh "$1" ${cli_args[@]:1} "${@:2}" || exit $?
}

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

# Helper function that checks if we need to generate a particular file
should_regen() {
    # Make sure we're called with only one argument
    if [[ "$#" -ne 1 ]]; then
        echo "Usage: should_regen <file>"
        exit 1
    fi
    file="$1"

    # Use recursive calls for all files in a folder if it's a folder
    if [[ -d "$file" ]]; then
        # Return that we should regenerate if any of the sub-files need to
        for target in "$file"/*; do
            if should_regen "$target"; then return 0; fi
        done
        return 1
    fi

    # Resolve the cache file location
    if [[ ! "$file" =~ ^\./ ]]; then
        echo "should_regen() only works for relative paths (i.e., beginning with './')"
        exit 1
    fi
    cache_file=${file//\.\//$CACHE_DIR\/}

    # We always regen if the file or cache file does not exist
    if [[ ! -f "$file" || ! -f "$cache_file" ]]; then return 0; fi
    
    # If it does, load and compare with the actual file hash
    file_hash=$(sha256sum "$file" | cut -d " " -f1)
    cache_hash=$(cat "$cache_file")
    if [[ "$file_hash" == "$cache_hash" ]]; then
        return 1
    else
        return 0
    fi
}

# Helper function that caches the hash of the given file so we may check if we need to regenerate it
cache_regen() {
    # Make sure we're called with only one argument
    if [[ "$#" -ne 1 ]]; then
        echo "Usage: cache_regen <file>"
        exit 1
    fi
    file="$1"

    # Use recursive calls for all files in a folder if it's a folder
    if [[ -d "$file" ]]; then
        # Return that we should regenerate if any of the sub-files need to
        for target in "$file"/*; do
            cache_regen "$target"
        done
        return
    fi

    # Resolve the cache file location
    if [[ ! "$file" =~ ^\./ ]]; then
        echo "cache_regen() only works for relative paths (i.e., beginning with './')"
        exit 1
    fi
    cache_file=${file//\.\//$CACHE_DIR\/}

    # Create the cache dir if it does not yet exist
    mkdir -p "$(dirname "$cache_file")"

    # Compute the file hash and store it
    file_hash=$(sha256sum "$file" | cut -d " " -f1)
    echo "$file_hash" > "$cache_file"
}

# Blocks until a given service is 'ready' according to kubectl
block_until_ready() {
    # Make sure we're called with only one argument
    if [[ "$#" -ne 1 ]]; then
        echo "Usage: block_until_read <service>"
        exit 1
    fi
    svc="$1"

    # Simply check once every half second for '1/1' message
    ready=""
    time_taken=0
    while [[ ! "$ready" =~ 1/1 ]]; do
        sleep 0.5
        ready="$(kubectl -n brane-control get deploy | grep -i "$svc")"
        ((time_taken=time_taken+1))
        if [[ "$((time_taken=time_taken/2))" -gt "$BRANE_INSTANCE_SERVICE_TIMEOUT" ]]; then
            echo "Timeout while waiting for service '$svc' to reach Ready"
            exit 1
        fi
    done
}

# Returns the architecture of the current host
host_arch() {
    # Make sure Docker is started
    err=$(docker info 2>&1)
    res="$?"
    if [[ "$res" -ne 0 ]]; then
        echo "$err" 1>&2
        exit 1
    fi

    # Return the architecture string
    echo "$(docker info | grep Architecture: | awk '{print $2}')"
    exit 0
}





##### CLI PARSING #####
target="local"
precompiled=0
precompiled_source=""
version=""
arch="$(host_arch)"; if [[ -z "$arch" ]]; then exit 1; fi
development=0
containerized=0
cluster_domain="cluster.local"
data_storage_name="brane-data-storage"
config_storage_name="brane-config-storage"
keep_registry=0

state="start"
i=0
pos_i=0
allow_opts=1
errored=0
while [[ "$i" -lt "$#" ]]; do
    arg=${cli_args[i]}

    # Switch between states
    if [[ "$state" == "start" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            # Match the specific option
            if [[ "$arg" == "-a" || "$arg" == "--arch" ]]; then
                # Move to the arch state to parse its value
                state="arch"

            elif [[ "$arg" == "--dev" || "$arg" == "--development" ]]; then
                # Simply check it
                development=1

            elif [[ "$arg" == "-p" || "$arg" == "--precompiled" ]]; then
                # Simply check it, but then check if the next one is the optional argument
                precompiled=1
                state="precompiled"

            elif [[ "$arg" == "-v" || "$arg" == "--version" ]]; then
                # go to the associated state to get the actual value
                state="version"

            elif [[ "$arg" == "-C" || "$arg" == "--containerized" ]]; then
                # Simply check it
                containerized=1

            elif [[ "$arg" == "--targets" ]]; then            
                echo ""
                echo "Meta targets:"
                echo "  local        Compiles a release instance and a release CLI tool for single-machine use."
                echo "  k8s          Compiles a release instance and a release CLI tool for deployment on a Kubernetes"
                echo "               cluster."
                echo "  clean        Clears everything build by this script (except for Docker images)."

            elif [[ "$arg" == "-D" || "$arg" == "--cluster-domain" ]]; then
                # Do again in the next iteration
                state="cluster-domain"

            elif [[ "$arg" == "-d" || "$arg" == "--data-storage-name" ]]; then
                # Do again in the next iteration
                state="data-storage-name"

            elif [[ "$arg" == "-c" || "$arg" == "--config-storage-name" ]]; then
                # Do again in the next iteration
                state="config-storage-name"

            elif [[ "$arg" == "-k" || "$arg" == "--keep-registry" ]]; then
                keep_registry=1

            elif [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
                # Show the help string
                echo ""
                echo "Usage: $0 [opts] [<target>]"
                echo ""
                echo "Positionals:"
                echo "  <target>               The target to build. For a list of possible targets, check '--targets'. If"
                echo "                         omitted, defaults to 'local'."
                echo ""
                echo "Options:"
                echo "  -a,--arch <arch>       The architecture for which to compile. Options are: 'x86_64' and 'aarch64'."
                echo "                         Default: 'x86_64'"
                echo "  --dev,--development    If given, compiles the Brane instance (and other executables) in"
                echo "                         development mode. This includes building them in debug mode instead of"
                echo "                         release, faster instance build times by building on-disk and adding"
                echo "                         '--debug' flags to all instance services."
                echo "  -C,--containerized     If given, builds musl-targets in a container instead of via cross-"
                echo "                         compilation on the host. This should only be desired in building on a mac"
                echo "                         with Apple Silicon."
                echo "  -p,--precompiled [<path>]"
                echo "                         If given, uses precompiled binaries instead of compiling them. If no path"
                echo "                         is given, then the binaries are downloaded from the repository; otherwise,"
                echo "                         the ones given by the path are used. For the CLI, the path should point to"
                echo "                         the executable directly; for the instance, this should point to the"
                echo "                         containing them."
                echo "  -v,--version <version> If '--precompiled' is given and this option is, then the script will"
                echo "                         download the binaries of the specific Brane version. Otherwise, the latest"
                echo "                         version will be used. Note: give as a triplet of values, without"
                echo "                         preprending 'v' (e.g., '1.0.0')."
                echo "     --targets           Lists all possible targets in the make script, then quits."
                echo "  -D,--cluster-domain <name>"
                echo "                         The domain name of the cluster where the Brane control plane services live."
                echo "                         Default: 'cluster.local'"
                echo "  -d,--data-storage-name <name>"
                echo "                         The name of the storage class where the '/data' volume will be stored."
                echo "                         Default: 'brane-data-storage'"
                echo "  -c,--config-storage-name <name>"
                echo "                         The name of the storage class that contains the infra.yml and secrets.yml"
                echo "                         files. Default: 'brane-config-storage'"
                echo "  -k,--keep-registry     If given, does not delete the registry in a remote Kubernetes environment"
                echo "                         when running 'stop-instance-k8s'."
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
            # Make sure the architectures are valid
            if [[ "$arg" != "x86_64" && "$arg" != "aarch64" ]]; then
                echo "Unknown architecture '$arg'"
                errored=1
            else
                # Simply set it
                arch="$arg"
            fi

        fi

        # Move back to the main state
        state="start"

    elif [[ "$state" == "precompiled" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            # It's an option; retry the value
            ((i=i-1))
        else
            # Store the path
            precompiled_source="$arg"
        fi

        # Move back to the main state
        state="start"

    elif [[ "$state" == "version" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            echo "Missing value for '--arch'"
            errored=1

        else
            # Store the version
            version="$arg"

        fi

        # Move back to the main state
        state="start"

    elif [[ "$state" == "cluster-domain" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            echo "Missing value for '--cluster-domain'"
            errored=1

        else
            # Simply set it
            cluster_domain="$arg"

        fi

        # Move back to the main state
        state="start"

    elif [[ "$state" == "data-storage-name" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            echo "Missing value for '--data-storage-name'"
            errored=1

        else
            # Simply set it
            data_storage_name="$arg"

        fi

        # Move back to the main state
        state="start"

    elif [[ "$state" == "config-storage-name" ]]; then
        # Switch between option or not
        if [[ "$allow_opts" -eq 1 && "$arg" =~ ^- ]]; then
            echo "Missing value for '--config-storage-name'"
            errored=1

        else
            # Simply set it
            config_storage_name="$arg"

        fi

        # Move back to the main state
        state="start"

    else
        echo "ERROR: Unknown state '$state'"
        exit 1

    fi

    # Increment the loop variable
    ((i=i+1))
done

# If we're not in a start state, we didn't exist cleanly (missing values)
if [[ "$state" == "arch" ]]; then
    echo "Missing value for '--arch'"
    errored=1

elif [[ "$state" == "version" ]]; then
    echo "Missing value for '--version'"
    errored=1

elif [[ "$state" == "cluster-domain" ]]; then
    echo "Missing value for '--cluster-domain'"
    errored=1

elif [[ "$state" == "data-storage-name" ]]; then
    echo "Missing value for '--data-storage-name'"
    errored=1

elif [[ "$state" == "config-storage-name" ]]; then
    echo "Missing value for '--config-storage-name'"
    errored=1

elif [[ "$state" != "start" && "$state" != "precompiled" ]]; then
    echo "ERROR: Unknown state '$state'"
    exit 1
fi

# Check if mandatory variables are given

# If an error occurred, go no further
if [[ "$errored" -ne 0 ]]; then
    exit 1
fi





##### TARGETS #####
### META TARGETS ###
# Build every relevant thing for a typical user
if [[ "$target" == "local" ]]; then
    # Use recursive calls to deal with it
    make_target instance
    make_target cli

# Build every relevant thing for a Kubernetes deployment
elif [[ "$target" == "k8s" ]]; then
    # Remove the target folder
    make_target instance
    make_target cli

# Clean the standard build folder
elif [[ "$target" == "clean" ]]; then
    # Remove the target folder
    exec_step rm -rf ./target



### BINARIES ###
# Build the command-line interface 
elif [[ "$target" == "cli" ]]; then
    # Decide the release flags and dir based on the development flag
    rls_flags="--release"
    rls_dir="release"
    if [[ "$development" -eq 1 ]]; then
        rls_flags=""
        rls_dir="debug"
    fi

    # Switch between downloading or not
    if [[ "$precompiled" -eq 1 ]]; then
        # Switch between using a path or not
        if [[ -z "$precompiled_source" ]]; then
            # Resolve the architecture with the OS info
            if [[ $OSTYPE =~ ^darwin ]]; then
                # It's macos
                arch_os="darwin-$arch"
            else
                # It's linux
                arch_os="linux-$arch"
            fi

            # Try to download using wget (more generally available)
            if [[ -z "$version" ]]; then
                exec_step wget -O "./target/$rls_dir/brane" "https://github.com/epi-project/brane/releases/latest/download/brane-$arch_os"
            else
                exec_step wget -O "./target/$rls_dir/brane" "https://github.com/epi-project/brane/releases/download/v$version/brane-$arch_os"
            fi
            # Make it executable
            exec_step chmod +x "./target/$rls_dir/brane"

            # Done
            echo "Downloaded executeable \"brane\" to './target/$rls_dir/brane'"

        else
            # Error if not a file
            if [[ ! -f "$precompiled_source" ]]; then
                echo "Precompiled CLI binary '$precompiled_source' does not point to a file"
                exit 1
            fi

            # Simply copy it
            exec_step cp "$precompiled_source" "./target/$rls_dir/brane"
            # Make it executable
            exec_step chmod +x "./target/$rls_dir/brane"

            # Done
            echo "Copied executeable \"brane\" from '$precompiled_source' to './target/$rls_dir/brane'"
        fi
    else
        # Switch between the normal or development build
        exec_step cargo build $rls_flags --package brane-cli
        echo "Compiled executeable \"brane\" to './target/$rls_dir/brane'"
    fi

    # Done

# Build the branelet executable by cross-compiling
elif [[ "$target" == "branelet" ]]; then
    # Split on how to compile
    if [[ "$containerized" -eq 0 ]]; then
        # Prepare flags to build in development mode or not
        rls_flag="--release"
        rls_dir="release"
        if [[ "$development" -eq 1 ]]; then
            rls_flag=""
            rls_dir="debug"
        fi

        # We let cargo sort out dependencies
        exec_step rustup target add "$arch-unknown-linux-musl"
        exec_step cargo build $rls_flag --package brane-let --target "$arch-unknown-linux-musl"

        # Done
        echo "Compiled package initialization binary \"branelet\" ($arch) to './target/$arch-unknown-linux-musl/$rls_dir/branelet'"

    else
        # Prepare flags to build in development mode or not
        rls_flag=""
        rls_dir="release"
        if [[ "$development" -eq 1 ]]; then
            rls_flag="--dev"
            rls_dir="debug"
        fi

        # Make sure the dev image exists
        make_target bld-image-dev --arch "$(host_arch)"

        # Call upon the container to do the heavy lifting
        exec_step docker run -it --name "brane-bld" -v "$(pwd):/build" --rm brane-bld branelet --arch "$arch" "$rls_flag"

        # If done, reset the folder permissions
        echo "Removing root ownership from target folder (might require sudo password)"
        exec_step sudo chown -R "$(id -u)":"$(id -g)" ./target

        # Copy the files to the correctly nested folder
        exec_step mkdir -p "./target/$arch-unknown-linux-musl/$rls_dir/"
        exec_step cp "./target/containers/$arch-unknown-linux-musl/$rls_dir/branelet" "./target/$arch-unknown-linux-musl/$rls_dir/branelet"

        # Done
        echo "Compiled package initialization binary \"branelet\" ($arch) to './target/$arch-unknown-linux-musl/$rls_dir/branelet'"
    fi



### IMAGES ###
# Build the xenon image
elif [[ "$target" == "xenon-image" ]]; then
    # Call upon Docker to build it (it tackles caches)
    # The Xenon image specifically is build on Java, so doing it cross-arch should be relatively straightforward
    exec_step docker build --load --platform "linux/$arch" -t brane-xenon -f ./contrib/images/Dockerfile.xenon ./contrib/images

    # Done
    echo "Built xenon image ($arch) to Docker Image 'brane-xenon'"

# Build the format image
elif [[ "$target" == "format-image" ]]; then
    # Translate the architecture names to proper ones
    if [[ "$arch" == "x86_64" ]]; then
        juicefs_arch="amd64"
    elif [[ "$arch" == "aarch64" ]]; then
        juicefs_arch="arm64"
    else
        # Sanity check
        echo "Unknown architecture '$arch' (for format image)"
        exit 1
    fi

    # Call upon Docker to build it (it tackles caches)
    # This image downloads a binary, so we'll have to compile from source instead
    exec_step docker build --build-arg "ARCH=$juicefs_arch" --load --platform "linux/$arch" -t brane-format -f ./contrib/images/Dockerfile.juicefs ./contrib/images

    # Done
    echo "Built Brane JuiceFS format image ($arch/$juicefs_arch) to Docker Image 'brane-format'"

# Build musl stuff in a Docker container
elif [[ "$target" == "bld-image-dev" ]]; then
    # Call upon Docker to build it (it tackles caches)
    exec_step docker build --load --platform "linux/$arch" -t brane-bld -f ./contrib/images/Dockerfile.build .

    # Done
    echo "Built Brane build image to Docker Image 'brane-bld'"

# Build the regular images
elif [[ "$target" =~ -image$ ]]; then
    # Get the name of the image
    image_name="${target%-image}"

    # Call upon Docker to build it (building in release as normal does not use any caching other than the caching of the image itself, sadly)
    exec_step docker build --load --platform "linux/$arch" -t "brane-$image_name" --target "brane-$image_name" -f Dockerfile.rls .

    # Done
    echo "Built $image_name image to Docker Image 'brane-$image_name'"

# Build the dev version of the images
elif [[ "$target" =~ -image-dev$ ]]; then
    # Get the name of the image
    image_name="${target%-image-dev}"

    # Call upon Docker to build it (we let it deal with caching)
    exec_step docker build --load --platform "linux/$arch" -t "brane-$image_name" --target "brane-$image_name" --build-arg "ARCH=$arch" -f Dockerfile.dev .

    # Done
    echo "Built $image_name development image to Docker Image 'brane-$image_name'"

# Build the version of the images that uses precompiled binaries
elif [[ "$target" =~ -image-bin$ ]]; then
    # Get the name of the image
    image_name="brane-${target%-image-bin}"

    # Based on the source, download it or make sure it exists
    exec_step mkdir -p "./.container-bins/$arch"
    if [[ -z "$precompiled_source" ]]; then
        # Try to download using wget (more generally available)
        if [[ -z "$version" ]]; then
            exec_step wget -O "./.container-bins/$arch/$image_name" "https://github.com/epi-project/brane/releases/latest/download/$image_name-$arch"
        else
            exec_step wget -O "./.container-bins/$arch/$image_name" "https://github.com/epi-project/brane/releases/download/v$version/$image_name-$arch"
        fi
        precompiled_source="./.container-bins/$arch"

    elif [[ ! -f "$precompiled_source/$image_name" ]]; then
        echo "Precompiled instance binary '$precompiled_source/$image_name' is not a file"
        exit 1
    fi

    # Call upon Docker to build it (we let it deal with further caching)
    exec_step docker build --load --platform "linux/$arch" -t "$image_name" --target "$image_name" --build-arg SOURCE="$precompiled_source/$image_name" -f Dockerfile.bin .

    # Done
    echo "Built $image_name image (from precompiled binary) to Docker Image '$image_name'"

# Target that bundles all the normal images together
elif [[ "$target" == "images" ]]; then
    # Simply build the images
    make_target xenon-image
    make_target format-image
    make_target api-image
    make_target clb-image
    make_target drv-image
    make_target job-image
    make_target log-image
    make_target plr-image

# Target that bundles all the development images together
elif [[ "$target" == "images-dev" ]]; then
    # Simply build the images
    make_target xenon-image
    make_target format-image
    make_target api-image-dev
    make_target clb-image-dev
    make_target drv-image-dev
    make_target job-image-dev
    make_target log-image-dev
    make_target plr-image-dev

# Target that bundles all the precompiled images together
elif [[ "$target" == "images-bin" ]]; then
    # Simply build the images
    make_target xenon-image
    make_target format-image
    make_target api-image-bin
    make_target clb-image-bin
    make_target drv-image-bin
    make_target job-image-bin
    make_target log-image-bin
    make_target plr-image-bin



### OPENSSL ###
# Build OpenSSL
elif [[ "$target" == "openssl" ]]; then
    # Prepare the build image for the SSL
    make_target bld-image-dev

    # Compile the OpenSSL library
    exec_step docker run --attach STDIN --attach STDOUT --attach STDERR --rm -v "$(pwd):/build" brane-bld openssl --arch "$arch"

    # Restore the permissions
	echo "Removing root ownership from target folder (might require sudo password)"
	exec_step sudo chown -R "$(id -u)":"$(id -g)" ./target

    # Done
	echo "Compiled Docker cross-compilation OpenSSL library to '$OPENSSL_DIR'"



### INSTANCE ###
# Builds the instance (which is just building the normal images OR cross-compilation, based on $development)
elif [[ "$target" == "instance" ]]; then
    # Switch on the way that we will build the images
    if [[ "$precompiled" -eq 1 ]]; then
        # Use the provided binaries
        make_target images-bin
        echo "Built Brane instance as Docker images"

    elif [[ "$development" -ne 1 ]]; then
        # We're building release mode
        make_target images
        echo "Built Brane instance as Docker images"

    else
        # Make sure the musl compilers are found
        if ! command -v musl-gcc &> /dev/null; then
            echo "musl-gcc not found; make sure the musl toolchain is installed and available in your PATH"
            exit 1
        elif ! command -v musl-g++ &> /dev/null; then
            echo "musl-g++ not found; make sure the musl toolchain is installed and available in your PATH"
            echo "(It might not provide musl-g++, though. In that case, simply link g++:"
            echo "   $ sudo ln -s /bin/g++ /usr/local/bin/musl-g++"
            echo ")"
            exit 1
        fi

        # Build openssl only if any of the files is not cached
        for target in "${OPENSSL_TARGETS[@]}"; do
            if [[ ! -f "$target" ]]; then
                make_target openssl
                break
            fi
        done

        # Prepare the cross-compilation target
        exec_step rustup target add "$arch-unknown-linux-musl"

        # Compile the framework, pointing to the compiled OpenSSL library
        echo " > OPENSSL_DIR=\"$OPENSSL_DIR\" \\"
        echo "   OPENSSL_LIB_DIR=\"$OPENSSL_DIR\" \\"
        echo "   RUSTFLAGS=\"-C link-arg=-lgcc\" \\"
        echo "   cargo build \\"
        echo "      --target-dir \"./target/containers/target\" \\"
        echo "      --target $arch-unknown-linux-musl \\"
        echo "      --package brane-api \\"
        echo "      --package brane-clb \\"
        echo "      --package brane-drv \\"
        echo "      --package brane-job \\"
        echo "      --package brane-log \\"
        echo "      --package brane-plr"
        OPENSSL_DIR="$OPENSSL_DIR" \
        OPENSSL_LIB_DIR="$OPENSSL_DIR/lib" \
        RUSTFLAGS="-C link-arg=-lgcc" \
        cargo build \
            --target "$arch-unknown-linux-musl" \
            --package brane-api \
            --package brane-clb \
            --package brane-drv \
            --package brane-job \
            --package brane-log \
            --package brane-plr \
            || exit $?

        # Copy the results to the correct location
        exec_step mkdir -p "./.container-bins/$arch"
        exec_step /bin/cp -f "./target/$arch-unknown-linux-musl/debug/brane-api" "./.container-bins/$arch"
        exec_step /bin/cp -f "./target/$arch-unknown-linux-musl/debug/brane-clb" "./.container-bins/$arch"
        exec_step /bin/cp -f "./target/$arch-unknown-linux-musl/debug/brane-drv" "./.container-bins/$arch"
        exec_step /bin/cp -f "./target/$arch-unknown-linux-musl/debug/brane-job" "./.container-bins/$arch"
        exec_step /bin/cp -f "./target/$arch-unknown-linux-musl/debug/brane-log" "./.container-bins/$arch"
        exec_step /bin/cp -f "./target/$arch-unknown-linux-musl/debug/brane-plr" "./.container-bins/$arch"

        # Build the instance images
        make_target images-dev

        # Done!
        echo "Compiled Brane instance as Docker images"
    fi

    # Regardless, update the source file cache status
    for crate in "${BRANE_INSTANCE_SRC[@]}"; do
        cache_regen "$crate"
    done

# Extract binaries from source images
elif [[ "$target" == "extract-binaries" ]]; then
    # Build the images first
    make_target images

    # Do quick onelines to extract the binaries in one go
    exec_step mkdir -p "./.container-bins/$arch"
    for image in brane-api brane-clb brane-drv brane-job brane-log brane-plr; do
        echo " > docker run --entrypoint /bin/echo --name TEMP_CONTAINER \"$image\" \"Hello, world\""
        docker run --entrypoint /bin/echo --name TEMP_CONTAINER "$image" "Hello, world" 2>&1 > /dev/null || exit "$?"
        exec_step docker cp TEMP_CONTAINER:/"$image" "./.container-bins/$arch/"
        echo " > docker rm TEMP_CONTAINER"
        exec_step docker rm TEMP_CONTAINER 2>&1 > /dev/null || exit "$?"
    done

    # Done
    echo "Extracted Brane instance binaries to './.container-bins/$arch'"



### INSTANCE HELPERS ###
# Makes sure the docker network for Brane is up and running
elif [[ "$target" == "ensure-docker-network" ]]; then
    # Only add it if it doesn't exist already
    if [ ! -n "$(docker network ls -f name=brane | grep brane)" ]; then
		exec_step docker network create brane
        echo "Created Docker network 'brane'"
    else
        echo "Docker network 'brane' already exists"
	fi;

# Makes sure that the required infrastructure files are there
elif [[ "$target" == "ensure-configuration" ]]; then
    # Check infra.yml
    if [[ -f ./config/infra.yml ]]; then
        echo "'./config/infra.yml' exists"
    else
        echo "Missing './config/infra.yml'; provide one before running the Brane instance" >&2
        exit 1
    fi

    # Check secrets.yml
    if [[ -f ./config/infra.yml ]]; then
        echo "'./config/secrets.yml' exists"
    else
        echo "Missing './config/secrets.yml'; provide one before running the Brane instance" >&2
        exit 1
    fi



### STARTING/STOPPING ###
# Starts the Brane services (the normal images)
elif [[ "$target" == "start-brn" ]]; then
    # Use Docker compose to start them
    exec_step bash -c "COMPOSE_IGNORE_ORPHANS=1 docker-compose -p brane -f docker-compose-brn.yml up -d"

    # Done
    echo "Started Brane services"

# Stops the Brane services (the normal images)
elif [[ "$target" == "stop-brn" ]]; then
    # Use Docker compose again
    exec_step bash -c "COMPOSE_IGNORE_ORPHANS=1 docker-compose -p brane -f docker-compose-brn.yml down"

    # Done
    echo "Stopped Brane services"

# Starts the instance (from the normal images)
elif [[ "$target" == "start-instance" ]]; then
    # Check if any of the instance source files needs rebuilding
    needs_regen=0
    for crate in "${BRANE_INSTANCE_SRC[@]}"; do
        if should_regen "$crate"; then
            needs_regen=1
            break
        fi
    done

    # Check if we need to rebuild according to missing images
    docker_image=$(docker image list)
    for image in "${BRANE_INSTANCE_IMAGES[@]}"; do
        if [[ -z $(echo "$docker_image" | grep "$image") ]]; then
            needs_regen=1
            break
        fi
    done

    # If we need to rebuild according to source files or images are missing, rebuild the instance
    if [[ "$needs_regen" -eq 1 ]]; then
        make_target instance
    fi

    # Ensure that everything is in order before we start
    make_target ensure-docker-network
    make_target ensure-configuration

    # Start Brane
    make_target start-brn

# Stops the instance (from the normal images)
elif [[ "$target" == "stop-instance" ]]; then
    # Stop Brane
    make_target stop-brn



### INSTANCE ON KUBERNETES ###
# Generates the k8s config file(s)
elif [[ "$target" == "k8s-config" ]]; then
    # Check if the storage_class_name is defined
    if [[ -z "$storage_class_name" ]]; then
        echo "k8s-config requires '--storage-class-name' to be defined"
        echo "(see --help)"
        exit 1
    fi

    # Make the script executable, then run it
    exec_step mkdir -p ./target/kube
    exec_step chmod +x ./contrib/scripts/generate-k8s-configs.sh
    exec_step ./contrib/scripts/generate-k8s-configs.sh --registry "$registry" --cluster-domain "$cluster_domain" ./docker-compose-brn.yml ./target/kube "$storage_class_name"

    # Done
    cache_regen "./docker-compose-brn.yml"
    echo "Generated Kubernetes resources files"

# Starts the Brane services (the normal images) but now on Kubernetes
elif [[ "$target" == "start-brn-k8s" ]]; then
    # Check if kubectl exists
    kubectl version 2>&1 > /dev/null
    if [[ "$?" -ne 0 ]]; then
        echo "'kubectl' not found or not working properly"
        exit 1
    fi

    # Copy all of the kube files to /tmp to have a better interface with per-run replacement values
    exec_step mkdir -p ./target/kube-resolved
    for svc in "${BRANE_INSTANCE_SERVICES[@]}"; do
        exec_step rm -f "./target/kube-resolved/$svc.yaml"
        exec_step cp "./kube/$svc.yaml" "./target/kube-resolved/$svc.yaml"

        # Replace any reference to the cluster domain
        exec_step sed -i "s/$K8S_DOMAIN_REPLACE/$cluster_domain/" "./target/kube-resolved/$svc.yaml"

        # Replace the data volumes
        if [[ "$svc" == "aux-minio" ]]; then
            exec_step sed -i "s/$K8S_DATA_SC_REPLACE/$data_storage_name/" "./target/kube-resolved/$svc.yaml"
        fi

        # Replace the config volumes
        if [[ "$svc" == "brane-drv" || "$svc" == "brane-job" || "$svc" == "brane-plr" ]]; then
            exec_step sed -i "s/$K8S_CONFIG_SC_REPLACE/$config_storage_name/" "./target/kube-resolved/$svc.yaml"
        fi
    done

    # Deploy the registry first
    exec_step kubectl -n brane-control apply -f ./target/kube-resolved/aux-registry.yaml

    # Wait until the service is up and running
    echo "Waiting for registry to come online..."
    block_until_ready "aux-registry"

    # Get the cluster IP from kubectl
    cluster_ip=$(kubectl config view --minify -o jsonpath='{.clusters[].cluster.server}' | awk -F[/:] '{print $4}')

    # Push the images to the registry
    for image in "${BRANE_INSTANCE_IMAGES[@]}"; do
        # Tag the image with the repo location
        exec_step docker tag "$image" "$cluster_ip:50050/$image"

        # Push the image
        exec_step docker push "$cluster_ip:50050/$image"
    done

    # Deploy the rest of the services - but with some timeout, to give the registry a breather
    # brane-networkpolicy
    for svc in "${BRANE_INSTANCE_SERVICES[@]}"; do
        # Skip the registry
        if [[ "$svc" == "aux-registry" ]]; then continue; fi

        # Apply the service
        exec_step kubectl -n brane-control apply -f "./target/kube-resolved/$svc.yaml"

        # Wait until the service is online (but only if not a once service)
        if [[ ! "$svc" =~ ^once- && "$svc" != "brane-networkpolicy" ]]; then block_until_ready "$svc"; fi
    done

    # Done
    echo "Started Brane pods"

# Stops the Brane services, removing the namespace as well
elif [[ "$target" == "stop-brn-k8s" ]]; then
    # Check if kubectl exists
    kubectl version 2>&1 > /dev/null
    if [[ "$?" -ne 0 ]]; then
        echo "'kubectl' not found or not working properly"
        exit 1
    fi

    # Simply reverse the files we ran
    for svc in "${BRANE_INSTANCE_SERVICES[@]}"; do
        # Only do the registry if allowed
        if [[ "$svc" == "aux-registry" && "$keep_registry" -eq 1 ]]; then continue; fi

        # For any other, always try to delete
        exec_step kubectl -n brane-control delete --ignore-not-found=true -f "./target/kube-resolved/$svc.yaml"
    done

    # Done
    echo "Stopped Brane pods"
    echo "Don't forget to reclaim the PersistentVolumes"

# Starts the Brane services on a Kubernetes cluster
elif [[ "$target" == "start-instance-k8s" ]]; then
    # Check if kubectl exists
    kubectl version 2>&1 > /dev/null
    if [[ "$?" -ne 0 ]]; then
        echo "'kubectl' not found or not working properly"
        exit 1
    fi

    # Otherwise, check if any of the instance source files needs rebuilding
    needs_regen=0
    for crate in "${BRANE_INSTANCE_SRC[@]}"; do
        if should_regen "$crate"; then
            needs_regen=1
            break
        fi
    done

    # Check if we need to rebuild according to missing images
    docker_image=$(docker image list)
    for image in "${BRANE_INSTANCE_IMAGES[@]}"; do
        if [[ -z $(echo "$docker_image" | grep "$image") ]]; then
            needs_regen=1
            break
        fi
    done

    # If we need to rebuild according to source files or images are missing, rebuild the instance
    if [[ "$needs_regen" -eq 1 ]]; then
        make_target instance
    fi

    # Prepare the configuration
    make_target ensure-configuration

    # Start brane
    make_target start-brn-k8s

elif [[ "$target" == "stop-instance-k8s" ]]; then
    make_target stop-brn-k8s



### TESTING ###
# Makes the tests and runs them
elif [[ "$target" == "test" ]]; then
    # Simply run cargo
    exec_step cargo test

# Makes the files and runs the linter (clippy)
elif [[ "$target" == "linter" ]]; then
    # Simply run cargo
    exec_step cargo clippy -- -D warnings



### OTHER ###
# Unrecognized target
else
    echo "Unrecognized target '$target'." >&2
    exit 1

fi
