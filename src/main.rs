use arrow::json::ReaderBuilder;
use clap::{Clap, ValueHint};
use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, Encoding},
    errors::ParquetError,
    file::properties::WriterProperties,
};
use serde_json::to_string_pretty;
use std::fs::File;
use std::path::PathBuf;

#[derive(clap::ArgEnum)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
enum ParquetCompression {
    UNCOMPRESSED,
    SNAPPY,
    GZIP,
    LZO,
    BROTLI,
    LZ4,
    ZSTD,
}

#[derive(clap::ArgEnum)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
enum ParquetEncoding {
    PLAIN,
    RLE,
    BIT_PACKED,
    DELTA_BINARY_PACKED,
    DELTA_LENGTH_BYTE_ARRAY,
    DELTA_BYTE_ARRAY,
    RLE_DICTIONARY,
}

#[derive(Clap)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = "Dominik Moritz <domoritz@cmu.edu>")]
struct Opts {
    /// Input JSON file.
    #[clap(name = "JSON", parse(from_os_str), value_hint = ValueHint::AnyPath)]
    input: PathBuf,

    /// Output file.
    #[clap(name = "PARQUET", parse(from_os_str), value_hint = ValueHint::AnyPath)]
    output: PathBuf,

    /// The number of records to infer the schema from. All rows if not present.
    /// Setting max-read-records to zero will stop schema inference.
    /// All columns will be string typed
    #[clap(long)]
    max_read_records: Option<usize>,

    /// Set the compression.
    #[clap(short, long, arg_enum)]
    compression: Option<ParquetCompression>,

    /// Sets encoding for any column.
    #[clap(short, long, arg_enum)]
    encoding: Option<ParquetEncoding>,

    /// Sets data page size limit.
    #[clap(long)]
    data_pagesize_limit: Option<usize>,

    /// Sets dictionary page size limit.
    #[clap(long)]
    dictionary_pagesize_limit: Option<usize>,

    /// Sets write batch size.
    #[clap(long)]
    write_batch_size: Option<usize>,

    /// Sets max size for a row group.
    #[clap(long)]
    max_row_group_size: Option<usize>,

    /// Sets "created by" property.
    #[clap(long)]
    created_by: Option<String>,

    /// Sets flag to enable/disable dictionary encoding for any column.
    #[clap(long)]
    dictionary: bool,

    /// Sets flag to enable/disable statistics for any column.
    #[clap(long)]
    statistics: bool,

    /// Sets max statistics size for any column. Applicable only if statistics are enabled._
    #[clap(long)]
    max_statistics_size: Option<usize>,

    /// Print the schema to stderr.
    #[clap(short, long)]
    print_schema: bool,

    /// Only print the schema
    #[clap(short = 'n', long)]
    dry: bool,
}

fn main() -> Result<(), ParquetError> {
    let opts: Opts = Opts::parse();

    let input = File::open(opts.input)?;
    let builder = ReaderBuilder::new().infer_schema(opts.max_read_records);
    let reader = builder.build(input)?;

    if opts.print_schema || opts.dry {
        let json = to_string_pretty(&reader.schema().to_json()).unwrap();
        eprintln!("Inferred Schema:\n{}", json);

        if opts.dry {
            return Ok(());
        }
    }

    let output = File::create(opts.output)?;

    let mut props = WriterProperties::builder()
        .set_dictionary_enabled(opts.dictionary)
        .set_statistics_enabled(opts.statistics);

    if let Some(compression) = opts.compression {
        let compression = match compression {
            ParquetCompression::UNCOMPRESSED => Compression::UNCOMPRESSED,
            ParquetCompression::SNAPPY => Compression::SNAPPY,
            ParquetCompression::GZIP => Compression::GZIP,
            ParquetCompression::LZO => Compression::LZO,
            ParquetCompression::BROTLI => Compression::BROTLI,
            ParquetCompression::LZ4 => Compression::LZ4,
            ParquetCompression::ZSTD => Compression::ZSTD,
        };

        props = props.set_compression(compression);
    }

    if let Some(encoding) = opts.encoding {
        let encoding = match encoding {
            ParquetEncoding::PLAIN => Encoding::PLAIN,
            ParquetEncoding::RLE => Encoding::RLE,
            ParquetEncoding::BIT_PACKED => Encoding::BIT_PACKED,
            ParquetEncoding::DELTA_BINARY_PACKED => Encoding::DELTA_BINARY_PACKED,
            ParquetEncoding::DELTA_LENGTH_BYTE_ARRAY => Encoding::DELTA_LENGTH_BYTE_ARRAY,
            ParquetEncoding::DELTA_BYTE_ARRAY => Encoding::DELTA_BYTE_ARRAY,
            ParquetEncoding::RLE_DICTIONARY => Encoding::RLE_DICTIONARY,
        };

        props = props.set_encoding(encoding);
    }

    if let Some(size) = opts.write_batch_size {
        props = props.set_write_batch_size(size);
    }

    if let Some(size) = opts.data_pagesize_limit {
        props = props.set_data_pagesize_limit(size);
    }

    if let Some(size) = opts.dictionary_pagesize_limit {
        props = props.set_dictionary_pagesize_limit(size);
    }

    if let Some(size) = opts.dictionary_pagesize_limit {
        props = props.set_dictionary_pagesize_limit(size);
    }

    if let Some(size) = opts.max_row_group_size {
        props = props.set_max_row_group_size(size);
    }

    if let Some(created_by) = opts.created_by {
        props = props.set_created_by(created_by);
    }

    if let Some(size) = opts.max_statistics_size {
        props = props.set_max_statistics_size(size);
    }

    let mut writer = ArrowWriter::try_new(output, reader.schema(), Some(props.build()))?;

    for batch in reader {
        match batch {
            Ok(batch) => writer.write(&batch)?,
            Err(error) => return Err(error.into()),
        }
    }

    match writer.close() {
        Ok(_) => Ok(()),
        Err(error) => Err(error),
    }
}
