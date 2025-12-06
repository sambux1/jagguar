# check that sage exists on this system
if ! command -v sage >/dev/null 2>&1; then
    echo "Warning: 'sage' not found in PATH. Install SageMath to use the lattice estimator." >&2
    exit 1
fi

git clone git@github.com:malb/lattice-estimator.git
