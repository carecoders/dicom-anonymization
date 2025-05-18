use anyhow::{bail, Context, Result};
use clap::builder::TypedValueParser;
use clap::Parser;
use dicom_anonymization::actions::Action;
use dicom_anonymization::config::uid_root::UidRoot;
use dicom_anonymization::processor::DefaultProcessor;
use dicom_anonymization::tags;
use dicom_anonymization::Anonymizer;
use dicom_anonymization::Tag;
use dicom_anonymization::{config::builder::ConfigBuilder, config::Config, AnonymizationError};
use dicom_object::DefaultDicomObject;
use env_logger::Builder;
use log::{info, warn, Level, LevelFilter};
use rayon::prelude::*;
use std::fmt;
use std::time::Instant;
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
};
use walkdir::WalkDir;

#[derive(Clone)]
struct TagValueParser;

impl TypedValueParser for TagValueParser {
    type Value = Tag;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let s = value.to_str().ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidUtf8,
                "invalid exclude tag(s)",
            )
        })?;

        Tag::from_str(s).map_err(|_e| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                format!("{s} is not a valid tag"),
            )
        })
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Show more verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Show debug output
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum ConfigCommands {
    /// Create a configuration file
    Create(ConfigCreateArgs),
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Anonymize DICOM files
    Anonymize(AnonymizeArgs),

    /// Configuration tools
    #[command(subcommand)]
    Config(ConfigCommands),
}

/// Anonymize DICOM files
#[derive(Parser, Debug)]
struct AnonymizeArgs {
    /// Input file ('-' for stdin) or directory
    #[arg(short, long, value_name = "INPUT_PATH")]
    input: PathBuf,

    /// Output file ('-' for stdout) or directory
    #[arg(short, long, value_name = "OUTPUT_PATH")]
    output: PathBuf,

    /// Path to config JSON file
    #[arg(short = 'c', long = "config", value_name = "CONFIG_FILE")]
    config_file: Option<PathBuf>,

    /// UID root (default '9999')
    #[arg(short, long)]
    uid_root: Option<String>,

    /// Tags to exclude from anonymization, e.g. '00100020,00080050'
    #[arg(long, value_name = "TAGS", value_delimiter = ',', value_parser = TagValueParser)]
    exclude: Vec<Tag>,

    /// Recursively look for files in input directory
    #[arg(short, long)]
    recursive: bool,

    /// Continue when file found is not DICOM
    #[arg(long = "continue")]
    r#continue: bool,
}

#[derive(Parser, Debug)]
struct ConfigCreateArgs {
    /// Path to save the config file  (‘-’ or omitted → stdout)
    #[arg(short, long, value_name = "CONFIG_FILE", default_value = "-")]
    output: PathBuf,

    /// UID root to use
    #[arg(short, long)]
    uid_root: Option<String>,

    /// Tags to exclude from anonymization, e.g. '00100020,00080050'
    #[arg(long, value_name = "TAGS", value_delimiter = ',', value_parser = TagValueParser)]
    exclude: Vec<Tag>,

    /// Only output the dfferences with the default config
    #[arg(long, default_value = "false")]
    diff_only: bool,
}

struct DicomOutputFilePath {
    study_instance_uid: String,
    series_instance_uid: String,
    sop_instance_uid: String,
}

impl fmt::Display for DicomOutputFilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}.dcm",
            self.study_instance_uid, self.series_instance_uid, self.sop_instance_uid
        )
    }
}

impl DicomOutputFilePath {
    fn new(
        study_instance_uid: String,
        series_instance_uid: String,
        sop_instance_uid: String,
    ) -> Self {
        Self {
            study_instance_uid,
            series_instance_uid,
            sop_instance_uid,
        }
    }

    fn to_path_buf(&self) -> PathBuf {
        format!("{}", self).into()
    }

    fn from_dicom_object(obj: &DefaultDicomObject) -> Result<Self> {
        let study_instance_uid_elem = obj.element(tags::STUDY_INSTANCE_UID)?;
        let series_instance_uid_elem = obj.element(tags::SERIES_INSTANCE_UID)?;
        let sop_instance_uid_elem = obj.element(tags::SOP_INSTANCE_UID)?;

        let study_instance_uid = study_instance_uid_elem.to_str()?;
        let series_instance_uid = series_instance_uid_elem.to_str()?;
        let sop_instance_uid = sop_instance_uid_elem.to_str()?;

        Ok(Self::new(
            study_instance_uid.to_string(),
            series_instance_uid.to_string(),
            sop_instance_uid.to_string(),
        ))
    }
}

fn anonymize(anonymizer: &Anonymizer, input_path: &PathBuf, output_path: &PathBuf) -> Result<()> {
    let input_src: Box<dyn Read> = if input_path == Path::new("-") {
        Box::new(io::stdin().lock())
    } else {
        Box::new(
            File::open(input_path)
                .with_context(|| format!("failed to open {}", input_path.display()))?,
        )
    };

    // Anonymize the input file
    let anonymized_obj = anonymizer
        .anonymize(input_src)
        .with_context(|| format!("failed to anonymize {}", input_path.display()))?;

    let output_target: Box<dyn Write> = if output_path == Path::new("-") {
        Box::new(io::stdout().lock())
    } else {
        let output_file_path = if output_path.is_dir() {
            let file_path = DicomOutputFilePath::from_dicom_object(&anonymized_obj.anonymized)?;
            &output_path.join(file_path.to_path_buf())
        } else {
            output_path
        };

        // Create intermediate output file directories if they don't exist yet
        if let Some(parent_dir) = output_file_path.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        Box::new(
            File::create(output_file_path)
                .with_context(|| format!("failed to create {}", output_file_path.display()))?,
        )
    };
    // Write the anonymized data to the output target
    let _ = anonymized_obj.write(output_target);

    Ok(())
}

