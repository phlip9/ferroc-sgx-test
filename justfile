miri:
    MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" \
        cargo run miri
