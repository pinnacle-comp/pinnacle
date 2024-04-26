#!/bin/sh

WLCS_SHA=26c5a8cfef265b4ae021adebfec90d758c08792e

if [ -f "./wlcs/wlcs" ] && [ "$(cd wlcs; git rev-parse HEAD)" = "${WLCS_SHA}" ] ; then
    echo "WLCS commit 26c5a8c is already compiled"
else
    echo "Compiling WLCS"
    git clone https://github.com/canonical/wlcs
    cd wlcs || exit
    # checkout a specific revision
    git reset --hard "${WLCS_SHA}"
    cmake -DWLCS_BUILD_ASAN=False -DWLCS_BUILD_TSAN=False -DWLCS_BUILD_UBSAN=False -DCMAKE_EXPORT_COMPILE_COMMANDS=1 .
    make
fi
