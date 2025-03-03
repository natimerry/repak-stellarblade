use std::sync::LazyLock;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use simd_str_cmp::{compare_string_vectors, compare_string_vectors_naive};

// Generate test data for 16-byte strings.
static TEST_DATA_16: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let size = 1000;
    let string_length = 16;
    let s = "a".repeat(string_length);
    let haystack1 = vec![s.clone(); size];
    let haystack2 = vec![s; size];
    (haystack1, haystack2)
});

// Generate test data for 32-byte strings.
static TEST_DATA_32: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let size = 1000;
    let string_length = 32;
    let s = "a".repeat(string_length);
    let haystack1 = vec![s.clone(); size];
    let haystack2 = vec![s; size];
    (haystack1, haystack2)
});

// Generate test data for 64-byte strings.
static TEST_DATA_64: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let size = 1000;
    let string_length = 64;
    let s = "a".repeat(string_length);
    let haystack1 = vec![s.clone(); size];
    let haystack2 = vec![s; size];
    (haystack1, haystack2)
});

// Generate test data for 128-byte strings.
static TEST_DATA_128: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let size = 1000;
    let string_length = 128;
    let s = "a".repeat(string_length);
    let haystack1 = vec![s.clone(); size];
    let haystack2 = vec![s; size];
    (haystack1, haystack2)
});

// Generate test data for 256-byte strings.
static TEST_DATA_256: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let size = 1000;
    let string_length = 256;
    let s = "a".repeat(string_length);
    let haystack1 = vec![s.clone(); size];
    let haystack2 = vec![s; size];
    (haystack1, haystack2)
});


static TEST_DATA_VERY_BIG: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let size = 1000;
    let string_length = 1024;
    let s = "a".repeat(string_length);
    let haystack1 = vec![s.clone(); size];
    let haystack2 = vec![s; size];
    (haystack1, haystack2)
});

static TEST_DATA_VERY_MASSIVE: LazyLock<(Vec<String>, Vec<String>)> = LazyLock::new(|| {
    let size = 1000;
    let string_length = 4096; // chosen at random
    let s = "a".repeat(string_length);
    let haystack1 = vec![s.clone(); size];
    let haystack2 = vec![s; size];
    (haystack1, haystack2)
});


pub fn criterion_benchmark_simd_vs_native(c: &mut Criterion) {
    // 16-byte strings benchmark.
    let (data16_1, data16_2) = TEST_DATA_16.clone();
    c.bench_function("SIMD + Rayon (16-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors(
                black_box(data16_1.clone()),
                black_box(data16_2.clone()),
            );
        })
    });
    c.bench_function("Native (16-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors_naive(
                black_box(data16_1.clone()),
                black_box(data16_2.clone()),
            );
        })
    });

    // 32-byte strings benchmark.
    let (data32_1, data32_2) = TEST_DATA_32.clone();
    c.bench_function("SIMD + Rayon (32-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors(
                black_box(data32_1.clone()),
                black_box(data32_2.clone()),
            );
        })
    });
    c.bench_function("Native (32-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors_naive(
                black_box(data32_1.clone()),
                black_box(data32_2.clone()),
            );
        })
    });

    // 64-byte strings benchmark.
    let (data64_1, data64_2) = TEST_DATA_64.clone();
    c.bench_function("SIMD + Rayon (64-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors(
                black_box(data64_1.clone()),
                black_box(data64_2.clone()),
            );
        })
    });
    c.bench_function("Native (64-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors_naive(
                black_box(data64_1.clone()),
                black_box(data64_2.clone()),
            );
        })
    });

    // 128-byte strings benchmark.
    let (data128_1, data128_2) = TEST_DATA_128.clone();
    c.bench_function("SIMD + Rayon (128-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors(
                black_box(data128_1.clone()),
                black_box(data128_2.clone()),
            );
        })
    });
    c.bench_function("Native (128-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors_naive(
                black_box(data128_1.clone()),
                black_box(data128_2.clone()),
            );
        })
    });

    // 256-byte strings benchmark.
    let (data256_1, data256_2) = TEST_DATA_256.clone();
    c.bench_function("SIMD + Rayon (256-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors(
                black_box(data256_1.clone()),
                black_box(data256_2.clone()),
            );
        })
    });
    c.bench_function("Native (256-bit)", |b| {
        b.iter(|| {
            let _ = compare_string_vectors_naive(
                black_box(data256_1.clone()),
                black_box(data256_2.clone()),
            );
        })
    });


    let (haystack1, haystack2) = TEST_DATA_VERY_BIG.clone();

    c.bench_function("SIMD + RAYON VERY BIG", |b| {
        b.iter(|| {
            let _ = compare_string_vectors(black_box(haystack1.clone()), black_box(haystack2.clone()));
        })
    });

    c.bench_function("Native", |b| {
        b.iter(|| {
            let _ = compare_string_vectors_naive(black_box(haystack1.clone()), black_box(haystack2.clone()));
        })
    });


    let (haystack1, haystack2) = TEST_DATA_VERY_MASSIVE.clone();

    c.bench_function("SIMD+ RAYON MASSIVE", |b| {
        b.iter(|| {
            let _ = compare_string_vectors(black_box(haystack1.clone()), black_box(haystack2.clone()));
        })
    });

    c.bench_function("Native", |b| {
        b.iter(|| {
            let _ = compare_string_vectors_naive(black_box(haystack1.clone()), black_box(haystack2.clone()));
        })
    });

}


criterion_group!(benches, criterion_benchmark_simd_vs_native);
criterion_main!(benches);