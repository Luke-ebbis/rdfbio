use clap::ValueEnum;
use clap::{Parser, Subcommand};
use rdfbio::biordf;
use rdfbio::biordf::core::searching::{page, Pager};
use rdfbio::biordf::{
    api::omicsdi::api::SearchBuilder as OmicsDIsearchBuilder,
    api::ols::api::SearchBuilder as OlssearchBuilder,
    core::data::{dump_quads, ToRDF},
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Ttl,
    Json,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum SupportedDatabase {
    OmicsDi,
    Ols,
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Subcommands for rdfbio
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Query OmicsDI endpoint
    Query {
        /// The search database (required)
        #[arg(value_enum)]
        target: SupportedDatabase,

        /// The search query (required)
        #[arg()]
        query: String,

        /// The size of the results to fetch
        #[arg(short = 'S', long, default_value = "10")]
        size: usize,

        /// The start index for the search results
        #[arg(short = 's', long, default_value = "0")]
        start: usize,

        /// Output format: RDF (ttl) or JSON (json). The JSON retrieves the raw database hits, the ttl retrieves the linked data as interepreted by this program.
        #[arg(value_enum, short, long, default_value_t = OutputFormat::Json)]
        format: OutputFormat,

        /// Output file (optional). If not provided, prints to stdout.
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn query_command_ols(
    query: String,
    format: OutputFormat,
    output: Option<String>,
) {
    let mut binding = OlssearchBuilder::default();

    let user_query = query.clone();
    let query_builder = binding.query(&user_query).mode(biordf::api::ols::api::Mode::Backward);
    let results = query_builder.build().unwrap().search().unwrap();

    match format {
        OutputFormat::Ttl => {
            todo!();
            // let quads = results.to_quads().unwrap();
            // if let Some(file) = output {
            //     std::fs::write(file, dump_quads(quads.to_owned())).unwrap();
            // } else {
            //     println!("{}", dump_quads(quads.to_owned()));
            // }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&results).unwrap();
            if let Some(file) = output {
                std::fs::write(file, json).unwrap();
            } else {
                println!("{}", json);
            }
        }
    }
}
fn query_command_omicsdi(
    query: String,
    size: usize,
    start: usize,
    format: OutputFormat,
    output: Option<String>,
) {
    let mut binding = OmicsDIsearchBuilder::default();
    let size: i32 = size.try_into().expect("Size too large for i32");
    let start: i32 = start.try_into().expect("Start value too large for i32");
    let search_size = {
        if size > 1000 {
            1000
        } else {
            size
        }
    };

    let user_query = query.clone();
    let query_builder = binding.size(search_size).start(start).query(&user_query);
    let pager: Pager<OmicsDIsearchBuilder> = Pager::new(
        query_builder,
        rdfbio::biordf::core::searching::SearchSize::Amount(size),
    );
    let results = page(pager).unwrap();
    match format {
        OutputFormat::Ttl => {
            let quads = results.to_quads().unwrap();
            if let Some(file) = output {
                std::fs::write(file, dump_quads(quads.to_owned())).unwrap();
            } else {
                println!("{}", dump_quads(quads.to_owned()));
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&results).unwrap();
            if let Some(file) = output {
                std::fs::write(file, json).unwrap();
            } else {
                println!("{}", json);
            }
        }
    }
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Query {
            target,
            query,
            size,
            start,
            format,
            output,
        } => match target {
            SupportedDatabase::OmicsDi => query_command_omicsdi(query, size, start, format, output),
            SupportedDatabase::Ols => query_command_ols(query,  format, output),
        },
    }
}
