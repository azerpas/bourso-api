use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use std::{
    fs,
    io::{stderr, IsTerminal},
    path::PathBuf,
};
use tracing_appender::rolling;
use tracing_subscriber::{
    filter::LevelFilter,
    fmt::{self, format::debug_fn},
    prelude::*,
    registry, EnvFilter,
};

use crate::settings::consts::{
    APP_NAME, APP_ORGANIZATION, APP_QUALIFIER, DEFAULT_LOG_LEVEL, LOG_FILE,
};

pub struct LoggerBuilder {
    directory: PathBuf, // platform config directory (from ProjectDirs)
    file: &'static str, // "bourso.log"
}

impl LoggerBuilder {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
            .ok_or_else(|| anyhow!("Could not determine project directories"))?;

        Ok(Self {
            directory: project_dirs.data_dir().to_path_buf(),
            file: LOG_FILE,
        })
    }

    pub fn init(self) -> Result<()> {
        fs::create_dir_all(&self.directory)?;

        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_LEVEL));

        let file_appender = rolling::never(&self.directory, &self.file);

        let console_layer = fmt::layer()
            .with_writer(stderr)
            .with_ansi(IsTerminal::is_terminal(&stderr()))
            .with_level(true)
            .without_time()
            .compact()
            .fmt_fields({
                debug_fn(move |writer, field, value| {
                    if field.name() == "message" {
                        write!(writer, "{:?}", value)?;
                    }
                    Ok(())
                })
            })
            .with_filter(env_filter);

        let json_layer = fmt::layer()
            .json()
            .with_writer(file_appender)
            .with_target(true)
            .with_level(true)
            .flatten_event(true)
            .with_filter(LevelFilter::TRACE);

        registry().with(console_layer).with(json_layer).init();

        Ok(())
    }
}
