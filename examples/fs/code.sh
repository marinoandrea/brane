#!/bin/bash
# FS.sh
#   By Tim Müller
#
# Implements a 'filesystem' for Brane, which is just a set of commands that
# allow one to inspect, read, write, list or remove files in the shared /data
# partition.
#


##### FUNCTIONS #####
# Shows the contents of a directory - recursively.
tree_dir() {
    # Print the contents of the file IFF it is a file
    list=""
    if [[ -f "$1" ]]; then
        list="$list
  f $1"

    elif [[ -d "$1" ]]; then
        list="$list
  d $1"
        for d in "$1"/*; do
            list="$list$(tree_dir "$d")"
        done

    else
        list="$list
  ? $1"

    fi

    # Return the things
    echo "$list"
}





##### CLI #####
# Read the command used
if [[ "$#" -ne 1 ]]; then
    echo "Usage: $0 <command>"
    echo ""
    echo "Use '$0 --help' to see a list of available commands."
    exit 1
fi
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  ls            Lists all files & directories in a folder. The list is returned as a JSON for nice"
    echo "                printing."
    echo "  lsdata        Lists all files & directories in a folder, while taking a dummy dataset to explore"
    echo "                its path. The list is returned as JSON for nice printing."
    echo "  treedata      Lists all files & directories (recursively) in a folder, while taking a dummy"
    echo "                dataset to explore its contents."
    echo "  read          Prints the contents of the given file as a regular string."
    echo "  read64        Prints the raw contents of the given file as base64."
    echo "  write         Writes the given string to the given file, overwriting what was already there. The"
    echo "                string should be given in the 'CONTENTS' environment variable."
    echo "  write64       Writes the given base64-encoded data to the given file, overwriting what was already"
    echo "                there. The encoded contents should be given in the 'CONTENTS' environment variable."
    echo "  append        Writes the given string to the end of the given file, keeping what was already"
    echo "                there. The encoded contents should be given in the 'CONTENTS' environment variable."
    echo "  append64      Write sthe given base64-encoded data at the end of the given file, keeping what was"
    echo "                already there. The encoded contents should be given in the 'CONTENTS' environment"
    echo "                variable."
    echo "  rm            Deletes the given file, but will refuse to delete a given directory."
    echo "  rm_dir        Deletes the given file or directory."
    echo ""
    echo "Note: all file names should be given in the 'TARGET' environment variable, and all outputs are written"
    echo "to the 'output' YAML variable."
    echo ""
fi
cmd="$1"





##### PREPROCESS #####
path=$(echo "$TARGET" | python3 -c "import json, sys; print(json.load(sys.stdin))")





##### COMMANDS #####
# Switch on the parsed command
if [[ "$cmd" == "ls" ]]; then
    # Print the contents of the file IFF it is a file
    if [[ -f "$path" ]]; then
        echo "output: \"f $path\""
    elif [[ -d "$path" ]]; then
        echo "output: |"
        for d in "$path"/*; do
            if [[ -f "$d" ]]; then
                echo "  f $d"
            elif [[ -d "$d" ]]; then
                echo "  d $d"
            else
                echo "  ? $d"
            fi
        done
    else
        echo "output: \"Given path '$path' is not a file or directory\""
        exit 0
    fi

# Switch on the parsed command
elif [[ "$cmd" == "lsdata" ]]; then
    # Print the contents of the file IFF it is a file
    if [[ -f "$path" ]]; then
        echo "output: \"f $path\""
    elif [[ -d "$path" ]]; then
        echo "output: |"
        for d in "$path"/*; do
            if [[ -f "$d" ]]; then
                echo "  f $d"
            elif [[ -d "$d" ]]; then
                echo "  d $d"
            else
                echo "  ? $d"
            fi
        done
    else
        echo "output: \"Given path '$path' is not a file or directory\""
        exit 0
    fi

elif [[ "$cmd" == "treedata" ]]; then
    # Print the contents of the file IFF it is a file
    echo "output: |$(tree_dir "$path")"

elif [[ "$cmd" == "read" ]]; then
    # Simply attempt to read
    if [[ -f "$path" ]]; then
        echo "output: |"
        out=$(cat "$path" | sed -z 's/\n/\n  /g;s/\n  $/\n/')
        echo "  $out"
    elif [[ -d "$path" ]]; then
        echo "output: \"Cannot read file '$path': is a directory\""
        exit 0
    else
        echo "output: \"Cannot read file '$path': not found\""
        exit 0
    fi

elif [[ "$cmd" == "read64" ]]; then
    # Simply attempt to read (but encode as Base64 first)
    if [[ -f "$path" ]]; then
        echo "output: \"$(base64 -e "$path")\""
    elif [[ -d "$path" ]]; then
        echo "output: \"Cannot read file '$path': is a directory\""
        exit 0
    else
        echo "output: \"Cannot read file '$path': not found\""
        exit 0
    fi

elif [[ "$cmd" == "write" ]]; then
    # Simply write the contents
    echo "$CONTENTS" > "$path"
    echo "output: \"\""

elif [[ "$cmd" == "write64" ]]; then
    # Simply write the contents, but base64 encoded
    echo "$(base64 -e "$CONTENTS")" > "$path"
    echo "output: \"\""

elif [[ "$cmd" == "append" ]]; then
    # Check if the file is a flie
    if [[ -f "$path" ]]; then
        # Write the contents
        echo "$CONTENTS" >> "$path"
        echo "output: \"\""
    elif [[ -d "$path" ]]; then
        echo "output: \"Could not append to file '$path': is a directory\""
        exit 0
    else
        echo "output: \"Could not append to file '$path': not found\""
        exit 0
    fi

elif [[ "$cmd" == "append64" ]]; then
    # Check if the file is a flie
    if [[ -f "$path" ]]; then
        # Write the contents
        echo "$(base64 -e "$CONTENTS")" >> "$path"
        echo "output: \"\""
    elif [[ -d "$path" ]]; then
        echo "output: \"Could not append to file '$path': is a directory\""
        exit 0
    else
        echo "output: \"Could not append to file '$path': not found\""
        exit 0
    fi

elif [[ "$cmd" == "rm" ]]; then
    # Remove only if file
    if [[ -f "$path" ]]; then
        rm -f "$path"
        echo "output: \"\""
    elif [[ -d "$path" ]]; then
        echo "output: \"Cannot remove file '$path': is a directory\""
    else
        echo "output: \"Cannot remove file '$path': not found\""
    fi

elif [[ "$cmd" == "rm_dir" ]]; then
    # Remove the file or directory
    if [[ -f "$path" || -d "$path" ]]; then
        rm -rf "$path"
        echo "output: \"\""
    else
        echo "output: \"Cannot remove '$path': not found\""
    fi

else
    echo "Unknown command '$cmd'"
    exit 1
fi
