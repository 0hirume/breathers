use std::path::Path;

use crate::{
    configuration::{self, Configuration, Selection},
    error::Failure,
};

use super::arguments::Arguments;

pub(super) struct Settings {
    pub(super) configuration: Configuration,
    pub(super) selection: Selection,
}

impl Settings {
    pub(super) fn load(arguments: &Arguments, directory: &Path) -> Result<Self, Failure> {
        let (mut configuration, root) =
            configuration::load_in(directory, arguments.config.as_deref())?;

        if !arguments.include.is_empty() {
            configuration.include = Some(arguments.include.clone());
        }

        if !arguments.exclude.is_empty() {
            configuration
                .exclude
                .get_or_insert_default()
                .extend(arguments.exclude.iter().cloned());
        }

        let selection = Selection::new(&configuration, root)?;

        Ok(Self {
            configuration,
            selection,
        })
    }
}
