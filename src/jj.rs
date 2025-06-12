use crate::cli::Args;

use anyhow::Result;
use jj_lib::{
    config::StackedConfig,
    repo::StoreFactories,
    settings::UserSettings,
    workspace::{Workspace, default_working_copy_factories},
};

pub struct Jj {
    workspace: Workspace,
}

impl Jj {
    pub fn load(args: &Args) -> Result<Self> {
        let user_settings = UserSettings::from_config(StackedConfig::with_defaults())?;
        let store_factories = StoreFactories::default();
        let working_copy_factories = default_working_copy_factories();

        let workspace = Workspace::load(
            &user_settings,
            &args.path,
            &store_factories,
            &working_copy_factories,
        )?;

        Ok(Jj { workspace })
    }

    pub fn get_commits(&self) -> Result<()> {
        Ok(())
    }
}
