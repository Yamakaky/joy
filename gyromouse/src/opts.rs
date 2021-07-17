use clap::Clap;

#[derive(Debug, Clap)]
pub enum Opts {
    Run(Run),
    List,
}

#[derive(Debug, Clap)]
pub struct Run {
    pub mapping_file: String,
}
