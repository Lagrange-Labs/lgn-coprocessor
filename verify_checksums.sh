#!/bin/bash
dir=$1
expected_checksums_file=$2

# Calculate checksums of files in the directory
find "$dir" -type f -exec sha256sum {} + > /tmp/calculated_checksums.txt

# Compare with expected checksums
if ! diff -q /tmp/calculated_checksums.txt "$expected_checksums_file"; then
  echo "Checksum mismatch found"
  exit 1
else
  echo "All checksums match"
fi
