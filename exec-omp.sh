#!/bin/bash
set -e

devcontainer up
devcontainer exec ./copy-models-yml.sh
devcontainer exec omp --model deepseek/deepseek-v4-flash
