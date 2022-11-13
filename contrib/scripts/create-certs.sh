#!/bin/bash
### CREATE CERTS.sh
###   by Tim Müller
#
# A script that can generate required certificates for securely connecting two domains to each other.
# 
# It will also tell you what to do with them, which is kinda neat.
# 


### CLI ###
# Read the CLI
download_dir="/tmp"
use_cfssl=""
use_cfssljson=""
mode=""
location_id=""
hostname=""
out=""
ca_cert=""
ca_key=""

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
            if [[ "$arg" == "-d" || "$arg" == "--download-dir" ]]; then
                # PArse the value next iteration
                state="download-dir"

            elif [[ "$arg" == "-u" || "$arg" == "--use-cfssl" ]]; then
                # PArse the value next iteration
                state="use-cfssl"

            elif [[ "$arg" == "-u" || "$arg" == "--use-cfssljson" ]]; then
                # PArse the value next iteration
                state="use-cfssljson"

            elif [[ "$arg" == "-o" || "$arg" == "--out" ]]; then
                # PArse the value next iteration
                state="out"

            elif [[ "$arg" == "--ca-cert" ]]; then
                # PArse the value next iteration
                state="ca-cert"

            elif [[ "$arg" == "--ca-key" ]]; then
                # PArse the value next iteration
                state="ca-key"

            elif [[ "$arg" == "-h" || "$arg" == "--help" ]]; then
                # Show the help string
                echo ""
                echo "Usage: $0 [opts] <mode> [<location_id>[ <hostname>]|<file>]"
                echo ""
                echo "This script generates keyfiles necessary to communicate with local domain registries."
                echo ""
                echo "It supports two modes: one 'server' mode, that sets up a server certificate and key; and a 'client'"
                echo "mode that signs the client with the server key to 'authorize' it."
                echo ""
                echo "Make sure that you put the server certificate and keys on each server, and then the server key and"
                echo "the signed client keys on the clients. Here, a 'server' is 'the domain itself', and the client is"
                echo "another domain connecting to it."
                echo ""
                echo "Positionals:"
                echo "  <mode>                 The mode to run. Can be 'ca', to generate new CA certificates; 'server',"
                echo "                         to generate server certificates with the generated CA; 'client', to"
                echo "                         generate client certificates with the generated CA; or 'view', to view the"
                echo "                         properties of a generated certificate/key/CSR."
                echo "  <location_id>          The location ID of this location. Note that in the case of 'client' mode,"
                echo "                         the location must be the one of the client itself."
                echo "  <hostname>             If the mode is 'server' or 'client', this specifies the IP-address to sign"
                echo "                         the certificate for."
                echo "  <file>                 If the mode is 'view', then the file to view must be specified here."
                echo ""
                echo "Options:"
                echo "  -d,--download-dir <dir>"
                echo "                         Directory where to download the cfssl tool."
                echo "  -u,--use-cfssl <file>  If given, uses the given cfssl binary instead of the default one."
                echo "  -u,--use-cfssljson <file>"
                echo "                         If given, uses the given cfssljson binary instead of the default one."
                echo "  -o,--out <file>        The output path of the certificate(s). Note that multiple files are"
                echo "                         generated, so don't provide any extensions or anything."
                echo "     --ca-cert <file>    Path to the CA certificate that we use to generate a certificate signed by"
                echo "                         it. Only relevant in mode 'server' or 'client'."
                echo "     --ca-key <file>     Path to the CA key that we use to generate a certificate signed by it."
                echo "                         Only relevant in mode 'server' or 'client'."
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
                mode="$arg"

            elif [[ "$pos_i" -eq 1 ]]; then
                # Store the location ID - which may also be the file in another setting
                location_id="$arg"

            elif [[ "$pos_i" -eq 2 ]]; then
                # Store the hostname to set
                hostname="$arg"

            else
                echo "Unknown positional '$arg' at index $pos_i"
                errored=1
            fi

            # Increment the index
            ((pos_i=pos_i+1))
        fi

    elif [[ "$state" == "download-dir" || "$state" == "use-cfssl" || "$state" == "use-cfssljson" || "$state" == "out" || "$state" == "ca-cert" || "$state" == "ca-key" ]]; then
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
        if [[ "$state" == "download-dir" ]]; then
            download_dir="$arg"
        elif [[ "$state" == "use-cfssl" ]]; then
            use_cfssl="$arg"
        elif [[ "$state" == "use-cfssljson" ]]; then
            use_cfssljson="$arg"
        elif [[ "$state" == "out" ]]; then
            out="$arg"
        elif [[ "$state" == "ca-cert" ]]; then
            ca_cert="$arg"
        elif [[ "$state" == "ca-key" ]]; then
            ca_key="$arg"
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
if [[ -z "$mode" ]]; then
    echo "No mode given; nothing to do."
    errored=1
fi

# If an error occurred, go no further
if [[ "$errored" -ne 0 ]]; then
    exit 1
fi





### DOWNLOAD BINARIES & CONFIGS ###
# Check if we're using a given one or now
if [[ ! -z "$use_cfssl" ]]; then
    # Simply use it
    cfssl="$use_cfssl"
else

    # Check if it already exists or not
    if [[ ! -f "$download_dir/cfssl" ]]; then
        echo "Downloading cfssl to '$download_dir/cfssl'..."

        # Download it using wget
        wget -O "$download_dir/cfssl" https://github.com/cloudflare/cfssl/releases/download/v1.6.3/cfssl_1.6.3_linux_amd64
        chmod +x "$download_dir/cfssl"
    fi

    # That's the path
    cfssl="$download_dir/cfssl"
