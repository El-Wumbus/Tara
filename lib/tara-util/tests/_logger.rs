// use std::num::NonZeroU64;

// use chrono::Utc;
// use tara_util::logging::{CommandLogger, LoggedCommandEvent};
// use temp_dir::TempDir;
// use tracing_subscriber::{prelude::*, registry};

// #[cfg(test)]
// #[ctor::ctor]
// fn init() {
//     // Setup logging
//     let filter = tracing_subscriber::EnvFilter::builder()
//         .with_default_directive(tracing::metadata::LevelFilter::INFO.into())
//         .parse("")
//         .unwrap();
//     registry()
//         .with(tracing_subscriber::fmt::layer().with_filter(filter))
//         .init();
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn logger() {
//     let one = NonZeroU64::new(1).unwrap();
//     let command_event = LoggedCommandEvent {
//         name:              String::new(),
//         time:              Utc::now(),
//         channel_id:        one,
//         user:              (String::new(), one),
//         called_from_guild: false,
//         guild_info:        Some((String::new(), one)),
//     };

//     let directory = TempDir::new().unwrap();
//     let path = directory.path().join("command-log.csv");
//     let logger = CommandLogger::new();

//     let logger_handle = tokio::spawn({
//         let logger = logger.clone();
//         async move { logger.log_to_file(path).await.unwrap() }
//     });


//     logger_handle.abort();
//     drop(directory);
// }
