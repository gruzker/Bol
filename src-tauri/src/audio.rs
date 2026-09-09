use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::{
    io::Cursor,
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

pub struct Audio {
    pub wav: Vec<u8>,
    pub seconds: f64,
}
pub struct Recording {
    pub stop: mpsc::Sender<bool>,
    pub result: oneshot::Receiver<Result<Audio, String>>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Meter {
    level: f32,
    seconds: f64,
    session_id: u64,
}
pub fn devices() -> Result<Vec<String>, String> {
    let host = cpal::default_host();
    Ok(host
        .input_devices()
        .map_err(|_| "Could not list microphones. Check your devices and reopen Bol.")?
        .filter_map(|d| d.name().ok())
        .collect())
}
pub fn start(app: AppHandle, device_name: String, session_id: u64) -> Recording {
    let (stop_tx, stop_rx) = mpsc::channel();
    let (result_tx, result) = oneshot::channel();
    std::thread::spawn(move || {
        let output = record(&app, &device_name, session_id, stop_rx);
        let _ = result_tx.send(output);
    });
    Recording {
        stop: stop_tx,
        result,
    }
}
fn record(
    app: &AppHandle,
    device_name: &str,
    session_id: u64,
    stop: mpsc::Receiver<bool>,
) -> Result<Audio, String> {
    let host = cpal::default_host();
    let device = if device_name.is_empty() {
        host.default_input_device()
    } else {
        host.input_devices()
            .map_err(|_| "Could not list microphones. Check your devices and reopen Bol.")?
            .find(|d| d.name().ok().as_deref() == Some(device_name))
    }
    .ok_or("Microphone not found. Select an available microphone in Settings.")?;
    let supported = device.default_input_config().map_err(|_| {
        "Could not open the microphone. Check Windows microphone access and other recording apps."
    })?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let failure = Arc::new(Mutex::new(false));
    let channels = config.channels as usize;
    let cap = config.sample_rate.0 as usize * 300;
    let stream = match format {
        cpal::SampleFormat::F32 => build::<f32>(
            &device,
            &config,
            samples.clone(),
            failure.clone(),
            channels,
            cap,
        ),
        cpal::SampleFormat::I16 => build::<i16>(
            &device,
            &config,
            samples.clone(),
            failure.clone(),
            channels,
            cap,
        ),
        cpal::SampleFormat::U16 => build::<u16>(
            &device,
            &config,
            samples.clone(),
            failure.clone(),
            channels,
            cap,
        ),
        cpal::SampleFormat::I32 => build::<i32>(
            &device,
            &config,
            samples.clone(),
            failure.clone(),
            channels,
            cap,
        ),
        _ => {
            return Err(
                "This microphone uses an unsupported audio format. Choose another microphone in Settings.".into(),
            )
        }
    }
    .map_err(|_| "Could not start recording. Check microphone access in Windows Settings.")?;
    stream
        .play()
        .map_err(|_| "The microphone could not start. Check the device and try again.")?;
    let started = Instant::now();
    loop {
        match stop.recv_timeout(Duration::from_millis(70)) {
            Ok(cancel) => {
                if cancel {
                    return Err("Recording cancelled.".into());
                }
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return Err("Recording cancelled.".into()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if *failure.lock().unwrap() {
            return Err("The microphone disconnected or stopped working. Select another microphone in Settings.".into());
        }
        let data = samples.lock().unwrap();
        let n = data.len().min(2048);
        let level = if n == 0 {
            0.0
        } else {
            (data[data.len() - n..].iter().map(|v| v * v).sum::<f32>() / n as f32).sqrt()
        };
        let _ = app.emit(
            "audio-level",
            Meter {
                level: (level * 7.0).min(1.0),
                seconds: started.elapsed().as_secs_f64(),
                session_id,
            },
        );
        if started.elapsed() >= Duration::from_secs(300) {
            break;
        }
    }
    drop(stream);
    if *failure.lock().unwrap() {
        return Err("The microphone stopped working. Check the device and record again.".into());
    }
    let samples = samples.lock().unwrap();
    let seconds = samples.len() as f64 / config.sample_rate.0 as f64;
    if seconds < 0.25 {
        return Err("The recording was too short. Speak for a moment, then stop recording.".into());
    }
    let peak = samples.iter().fold(0.0_f32, |p, s| p.max(s.abs()));
    if peak < 0.002 {
        return Err("No audio detected. Check your microphone and try again.".into());
    }
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(
            &mut cursor,
            hound::WavSpec {
                channels: 1,
                sample_rate: config.sample_rate.0,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .map_err(|_| "Could not prepare the recording. Please try again.")?;
        for sample in samples.iter() {
            writer
                .write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                .map_err(|_| "Could not prepare the recording. Please try again.")?;
        }
        writer
            .finalize()
            .map_err(|_| "Could not finish preparing the recording. Please try again.")?;
    }
    Ok(Audio {
        wav: cursor.into_inner(),
        seconds,
    })
}
fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    failed: Arc<Mutex<bool>>,
    channels: usize,
    cap: usize,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mut output = samples.lock().unwrap();
            for frame in data.chunks(channels) {
                if output.len() >= cap {
                    break;
                }
                let mono = frame
                    .iter()
                    .map(|s| <f32 as cpal::FromSample<T>>::from_sample_(*s))
                    .sum::<f32>()
                    / channels as f32;
                output.push(if mono.is_finite() { mono } else { 0.0 });
            }
        },
        move |_| {
            *failed.lock().unwrap() = true;
        },
        None,
    )
}
