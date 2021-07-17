use clap::Clap;

#[derive(Debug, Clap)]
pub struct Opts {
    pub mapping_file: String,
}
