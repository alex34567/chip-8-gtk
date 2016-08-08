use sdl2;
use sdl2::audio::*;

extern crate chip_8_core;

use chip_8_core::*;

pub struct SimpleAudioDevice {
    current_feq: f32,
    feq_inc: f32,
    feq_target: f32,
    volume: f32,
}

impl AudioCallback for SimpleAudioDevice {
    type Channel = f32;

    #[allow(unknown_lints)] // Overwriting a clippy lint.
    #[allow(float_cmp)] // Signum only returns a 1, -1, 0 or NAN.
    fn callback(&mut self, out: &mut [f32]) {
        for x in out {
            if self.feq_inc.signum() == 1.0 {
                if (self.current_feq - self.feq_target) > 0.000005 {
                    self.feq_target = -self.feq_target;
                    self.feq_inc = -self.feq_inc;
                }
            } else if (self.current_feq - self.feq_target) < 0.000005 {
                self.feq_target = -self.feq_target;
                self.feq_inc = -self.feq_inc;
            }
            *x = self.volume / self.current_feq;
            self.current_feq += self.feq_inc;
        }
    }
}

pub struct SdlAudioWrapper<CB: AudioCallback>(AudioDevice<CB>);

impl<CB: AudioCallback> AudioWrapper for SdlAudioWrapper<CB> {
    fn play(&mut self) {
        self.0.resume();
    }

    fn stop(&mut self) {
        self.0.pause();
    }
}

pub fn init_sound() -> SdlAudioWrapper<SimpleAudioDevice> {
    let sdl = sdl2::init().unwrap();
    let sdl_audio = sdl.audio().unwrap();
    let spec = AudioSpecDesired {
        freq: None,
        channels: Some(1),
        samples: None
    };
    let sdl_audio_device = sdl_audio.open_playback(None, &spec, |spec| {
        SimpleAudioDevice {
            current_feq: 587.33,
            feq_inc: spec.freq as f32 / 587.33,
            feq_target: 587.33,
            volume: 1.00,
        }
    }).unwrap();
    SdlAudioWrapper(sdl_audio_device)
}
