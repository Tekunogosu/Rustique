use clap::Args;

#[derive(Args)]
pub struct ChangeLogArgs {
    pub(crate) name: Option<String>,
}

