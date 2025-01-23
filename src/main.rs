use clap::ValueEnum;
use clap::{Parser, Subcommand};
use env_logger;
use rdfbio::biordf::{
    core::data::{dump_quads, ToRDF},
    omicsdi::api::SearchBuilder,
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Ttl,
    Json,
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
    /// Query OmicsDI data
    Query {
        /// The search query (required)
        #[arg()]
        query: String,

        /// The size of the results to fetch
        #[arg(short = 'S', long, default_value = "10")]
        size: usize,

        /// The start index for the search results
        #[arg(short = 's', long, default_value = "0")]
        start: usize,

        /// Output format: RDF or JSON
        #[arg(value_enum, short, long, default_value_t = OutputFormat::Json)]
        format: OutputFormat,

        /// Output file (optional). If not provided, prints to stdout.
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Query {
            query,
            size,
            start,
            format,
            output,
        } => {
            let mut binding = SearchBuilder::default();
            let size: i32 = size.try_into().expect("Size too large for i32");
            let start: i32 =
                start.try_into().expect("Start value too large for i32");
            let builder = binding.size(size).start(start);
            let query = builder.query(query).build().unwrap();
            let results = query.search().await.unwrap();

            match format {
                OutputFormat::Ttl => {
                    let quads = results.to_quads().unwrap();
                    if let Some(file) = output {
                        std::fs::write(file, dump_quads(quads.to_owned()))
                            .unwrap();
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
    }
}
