# START CENTRAL INSTANCE.sh
#   by Tim Müller
# 
# A slimmed-down version of the `make.py` script that contains commands to
# only run existing images (not install or compile anything).
# 


### DEFAULTS ###
# TODO: Fix
FILE="./docker-compose-worker.yml"
SCYLLA_IMAGE="./target/debug/brane-"
API_IMAGE="./target/debug/brane-api.tar"
DRV_IMAGE="./target/debug/brane-drv.tar"
PLR_IMAGE="./target/debug/brane-plr.tar"
CNT_PORT=50050
API_PORT=50051
DRV_PORT=50053
CONFIG="./config"
CERTS="$CONFIG/certs"





### CLI ###
# Read the CLI
cmd=""
file="$FILE"
scylla_image="$SCYLLA_IMAGE"
api_image="$API_IMAGE"
drv_image="$DRV_IMAGE"
plr_image="$PLR_IMAGE"
cnt_port="$CNT_PORT"
api_port="$API_PORT"
drv_port="$DRV_PORT"
config="$CONFIG"
certs=""
load=0
verbose=0

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
            if [[ "$arg" == '-f' || "$arg" == "--file" ]]; then
                # Wait for the next argument to parse the path
                state="file"

            elif [[ "$arg" == "-j" || "$arg" == "--job-image" ]]; then
                # Wait for the next argument to parse the path
                state="job-image"

            elif [[ "$arg" == "-r" || "$arg" == "--reg-image" ]]; then
                # Wait for the next argument to parse the path
                state="reg-image"

            elif [[ "$arg" == "-j" || "$arg" == "--job-port" ]]; then
                # Wait for the next argument to parse the path
                state="job-port"

            elif [[ "$arg" == "-r" || "$arg" == "--reg-port" ]]; then
                # Wait for the next argument to parse the path
                state="reg-port"

            elif [[ "$arg" == "-c" || "$arg" == "--config" ]]; then
                # Wait for the next argument to parse the path
                state="config"

            elif [[ "$arg" == "-D" || "$arg" == "--data" ]]; then
                # Wait for the next argument to parse the path
                state="data"

            elif [[ "$arg" == "-R" || "$arg" == "--results" ]]; then
                # Wait for the next argument to parse the path
                state="results"

            elif [[ "$arg" == "-C" || "$arg" == "--certs" ]]; then
                # Wait for the next argument to parse the path
                state="certs"

            elif [[ "$arg" == "-l" || "$arg" == "--load" ]]; then
                # Mark the start as being loaded
                load=1

            elif [[ "$arg" == "-v" || "$arg" == "--verbose" ]]; then
                # Mark that the script should be verbose
                verbose=1

            elif [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
                # Show the help string
                echo ""
                echo "Usage: $0 [opts] <command>[ <location_id>]"
                echo ""
                echo "This script starts or stops a worker instance on the current node. Note that, to do so, you should"
                echo "have Docker with the Buildx plugin installed."
                echo ""
                echo "It requires that there are already images available in '.tar' format. Check 'make.py' (in the Brane"
                echo "repository) to compile them, and check https://wiki.enablingpersonalizedinterventions.nl to learn"
                echo "how."
                echo ""
                echo "Positionals:"
                echo "  <command>              The command to run. Can be 'start', to start the instance, or 'stop', to"
                echo "                         stop the instance (wow)."
                echo "  <location_id>          If the command is 'start', then this *must* be given to describe this"
                echo "                         domain's location identifier. Must match its ID in the central node's"
                echo "                         'infra.yml' file."
                echo ""
                echo "Options:"
                echo "  -f,--file <PATH>       Path to the Docker compose file for the worker instance to run."
                echo "                         Default: '$FILE'"
                echo "  -j,--job-image <PATH>  Path to the image of the job service."
                echo "                         Default: '$JOB_IMAGE'"
                echo "  -r,--reg-image <PATH>  Path to the image of the registry service."
                echo "                         Default: '$REG_IMAGE'"
                echo "     --job-port <PORT>   The port on which the job service will be hosted."
                echo "                         Default: '$JOB_PORT'"
                echo "     --reg-port <PORT>   The port on which the registry service will be hosted."
                echo "                         Default: '$REG_PORT'"
                echo "  -c,--config <PATH>     Path to the configuration folder to use for the services. Contains stuff"
                echo "                         like the backend credentials, certificates and datasets. See the wiki for"
                echo "                         more information."
                echo "                         Default: '$CONFIG'"
                echo "  -D,--data <PATH>       Path to the folder where we read datasets from and commit new datasets to."
                echo "                         Check the documentation to find out how creating new datasets yourself"
                echo "                         works."
                echo "                         Default: '$DATA'"
                echo "  -R,--results <PATH>    Path to the folder where we store intermediate results."
                echo "                         Default: '$RESULTS'"
                echo "  -C,--certs <PATH>      Path to the certificate folder that is used by *both* 'brane-job' and"
                echo "                         'brane-reg'. Typically, it should be a folder containing a root"
                echo "                         certificate and key ('ca.pem' and 'ca-key.pem') and a server certificate"
                echo "                         and key ('server.pem' and 'server-key.pem') signed by that root"
                echo "                         certificate. Then, for every other domain that we may download from, there"
                echo "                         should be a folder with that domain's ID as name, with their nested root"
                echo "                         certificate ('ca.pem') and identity file signed by that root certificate"
                echo "                         ('client-id.pem')."
                echo "                         Default: '<CONFIG>/certs'"
                echo "  -l,--load              Loads the images before 'start'ing them. You should do this at least once,"
                echo "                         and then every time you have new image files."
                echo "  -v,--verbose           If given, prints some additional debug things that may be useful to check."
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
                # Store the command; it's validity will be checked when switching on it
                cmd="$arg"

            elif [[ "$pos_i" -eq 1 ]]; then
                # Store the location ID - which may also be the file in another setting
                location_id="$arg"

            else
                echo "Unknown positional '$arg' at index $pos_i"
                errored=1
            fi

            # Increment the index
            ((pos_i=pos_i+1))
        fi

    elif [[ "$state" == "file" || "$state" == "job-image" || "$state" == "reg-image" || "$state" == "job-port" || "$state" == "reg-port" || "$state" == "config" || "$state" == "data" || "$state" == "results" || "$state" == "certs" ]]; then
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
        if [[ "$state" == "file" ]]; then
            file="$arg"
        elif [[ "$state" == "job-image" ]]; then
            job_image="$arg"
        elif [[ "$state" == "reg-image" ]]; then
            reg_image="$arg"
        elif [[ "$state" == "job-port" ]]; then
            # Make sure it's numerical
            if [[ ! ("$arg" =~ ^[0-9]+$) ]]; then echo "Job port has to be a non-negative number"; errored=1; fi
            job_port="$arg"
        elif [[ "$state" == "reg-port" ]]; then
            if [[ ! ("$arg" =~ ^[0-9]+$) ]]; then echo "Registry port has to be a non-negative number"; errored=1; fi
            reg_port="$arg"
        elif [[ "$state" == "config" ]]; then
            config="$arg"
        elif [[ "$state" == "data" ]]; then
            # Make sure it's absolute
            if [[ ! ("$arg" =~ ^/.*$) ]]; then arg="$(pwd)/$arg"; fi
            data="$arg"
        elif [[ "$state" == "results" ]]; then
            if [[ ! ("$arg" =~ ^/.*$) ]]; then arg="$(pwd)/$arg"; fi
            results="$arg"
        elif [[ "$state" == "certs" ]]; then
            certs="$arg"
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
if [[ -z "$cmd" ]]; then
    echo "No command given; nothing to do."
    errored=1
fi
if [[ "$cmd" == "start" && -z "$location_id" ]]; then
    echo "Missing location ID."
    errored=1
fi
if [[ -z "$certs" ]]; then
    certs="$config/certs"
fi

# If an error occurred, go no further
if [[ "$errored" -ne 0 ]]; then
    exit 1
fi

