# install with `ln -s ../../pre-push.sh .git/hooks/pre-push`

echo "Running pre-push hook"
cargo clippy
cargo fmt --check
