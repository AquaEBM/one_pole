#![feature(portable_simd)]

use plugin_util::{
    filter::one_pole::{FilterMode, OnePole},
    simd::*,
};

use nih_plug::prelude::*;

use core::f32::consts::TAU;
use std::sync::Arc;

const MIN_FREQ: f32 = 13.;
const MAX_FREQ: f32 = 21000.;

const NUM_CHANNELS: usize = 2;

type Filter = OnePole<NUM_CHANNELS>;

#[derive(Params)]
pub struct OnePoleParams {
    #[id = "cutoff"]
    cutoff: FloatParam,
    #[id = "gain"]
    gain: FloatParam,
    #[id = "mode"]
    mode: EnumParam<FilterMode>,
}

impl Default for OnePoleParams {
    fn default() -> Self {
        Self {
            cutoff: FloatParam::new("Cutoff", 0.5, FloatRange::Linear { min: 0., max: 1. })
                .with_value_to_string(Arc::new(|value| {
                    (MIN_FREQ * (MAX_FREQ / MIN_FREQ).powf(value)).to_string()
                })),

            gain: FloatParam::new(
                "Gain",
                0.,
                FloatRange::Linear {
                    min: -30.,
                    max: 30.,
                },
            )
            .with_unit(" db"),

            mode: EnumParam::new("Filter Mode", FilterMode::default()),
        }
    }
}

impl OnePoleParams {
    fn get_values(&self, pi_tick: f32) -> (f32x2, f32x2, FilterMode) {
        let cutoff_normalized = self.cutoff.unmodulated_plain_value();
        let gain_normalized = self.gain.unmodulated_plain_value();
        (
            Simd::splat(pi_tick * MIN_FREQ * (MAX_FREQ / MIN_FREQ).powf(cutoff_normalized)),
            Simd::splat(10f32.powf(gain_normalized * (1. / 20.))),
            self.mode.unmodulated_plain_value(),
        )
    }
}

#[derive(Default)]
pub struct OnePoleFilter {
    params: Arc<OnePoleParams>,
    pi_tick: f32,
    filter: Filter,
}

impl Plugin for OnePoleFilter {
    const NAME: &'static str = "One Pole Filter";

    const VENDOR: &'static str = "AquaEBM";

    const URL: &'static str = "monkey.com";

    const EMAIL: &'static str = "monke@monkey.com";

    const VERSION: &'static str = "0.6.9";

    const MIDI_INPUT: MidiConfig = MidiConfig::None;

    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    const HARD_REALTIME_ONLY: bool = false;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(NUM_CHANNELS as u32),
        main_output_channels: NonZeroU32::new(NUM_CHANNELS as u32),
        ..AudioIOLayout::const_default()
    }];

    type SysExMessage = ();

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let (w_c, gain, mode) = self.params.get_values(self.pi_tick);
        let update = Filter::get_smoothing_update_function(mode);
        let get_output = Filter::get_output_function(mode);

        let f = &mut self.filter;

        let num_samples = buffer.samples();
        update(f, w_c, gain, num_samples);

        for mut frame in buffer.iter_samples() {
            // SAFETY: we only support a stereo configuration so these indices are valid

            let mut sample = Simd::from_array(unsafe {
                [*frame.get_unchecked_mut(0), *frame.get_unchecked_mut(1)]
            });

            f.update_smoothers();
            f.process(sample);

            sample = get_output(f);

            unsafe {
                *frame.get_unchecked_mut(0) = sample[0];
                *frame.get_unchecked_mut(1) = sample[1];
            }
        }

        ProcessStatus::Normal
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        None
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.pi_tick = TAU / buffer_config.sample_rate;

        let (w_c, gain, mode) = self.params.get_values(self.pi_tick);
        let update = Filter::get_update_function(mode);

        update(&mut self.filter, w_c, gain);
        true
    }

    fn reset(&mut self) {
        self.filter.reset();
    }
}

impl Vst3Plugin for OnePoleFilter {
    const VST3_CLASS_ID: [u8; 16] = *b"one_pole_monkeee";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Filter, Vst3SubCategory::Fx];
}

impl ClapPlugin for OnePoleFilter {
    const CLAP_ID: &'static str = "com.AquaEBM.one_pole_filter";

    const CLAP_DESCRIPTION: Option<&'static str> = Some("Linear one-pole Filter");

    const CLAP_MANUAL_URL: Option<&'static str> = None;

    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Filter];
}

nih_export_clap!(OnePoleFilter);
nih_export_vst3!(OnePoleFilter);
