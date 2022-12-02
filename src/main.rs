use eyre::{eyre, Result};
use libflate::gzip::{Decoder, Encoder};
use rfd::FileDialog;
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    if let Some(path) = FileDialog::new().pick_file() {
        trim_file(&path)?;
    }

    Ok(())
}

fn trim_notifications(data: &mut Value) -> Option<usize> {
    /*
    {
        "gamestates" {
            "PavonisInteractive.TerraInvicta.TINotificationQueueState": [
                {
                    "Value": {
                        "notificationSummaryQueue": [{ ... }, ...] # trim this
                    }
                }
            ]
        }
    }
    */
    for obj in data
        .get_mut("gamestates")?
        .get_mut("PavonisInteractive.TerraInvicta.TINotificationQueueState")?
        .as_array_mut()?
    {
        if let Some(notifications) = obj
            .get_mut("Value")
            .and_then(|o| o.get_mut("notificationSummaryQueue"))
            .and_then(|a| a.as_array_mut())
        {
            let count = notifications.len();
            notifications.clear();
            return Some(count);
        }
    }

    None
}

fn safe_write(path: PathBuf, data: &[u8]) -> Result<()> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(data)?;

    Ok(())
}

fn trim_file(path: &Path) -> Result<()> {
    let gzip = matches!(path.extension().and_then(|s| s.to_str()), Some("gz"));

    let data = if gzip {
        let compressed = std::fs::read(path)?;
        let decoder = Decoder::new(&compressed[..])?;
        std::io::read_to_string(decoder)
    } else {
        std::fs::read_to_string(path)
    }?;

    let mut data: Value = serde_json::from_str(&data)?;

    let trimmed =
        trim_notifications(&mut data).ok_or(eyre!("Couldn't find notifications to trim"))?;

    if trimmed == 0 {
        return Err(eyre!("No notifications trimmed"));
    }

    let mut name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let output = serde_json::to_string_pretty(&data)?;

    if gzip {
        let mut encoder = Encoder::new(Vec::new())?;
        encoder.write_all(output.as_bytes())?;
        let save = encoder.finish().into_result()?;

        name.push_str(".Trimmed.gz");
        safe_write(path.with_file_name(name), &save)?;
    } else {
        name.push_str(".Trimmed");
        safe_write(path.with_file_name(name), output.as_bytes())?;
    };

    Ok(())
}