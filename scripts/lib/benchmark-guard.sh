guard_benchmark_entrypoint() {
    if [ -n "${KFIND_BENCHMARK_SESSION:-}" ]; then
        return 0
    fi

    benchmark_guard_root=$1
    benchmark_guard_name=$2
    shift 2
    exec "$benchmark_guard_root/scripts/benchmark-run.sh" run \
        --name "$benchmark_guard_name" \
        -- "$@"
}
