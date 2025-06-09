use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dicom_anonymization::processor::NoopProcessor;
use dicom_anonymization::{
    Anonymizer, config::builder::ConfigBuilder, config::uid_root::UidRoot,
    processor::DefaultProcessor,
};
use std::fs::File;
use std::io::Read;

fn load_test_dicom() -> Vec<u8> {
    let mut file = File::open("tests/data/test.dcm").expect("Failed to open test DICOM file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Failed to read test DICOM file");
    buffer
}

/// Benchmark anonymization performance using the default configuration.
///
/// This benchmark measures the time to anonymize a single DICOM file using
/// the default anonymization settings, which include standard tag actions
/// and privacy policies for removing private tags, curves, and overlays.
fn benchmark_anonymization_default(c: &mut Criterion) {
    let test_data = load_test_dicom();
    let anonymizer = Anonymizer::default();

    c.bench_function("anonymize_default_config", |b| {
        b.iter(|| {
            anonymizer
                .anonymize(black_box(test_data.as_slice()))
                .expect("Anonymization failed")
        })
    });
}

/// Compare anonymization performance across different configuration options.
///
/// This benchmark group tests various anonymization configurations to understand
/// the performance impact of different settings:
/// - Default: Standard anonymization with all policies enabled
/// - Private tags only: Only removes private tags, keeps standard data
/// - Minimal: Minimal anonymization, keeps most data intact
/// - NoopProcessor: Baseline measurement with no actual anonymization
fn benchmark_anonymization_custom_configs(c: &mut Criterion) {
    let test_data = load_test_dicom();

    let mut group = c.benchmark_group("anonymization_configs");
    group.significance_level(0.1).sample_size(100);

    // Default configuration - standard anonymization with all policies enabled
    let default_config = ConfigBuilder::default().build();
    let default_processor = DefaultProcessor::new(default_config);
    let default_anonymizer = Anonymizer::new(default_processor);

    group.bench_function("default", |b| {
        b.iter(|| {
            default_anonymizer
                .anonymize(black_box(test_data.as_slice()))
                .expect("Anonymization failed")
        })
    });

    // Private tags only - removes only private tags, preserves standard DICOM data
    let private_only_config = ConfigBuilder::new()
        .remove_private_tags(true)
        .remove_curves(false)
        .remove_overlays(false)
        .build();
    let private_only_processor = DefaultProcessor::new(private_only_config);
    let private_only_anonymizer = Anonymizer::new(private_only_processor);

    group.bench_function("private_tags_only", |b| {
        b.iter(|| {
            private_only_anonymizer
                .anonymize(black_box(test_data.as_slice()))
                .expect("Anonymization failed")
        })
    });

    // Minimal anonymization - preserves maximum data, minimal processing overhead
    let minimal_config = ConfigBuilder::new()
        .remove_private_tags(false)
        .remove_curves(false)
        .remove_overlays(false)
        .build();
    let minimal_processor = DefaultProcessor::new(minimal_config);
    let minimal_anonymizer = Anonymizer::new(minimal_processor);

    group.bench_function("minimal", |b| {
        b.iter(|| {
            minimal_anonymizer
                .anonymize(black_box(test_data.as_slice()))
                .expect("Anonymization failed")
        })
    });

    // No-op baseline - measures framework overhead without any actual anonymization
    let noop_processor = NoopProcessor::new();
    let noop_anonymizer = Anonymizer::new(noop_processor);

    group.bench_function("noop", |b| {
        b.iter(|| {
            noop_anonymizer
                .anonymize(black_box(test_data.as_slice()))
                .expect("Anonymization failed")
        })
    });

    group.finish();
}

/// Measure anonymization throughput in bytes per second.
///
/// This benchmark measures the data processing rate for DICOM anonymization,
/// providing insights into how much data can be processed per unit time.
/// Results are reported in GiB/s to understand real-world performance
/// characteristics for large datasets.
fn benchmark_anonymization_throughput(c: &mut Criterion) {
    let test_data = load_test_dicom();
    let anonymizer = Anonymizer::default();

    let mut group = c.benchmark_group("anonymization_throughput");
    group.throughput(Throughput::Bytes(test_data.len() as u64));

    group.bench_function("throughput", |b| {
        b.iter(|| {
            anonymizer
                .anonymize(black_box(test_data.as_slice()))
                .expect("Anonymization failed")
        })
    });

    group.finish();
}

/// Test anonymization performance scalability with multiple files.
///
/// This benchmark evaluates how anonymization performance scales when processing
/// multiple DICOM files sequentially. Tests with 1, 10, 50, and 100 files to
/// identify any performance degradation or overhead that emerges at scale.
/// Results help understand batch processing characteristics.
fn benchmark_anonymization_scalability(c: &mut Criterion) {
    let test_data = load_test_dicom();
    let anonymizer = Anonymizer::default();

    let mut group = c.benchmark_group("anonymization_scalability");

    for &size in &[1, 10, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                for _ in 0..size {
                    anonymizer
                        .anonymize(black_box(test_data.as_slice()))
                        .expect("Anonymization failed");
                }
            })
        });
    }

    group.finish();
}

/// Benchmark configuration building performance.
///
/// This benchmark measures the overhead of creating anonymization configurations
/// using the ConfigBuilder pattern. Tests both default and complex configurations
/// to understand the cost of configuration setup, which may be important for
/// applications that create many configurations dynamically.
fn benchmark_config_builder(c: &mut Criterion) {
    c.bench_function("config_builder_default", |b| {
        b.iter(|| black_box(ConfigBuilder::default().build()))
    });

    c.bench_function("config_builder_complex", |b| {
        b.iter(|| {
            black_box(
                ConfigBuilder::default()
                    .remove_private_tags(true)
                    .remove_curves(true)
                    .remove_overlays(true)
                    .uid_root(UidRoot::new("1.2.3.4").unwrap())
                    .build(),
            )
        })
    });
}

/// Benchmark processor creation performance.
///
/// This benchmark measures the cost of creating a DefaultProcessor instance
/// from a configuration. This overhead may be significant for applications
/// that create processors frequently, though most applications would typically
/// create processors once and reuse them.
fn benchmark_processor_creation(c: &mut Criterion) {
    let config = ConfigBuilder::default().build();

    c.bench_function("processor_creation", |b| {
        b.iter(|| black_box(DefaultProcessor::new(config.clone())))
    });
}

criterion_group!(
    benches,
    benchmark_anonymization_default,
    benchmark_anonymization_custom_configs,
    benchmark_anonymization_throughput,
    benchmark_anonymization_scalability,
    benchmark_config_builder,
    benchmark_processor_creation
);
criterion_main!(benches);
