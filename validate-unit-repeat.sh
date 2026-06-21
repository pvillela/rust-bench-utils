#!/bin/bash

export RUSTFLAGS="-Awarnings"

#!/bin/bash

# Check if exactly two arguments are provided
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <number_of_repeats> <fully_qualified_test_name>"
    exit 1
fi

# Assign arguments to variables
REPEATS=$1
TEST_NAME=$2

# Validate that the first argument is a positive integer
if ! [[ "$REPEATS" =~ ^[0-9]+$ ]] || [ "$REPEATS" -le 0 ]; then
    echo "Error: First argument must be a positive integer."
    exit 1
fi

echo "Running '$TEST_NAME' $REPEATS times..."

# Loop and execute the cargo test command
for ((i=1; i<=REPEATS; i++)); do
    echo "----------------------------------------"
    echo "Execution $i of $REPEATS"
    echo "----------------------------------------"
    
    # --exact ensures only the specific test runs, not matches
    # -- --nocapture optional flag can be added to see stdout
    cargo test -q --message-format short -r --lib --features _ALL_NON_TEST,_bench -- "$TEST_NAME" --test-threads=1 --exact
    # Check if the test failed
    if [ $? -ne 0 ]; then
        echo "Test failed on execution $i!"
        exit 1
    fi
done

echo "All $REPEATS executions passed successfully!"
