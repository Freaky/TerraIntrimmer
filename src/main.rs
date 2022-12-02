#![windows_subsystem = "windows"]

use eyre::{eyre, Result, WrapErr};
use jsondata::Json;
use json5format::{Json5Format,ParsedDocument};
use libflate::gzip::{Decoder, Encoder};
use native_dialog::{FileDialog, MessageDialog, MessageType};

use std::io::Write;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    if let Some(path) = FileDialog::new().show_open_single_file()? {
        match trim_file(&path) {
            Ok(count) => {
                MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("Success!")
                    .set_text(&format!(
                        "Trimmed {} notifications from {}",
                        count,
                        path.display()
                    ))
                    .show_alert()?;
            }
            Err(e) => {
                MessageDialog::new()
                    .set_type(MessageType::Error)
                    .set_title("Failure!")
                    .set_text(&format!("{:#}", e))
                    .show_alert()?;

                Err(e)?;
            }
        }
    }

    Ok(())
}

fn trim_notifications(data: &mut Json) -> Result<usize> {
    /*
    {
        "gamestates" {
            "PavonisInteractive.TerraInvicta.TINotificationQueueState": [
                {
                    "Value": {
                        "notificationSummaryQueue": [{ ... }, ...], # trim this
                        "timerNotificationQueue": [{ ... }] # and this
                    }
                }
            ]
        }
    }
    */

    Ok(clear_array("/gamestates/PavonisInteractive.TerraInvicta.TINotificationQueueState/0/Value/notificationSummaryQueue", data)? +
        clear_array("/gamestates/PavonisInteractive.TerraInvicta.TINotificationQueueState/0/Value/timerNotificationQueue", data)?)
}

fn clear_array(path: &str, data: &mut Json) -> Result<usize> {
    let len = match data.get(path)? {
        Json::Array(a) => a.len(),
        _ => return Err(eyre!("unexpected type at {}", path))
    };
    data.set(path, Json::Array(vec![]))?;
    Ok(len)
}

fn safe_write(path: PathBuf, data: &[u8]) -> Result<()> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .wrap_err_with(|| format!("Can't open for writing: {}", path.display()))?
        .write_all(data)
        .wrap_err_with(|| format!("Write error to {}", path.display()))?;

    Ok(())
}

fn trim_file(path: &Path) -> Result<usize> {
    let gzip = matches!(path.extension().and_then(|s| s.to_str()), Some("gz"));

    let data = if gzip {
        let compressed = std::fs::read(path)?;
        let decoder = Decoder::new(&compressed[..])?;
        std::io::read_to_string(decoder)
    } else {
        std::fs::read_to_string(path)
    }?;

    let mut data: Json = data.parse()?;

    let trimmed = trim_notifications(&mut data)?;

    if trimmed == 0 {
        return Err(eyre!("No notifications trimmed"));
    }

    let mut name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let output = data.to_string();
    let parsed = ParsedDocument::from_str(&output, None)?;
    let output = Json5Format::with_options(Default::default())?.to_utf8(&parsed)?;

    if gzip {
        let mut encoder = Encoder::new(Vec::new())?;
        encoder.write_all(&output)?;
        let save = encoder.finish().into_result()?;

        name.push_str(".Trimmed.gz");
        safe_write(path.with_file_name(name), &save)?;
    } else {
        name.push_str(".Trimmed");
        safe_write(path.with_file_name(name), &output)?;
    };

    Ok(trimmed)
}
