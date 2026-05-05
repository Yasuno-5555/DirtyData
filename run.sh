#!/bin/bash
# DirtyRack Launcher
# Usage: ./run.sh [args]

cargo run --release -p dirtyrack-gui -- "$@"
