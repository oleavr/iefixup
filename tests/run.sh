#!/bin/sh
set -e
cd "$(dirname "$0")"
mkdir -p ../build/tests
cc -O1 -g -fsanitize=address,undefined -Wall -Ishim -I../src -o ../build/tests/rewrite rewrite.c ../src/html.c \
  ../src/utf8.c ../src/buffer.c
for page in "$@"; do
  ../build/tests/rewrite "$page" 1 > ../build/tests/expected
  for seed in 2 3 4 5 6 7 8 9; do
    ../build/tests/rewrite "$page" $seed | cmp -s - ../build/tests/expected || { echo "$page: seed $seed differs"; exit 1; }
  done
done
echo ok
