use crate::exactstreamer::ExactStreamer;
use crate::gen::LowPassFilter;
use crate::recorder::Recorder;
use crate::utils::{fix_engine, load_engine, seconds_to_samples};
use clap::{value_t, value_t_or_exit, App, Arg};
use parking_lot::RwLock;
use std::sync::Arc;


#[cfg(feature = "gui")]
use crate::{
    audio::GENERATOR_BUFFER_SIZE,
    fft::FFTStreamer,
    gui::{GUIState, WATERFALL_WIDTH},
};


#[cfg(feature = "gui")]
mod audio;
#[cfg(feature = "gui")]
mod fft;
#[cfg(feature = "gui")]
mod gui;
#[cfg(feature = "gui")]
mod support;

mod constants;
mod exactstreamer;
mod gen;
mod recorder;
mod utils;

#[cfg(feature = "gui")]
const WINDOW_WIDTH: f64 = 800.0;
#[cfg(feature = "gui")]
const WINDOW_HEIGHT: f64 = 800.0;

const DEFAULT_CONFIG: &[u8] = include_bytes!("default.esc");

fn main() {
    let matches = App::new("Engine Sound Generator")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(Arg::with_name("headless").short("h").long("headless").help("CLI mode without GUI or audio playback").requires("config"))
        .arg(Arg::with_name("config").short("c").long("config").help("Sets the input file to load as an engine config").takes_value(true))
        .arg(Arg::with_name("volume").short("v").long("volume").help("Sets the master volume").default_value( "0.1"))
        .arg(Arg::with_name("rpm").short("r").long("rpm").help("Engine RPM").takes_value(true))
        .arg(Arg::with_name("warmup_time").short("w").long("warmup_time").help("Sets the time to wait in seconds before recording").default_value_if("headless", None, "3.0"))
        .arg(Arg::with_name("reclen").short("l").long("length").help("Sets the time to record in seconds. The formula for the recommended time to record to get a seamless loop is as follows:\n    let wavelength = 120.0 / rpm;\n    let crossfade = wavelength * 2.0;\n    let reclen = n * wavelength + crossfade / 2.0;").default_value_if("headless", None, "5.0"))
        .arg(Arg::with_name("output_file").short("o").long("output").help("Sets the output .wav file path").default_value_if("headless", None, "output.wav"))
        .arg(Arg::with_name("crossfade").short("f").long("crossfade").help("Crossfades the recording in the middle end-to-start to create a seamless loop, although adjusting the recording's length to the rpm is recommended. The value sets the size of the crossfade, where the final output is decreased in length by crossfade_time/2.").default_value_if("headless", None, "0.00133"))
        .arg(Arg::with_name("samplerate").short("q").long("samplerate").help("Generator sample rate").default_value("48000"))
        .arg(Arg::with_name("no-drag-drop").short("d").long("no-drag-drop").help("Disabled drag-and-drop support for the window").conflicts_with("headless"))
        .get_matches();

    let sample_rate = 20000;//value_t_or_exit!(matches, "samplerate", u32);

    let mut engine = match matches.value_of("config") {
        Some(path) => match load_engine(path, sample_rate, path.ends_with("json")) {
            Ok(engine) => {
                println!("Successfully loaded config \"{}\"", path);
                engine
            }
            Err(e) => {
                eprintln!("Failed to load engine config \"{}\": {}", path, e);
                std::process::exit(1);
            }
        },
        None => {
            let mut engine =
                ron::de::from_bytes(DEFAULT_CONFIG).expect("default config is invalid");
            fix_engine(&mut engine, sample_rate);
            engine
        }
    };
    if let Ok(rpm) = value_t!(matches, "rpm", f32) {
        engine.rpm = rpm.max(0.0);
    }

    let cli_mode = matches.is_present("headless");
            
    // sound generator
    let mut generator =
        gen::Generator::new(sample_rate, engine, LowPassFilter::new(0.5, sample_rate));

    generator.volume = value_t!(matches.value_of("volume"), f32).unwrap();

    

        {            
            static mut rpm :f32 = 1000.0;
            
            unsafe {
            generator.engine.rpm = rpm;
            }
            
            let generator = Arc::new(RwLock::new(generator));

            let (audio, fft_receiver) = match audio::init(generator.clone(), sample_rate) {
                Ok(audio) => audio,
                Err(e) => {
                    eprintln!("Failed to initialize SDL2 audio: {}", e);
                    std::process::exit(3);
                }
            };


            // GUI
            {
                'main: loop {
                    unsafe{
                        let mut generator = generator.write();
                    generator.engine.rpm = rpm;
                    rpm = rpm + 0.01;
                    }
                    
                }
            }

            // audio lives until here
            std::mem::drop(audio);
            
        }
    
}
