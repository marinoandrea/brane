#!/bin/bash
res=$(nvidia-smi)
if [[ "$?" -ne 0 ]]; then exit 1; fi
echo "output: |"
echo "  $res" | sed -z 's/\n/\n  /g'
