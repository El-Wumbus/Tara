use std::path::PathBuf;

use rustyline::{history::FileHistory, Editor};
use tara_util::paths;
use tokio::fs;

use crate::{config, error::{Error, Result}};

fn get_optional_value(rl: &mut Editor<(), FileHistory>, prompt: &str) -> Result<Option<String>> {
    let value = rl.readline(prompt).map_err(Error::ReadLine)?.trim().to_owned();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub(super) async fn init() -> Result<()> {
    // Collect all configuration values
    let mut rl = rustyline::DefaultEditor::new().unwrap();

    let token = {
        let mut token = String::new();
        while token.is_empty() {
            token = rl
                .readline("Enter Discord token [Required]: ")
                .map_err(Error::ReadLine)?
                .trim()
                .to_owned();
        }
        token
    };

    let currency_api_key = get_optional_value(&mut rl, "Enter API key for currencyapi.com [Optional]: ")?;
    let direct_message_cooldown = get_optional_value(
        &mut rl,
        "Enter cooldown, in seconds, for direct message commands [Optional]: ",
    )?;
    let direct_message_cooldown = match direct_message_cooldown {
        Some(x) => {
            Some(std::time::Duration::from_secs(
                x.parse::<u64>()
                    .map_err(|e| Error::ParseNumber(format!("\"{x}\": {e}")))?,
            ))
        }
        None => None,
    };

    let random_error_message = get_optional_value(
        &mut rl,
        "Enter path to randomErrorMessage file (Type \"default\" to use the default path) [Optional]: ",
    )?;
    let random_error_message =
        random_error_message.map_or(config::ConfigurationRandomErrorMessages::Boolean(false), |x| {
            if x == "default" {
                config::ConfigurationRandomErrorMessages::Boolean(true)
            } else {
                config::ConfigurationRandomErrorMessages::Path(PathBuf::from(x))
            }
        });


    let config_file_path = get_optional_value(
        &mut rl,
        "Enter where to save generated config file (Press Enter to use default) [Optional]: ",
    )?;
    let config_file_path = match config_file_path {
        Some(x) => PathBuf::from(x),
        None => {
            if let Some(path) = paths::TARA_CONFIGURATION_FILE.as_ref() {
                path.clone()
            } else {
                eprintln!("Couldn't get default config file location!");
                return Err(Error::MissingConfigurationFile);
            }
        }
    };


    let config = config::Configuration {
        secrets:              config::ConfigurationSecrets {
            token:            token.clone(),
            currency_api_key: currency_api_key.clone(),
            omdb_api_key:     None,
            unsplash_key:     None,
        },
        random_error_message: random_error_message.clone(),
        music:                Some(Default::default()),
    };

    let config = toml::to_string_pretty(&config).map_err(|e| {
        Error::ConfigurationSave {
            error: Box::new(e),
            path:  config_file_path.clone(),
        }
    })?;

    println!(
        "Selected Configuration:\n\ttoken = '{token}' \n\tcurrencyApiKey = {currency_api_key:?} \
         \n\tdirectMessageCooldown = {direct_message_cooldown:?} \n\trandomErrorMessage = \
         {random_error_message:?}"
    );

    // If we should continue, save, otherwise we exit.
    let cont = get_optional_value(&mut rl, "Is this okay? [y/N]: ")?.map_or(false, |mut x| {
        x = x.to_lowercase();
        x == "y" || x == "yes"
    });
    if cont {
        fs::create_dir_all(&config_file_path.parent().unwrap())
            .await
            .map_err(Error::Io)?;
        fs::write(&config_file_path, config).await.map_err(Error::Io)?;
        println!("Saved config to \"{}\"", config_file_path.display());
    } else {
        println!("Quitting...");
    }

    Ok(())
}