fi
echo "Using cfssl @ '$cfssl'"

# Check if we're using a given one or now
if [[ ! -z "$use_cfssljson" ]]; then
    # Simply use it
    cfssljson="$use_cfssljson"
else

    # Check if it already exists or not
    if [[ ! -f "$download_dir/cfssljson" ]]; then
        echo "Downloading cfssljson to '$download_dir/cfssljson'..."

        # Download it using wget
        wget -O "$download_dir/cfssljson" https://github.com/cloudflare/cfssl/releases/download/v1.6.3/cfssljson_1.6.3_linux_amd64
        chmod +x "$download_dir/cfssljson"
    fi

    # That's the path
    cfssljson="$download_dir/cfssljson"
fi
echo "Using cfssljson @ '$cfssljson'"

# Write the configs if necessary
ca_config_json="$download_dir/ca-config.json"
if [[ ! -f "$ca_config_json" ]]; then
    echo "Writing 'ca-config.json' config to '$ca_config_json'..."
    cat <<EOF > "$ca_config_json"
{
  "signing": {
    "default": {
      "expiry": "8760h"
    },
    "profiles": {
      "server": {
        "usages": ["signing", "key encipherment", "server auth"],
        "expiry": "8760h"
      },
      "client": {
        "usages": ["signing","key encipherment","client auth"],
        "expiry": "8760h"
      }
    }
  }
}
EOF
fi
echo "Using ca-config.json @ '$ca_config_json'"

# Mark the location of other config files
ca_csr_json="$download_dir/ca-csr.json"
echo "Using ca-csr.json @ '$ca_csr_json'"

server_csr_json="$download_dir/server-csr.json"
echo "Using server-csr.json @ '$server_csr_json'"

client_csr_json="$download_dir/client-csr.json"
echo "Using client-csr.json @ '$client_csr_json'"





### MODES ###
# Switch on the modes
if [[ "$mode" == "ca" ]]; then
    # Make sure the client ID is given
    if [[ -z "$location_id" ]]; then echo "Please specify a location_id"; exit 1; fi
    # Make sure the required flags are non-empty
    if [[ -z "$out" ]]; then out="./ca"; fi

    # Write the proper CA's CSR config
    echo "Writing '$ca_csr_json' with location ID '$location_id'"
    cat <<EOF > "$ca_csr_json"
{
  "CN": "CA for $location_id",
  "key": {
    "algo": "rsa",
    "size": 4096
  },
  "names": [
    {
      "C": "US"
    }
  ]
}
EOF

    # We generate a server CA first
    echo "Generating server CA..."
    # openssl req -newkey rsa:2048 -new -nodes -x509 -days 3650 -keyout "$key_out" -out "$cert_out" || exit $?
    "$cfssl" gencert -initca "$ca_csr_json" | "$cfssljson" -bare "$out"

elif [[ "$mode" == "server" || "$mode" == "client" ]]; then
    # Make sure the location_id & hostname are given
    if [[ -z "$location_id" ]]; then echo "Please specify a location_id"; exit 1; fi
    if [[ -z "$hostname" ]]; then echo "Please specify a hostname"; exit 1; fi

    # Make sure the required flags are non-empty
    if [[ -z "$out" ]]; then out="./$mode"; fi
    if [[ -z "$ca_cert" ]] ; then ca_cert="./ca.pem"; fi
    if [[ -z "$ca_key" ]]; then ca_key="./ca-key.pem"; fi

    # Select the proper config
    if [[ "$mode" == "server" ]]; then
        # Write it first, with the correct hostname
        echo "Writing server-csr.json to '$server_csr_json' with location ID '$location_id' and hostname '$hostname'..."
        cat <<EOF > "$server_csr_json"
{
  "CN": "$location_id",
  "hosts": ["$hostname"],
  "key": {
    "algo": "rsa",
    "size": 4096
  },
  "names": [
    {
      "C": "US"
    }
  ]
}
EOF

        # Select the filename
        csr_json="$server_csr_json"
    elif [[ "$mode" == "client" ]]; then
        # Write it first, with the correct hostname
        echo "Writing client-csr.json to '$client_csr_json' with location ID '$location_id' and hostname '$hostname'..."
        cat <<EOF > "$client_csr_json"
{
  "CN": "$location_id",
  "hosts": ["$hostname"],
  "key": {
    "algo": "rsa",
    "size": 4096
  },
  "names": [
    {
      "C": "US"
    }
  ]
}
EOF

        # Select the filename
        csr_json="$client_csr_json"
    fi

    # Do everything in one go _again_ (epic, what a tool)
    echo "Generating signed $mode certificate..."
    "$cfssl" gencert -ca="$ca_cert" -ca-key="$ca_key" -config="$ca_config_json" -profile="$mode" "$csr_json" | "$cfssljson" -bare "$out"

    # Also create an ID file
    cp "$out-key.pem" "$out-id.pem"
    cat "$out.pem" >> "$out-id.pem"

elif [[ "$mode" == "view" ]]; then
    # Make sure the file is non-empty
    if [[ -z "$location_id" ]]; then
        echo "No file to view given; nothing to do."
        exit 1
    fi

    # Run the view command
    openssl x509 -noout -text -in "$location_id"

else
    echo "Unknown mode '$mode'"
    exit 1
fi
