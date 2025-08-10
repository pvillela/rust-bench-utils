# Command line arguments:
#
# $1  # NREPEATS

export LATENCY_UNIT="micro" 
export BASE_MEDIAN="100"

./bench-criterion-comp.sh 1.00 $1
./bench-criterion-comp.sh 1.01 $1
./bench-criterion-comp.sh 1.02 $1
./bench-criterion-comp.sh 1.05 $1
./bench-criterion-comp.sh 1.10 $1
./bench-criterion-comp.sh 1.25 $1
