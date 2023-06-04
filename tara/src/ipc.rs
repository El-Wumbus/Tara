use async_trait::async_trait;
use chrono::{DateTime, Utc};
use csv_async::{AsyncReaderBuilder, StringRecord};
use tara_util::{
    ipc::{ActionMessage, ActionMessageReceiver, ResponseMessage},
    logging, paths,
};
use tokio::fs::File;

#[derive(Debug, Clone)]
pub struct ActionReceiver {}


#[async_trait]
impl ActionMessageReceiver for ActionReceiver {
    async fn perform(&self, action: ActionMessage) -> ResponseMessage {
        match action {
            ActionMessage::NoOp => ResponseMessage::ActionCompleted,
            ActionMessage::EndTransmission => unreachable!(),
            ActionMessage::GetCommandLogs {
                upper_cutoff,
                lower_cutoff,
            } => {
                // Test wether get_command_logs1 or get_command_logs2 is faster for files of differing
                // sizes
                return match get_command_logs1(lower_cutoff, upper_cutoff.unwrap_or_else(Utc::now)).await {
                    Ok(x) => x,
                    Err(e) => e,
                };
            }
        }
    }
}

async fn get_command_logs1(
    lower_cutoff: DateTime<Utc>,
    upper_cutoff: DateTime<Utc>,
) -> Result<ResponseMessage, ResponseMessage> {
    let mut deserializer = AsyncReaderBuilder::new()
        .has_headers(false)
        .create_deserializer(File::open(paths::TARA_COMMAND_LOG_PATH.as_path()).await?);
    let mut record = StringRecord::new();
    let mut command_events = Vec::new();
    while deserializer.read_record(&mut record).await? {
        let command_event = record.deserialize::<logging::LoggedCommandEvent>(None)?;
        if command_event.time > lower_cutoff && command_event.time < upper_cutoff {
            command_events.push(command_event);
        }
    }
    Ok(ResponseMessage::CommandLogs(command_events))
}

// TODO: Test
async fn _get_command_logs2(
    lower_cutoff: DateTime<Utc>,
    upper_cutoff: DateTime<Utc>,
) -> Result<ResponseMessage, ResponseMessage> {
    let mut deserializer = AsyncReaderBuilder::new()
        .has_headers(false)
        .create_deserializer(File::open(paths::TARA_COMMAND_LOG_PATH.as_path()).await?);
    let mut record = StringRecord::new();
    let mut command_events = Vec::new();
    while deserializer.read_record(&mut record).await? {
        let command_event = record.deserialize::<logging::LoggedCommandEvent>(None)?;
        command_events.push(command_event);
    }
    let lower = match command_events.binary_search_by(|x| x.time.cmp(&lower_cutoff)) {
        Ok(x) => dbg!(x),
        Err(x) => {
            dbg!(x, lower_cutoff);
            x
        }
    };
    let upper = match command_events.binary_search_by(|x| x.time.cmp(&upper_cutoff)) {
        Ok(x) => dbg!(x),
        Err(x) => {
            dbg!(x, lower_cutoff);
            x
        }
    };
    Ok(ResponseMessage::CommandLogs(
        command_events[lower..upper].to_vec(),
    ))
}
