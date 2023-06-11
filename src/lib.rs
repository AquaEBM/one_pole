#![feature(portable_simd)]

use std::{f32::consts::PI, sync::Arc, array};

use plugin_util::{
    util::map,
    smoothing::{LogSmoother, SIMDSmoother},
    filter::Integrator,
};

use nih_plug::prelude::*;
use core_simd::simd::*;

#[derive(Params)]
pub struct OnePoleParams {
    #[id = "cutoff"]
    cutoff: FloatParam,
}

impl Default for OnePoleParams {
    fn default() -> Self {

        Self {

            cutoff: FloatParam::new(
                "Cutoff",
                660.,
                FloatRange::Skewed {
                    min: 13.,
                    max: 21000.,
                    factor: 0.2,
                },
            ),
        }
    }
}

#[derive(Default)]
pub struct OnePole<const N: usize>
where
    LaneCount<N>: SupportedLaneCount
{
    params: Arc<OnePoleParams>,
    g: LogSmoother<N>,
    integrator: Integrator<N>,
    pi_tick: Simd<f32, N>,
}

impl<const N: usize> OnePole<N>
where
    LaneCount<N>: SupportedLaneCount
{
    fn update_smoothers(&mut self, block_len: usize) {
        let target_cutoff = Simd::splat(self.params.cutoff.unmodulated_plain_value());
        let g = map(target_cutoff * self.pi_tick, f32::tan);

        self.g.set_target(
            g / (Simd::splat(1.) + g),
            block_len
        );
    }

    fn process(&mut self, sample: Simd<f32, N>) -> Simd<f32, N> {

        self.g.tick();

        self.integrator.process(
            sample - self.integrator.previous_output(),
            *self.g.current()
        )
    }

    fn reset(&mut self) {
        self.integrator.reset()
    }

    fn set_sample_rate(&mut self, sr: f32) {
        self.pi_tick = Simd::splat(PI / sr);
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
}

impl<const N: usize> Plugin for OnePole<N>
where
    LaneCount<N>: SupportedLaneCount
{
    const NAME: &'static str = "One Pole Filter";

    const VENDOR: &'static str = "AquaEBM";

    const URL: &'static str = "monkey.com";

    const EMAIL: &'static str = "monke@monkey.com";

    const VERSION: &'static str = "0.6.9";

    const MIDI_INPUT: MidiConfig = MidiConfig::None;

    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    const HARD_REALTIME_ONLY: bool = false;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(N as u32),
            main_output_channels: NonZeroU32::new(N as u32),
            ..AudioIOLayout::const_default()
        }
    ];

    type SysExMessage = ();

    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params()
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {

        self.update_smoothers(buffer.samples());

        for mut frame in buffer.iter_samples() {

            let mut sample = array::from_fn(
                |i| *unsafe { frame.get_unchecked_mut(i) }
            ).into();

            sample = self.process(sample);

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

        self.set_sample_rate(buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.reset();
    }
}

impl<const N: usize> Vst3Plugin for OnePole<N>
where
    LaneCount<N>: SupportedLaneCount
{
    const VST3_CLASS_ID: [u8; 16] = *b"one_pole_monkeee";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Filter,
        Vst3SubCategory::Fx,
    ];
}

impl<const N: usize> ClapPlugin for OnePole<N>
where
    LaneCount<N>: SupportedLaneCount
{
    const CLAP_ID: &'static str = "com.AquaEBM.one_pole_filter";

    const CLAP_DESCRIPTION: Option<&'static str> = Some("Linear one-pole Filter");

    const CLAP_MANUAL_URL: Option<&'static str> = None;

    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Filter,
    ];
}

nih_export_clap!(OnePole<2>);
nih_export_vst3!(OnePole<2>);