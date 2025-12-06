# check that there is a command line argument
if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <sage_script>" >&2
    exit 1
fi

PYTHONPATH="./lattice-estimator" sage "$1"