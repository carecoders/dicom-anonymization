use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dicom_anonymization::processor::NoopProcessor;
use dicom_anonymization::{
    config::builder::ConfigBuilder, config::uid_root::UidRoot, processor::DefaultProcessor,
    Anonymizer,
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

fn benchmark_anonymization_custom_configs(c: &mut Criterion) {
    let test_data = load_test_dicom();

    let mut group = c.benchmark_group("anonymization_configs");

    // Default configuration
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

    // Remove private tags only, do nothing else
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

    // Minimal anonymization
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

    // Noop anonymization
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