fn config_create_command(args: &ConfigCreateArgs) -> Result<()> {
    let mut config_builder = match &args.diff_only {
        true => ConfigBuilder::new(),
        false => ConfigBuilder::default(),
    };

    if let Some(uid_root) = &args.uid_root {
        match uid_root.parse::<UidRoot>() {
            Ok(uid_root) => config_builder = config_builder.uid_root(uid_root),
            Err(e) => bail!(e),
        }
    }

    for tag in &args.exclude {
        config_builder = config_builder.tag_action(*tag, Action::Keep);
    }

    let config = config_builder.build();

    let mut json = serde_json::to_string_pretty(&config)?;
    json.push('\n'); // newline al final

    if args.output == Path::new("-") {
        let mut w = io::stdout().lock();
        write!(w, "{}", json)?;
    } else {
        std::fs::write(&args.output, json)
            .with_context(|| format!("failed to write config to {}", args.output.display()))?;
        info!("configuration saved to {}", args.output.display());
    }
    Ok(())
}

fn anonymize_command(args: &AnonymizeArgs) -> Result<()> {
    let input_path = args.input.clone();
    let output_path = args.output.clone();
    let config_file = args.config_file.clone();
    let uid_root = args.uid_root.clone();
    let exclude_tags = args.exclude.clone();
    let recurse = args.recursive;
    let continue_on_read_error = args.r#continue;

    let mut config_builder = ConfigBuilder::default();

    if let Some(config_path) = config_file {
        let json_content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read config from {}", config_path.display()))?;

        let config = serde_json::from_str::<Config>(&json_content)?;
        config_builder = config_builder.from_config(&config);
    }

    // UID root
    if let Some(uid_root) = uid_root {
        match uid_root.parse::<UidRoot>() {
            Ok(uid_root) => config_builder = config_builder.uid_root(uid_root),
            Err(e) => bail!(e),
        }
    }

    // tags to be excluded from anonymization
    for tag in exclude_tags {
        config_builder = config_builder.tag_action(tag, Action::Keep);
    }

    let config = config_builder.build();
    let processor = DefaultProcessor::new(config);
    let anonymizer = Anonymizer::new(processor);

    // Input is stdin or a file
    if input_path == Path::new("-") || input_path.is_file() {
        let start_time = Instant::now();

        anonymize(&anonymizer, &input_path, &output_path)?;

        let duration = start_time.elapsed();
        info!("successfully processed 1 file in {:?}", duration);

        return Ok(());
    }

    // Input is a directory
    if input_path.is_dir() {
        if output_path == Path::new("-") || !output_path.is_dir() {
            bail!("output path should be an existing directory");
        }

        let mut walk_dir = WalkDir::new(&input_path);
        if !recurse {
            walk_dir = walk_dir.max_depth(1);
        }

        // Process files
        let start_time = Instant::now();

        let processed_count = walk_dir
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path_buf = entry.into_path();
                if path_buf.is_file() {
                    Some(path_buf)
                } else {
                    None
                }
            })
            .par_bridge() // convert to a parallel iterator
            .try_fold(
                || 0, // initial value for each thread
                |count, path_buf| {
                    let result = anonymize(&anonymizer, &path_buf, &output_path);
                    match result {
                        Err(e) if continue_on_read_error => {
                            if let Some(&AnonymizationError::ReadError(_)) =
                                e.downcast_ref::<AnonymizationError>()
                            {
                                warn!("{}", e);
                                Ok(count)
                            } else {
                                Err(e)
                            }
                        }
                        Err(e) => Err(e),
                        Ok(_) => Ok(count + 1),
                    }
                },
            )
            .try_reduce(|| 0, |a, b| Ok(a + b))?;

        let duration = start_time.elapsed();
        info!(
            "successfully processed {} files in {:?}",
            processed_count, duration
        );

        return Ok(());
    }

    bail!("input should either be a file, stdin ('-') or a directory");
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let debug = cli.debug;
    let verbose = cli.verbose;

    let log_level = match (debug, verbose) {
        (true, _) => LevelFilter::Debug,
        (false, true) => LevelFilter::Info,
        (false, false) => LevelFilter::Error,
    };

    let mut builder = Builder::from_default_env();
    builder
        .format(|buf, record| {
            let level = match record.level() {
                Level::Error => "Error",
                Level::Warn => "Warning",
                Level::Info => "Info",
                Level::Debug => "Debug",
                Level::Trace => "Trace",
            };
            writeln!(buf, "{}: {}", level, record.args())
        })
        .filter(None, log_level);
    builder.init();

    // Handle commands
    match &cli.command {
        Commands::Anonymize(args) => anonymize_command(args),
        Commands::Config(cfg_cmd) => match cfg_cmd {
            ConfigCommands::Create(args) => config_create_command(args),
        },
    }
}
