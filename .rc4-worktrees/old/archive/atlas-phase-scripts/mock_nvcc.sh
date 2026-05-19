#!/bin/bash
# Mock nvcc script
echo "Mocking nvcc $@"

OUTFILE=""
for i in "$@"; do
    if [ "$OUTFILE" == "NEXT" ]; then
        OUTFILE="$i"
        break
    fi
    if [ "$i" == "-o" ]; then
        OUTFILE="NEXT"
    fi
done

if [ -n "$OUTFILE" ] && [ "$OUTFILE" != "NEXT" ]; then
    echo "Creating mock output file: $OUTFILE"
    mkdir -p "$(dirname "$OUTFILE")"
    touch "$OUTFILE"
fi
exit 0
