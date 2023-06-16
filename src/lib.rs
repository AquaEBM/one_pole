#![feature(portable_simd)]

use std::{sync::Arc, array};

use plugin_util::{
    filter::one_pole::OnePole,
    simd::*
};

use nih_plug::prelude::*;

const MIN_FREQ: f32 = 13.;
const MAX_FREQ: f32 = 21000.;

#[derive(Params)]
pub struct OnePoleParams {
    #[id = "cutoff"]
    cutoff: FloatParam,
    #[id = "gain"]
    gain: FloatParam,
    #[id = "mode"]
    mode: EnumParam<FilterMode>
}

#[derive(Enum, PartialEq, Eq)]
enum FilterMode {
    #[name = "Highpass"]
    HP,
    #[name = "Lowpass"]
    LP,
    #[name = "Allpass"]
    AP,
    #[name = "Low Shelf"]
    LSH,
    #[name = "High Shelf"]
    HSH,
}

impl FilterMode {
    pub fn output_function<const N: usize>(&self) -> fn(&OnePole<N>) -> Simd<f32, N>
    where
        LaneCount<N>: SupportedLaneCount
    {
        match self {
            FilterMode::HP => OnePole::<N>::get_highpass,
            FilterMode::LP => OnePole::<N>::get_lowpass,
            FilterMode::AP => OnePole::<N>::get_allpass,
            FilterMode::LSH => OnePole::<N>::get_lowshelf,
            FilterMode::HSH => OnePole::<N>::get_highshelf,
        }
    }
}

impl Default for OnePoleParams {
    fn default() -> Self {

        Self {

            cutoff: FloatParam::new(
                "Cutoff",
                0.5,
                FloatRange::Linear { min: 0., max: 1. }
            )
            .with_value_to_string(Arc::new(
                |value| (MIN_FREQ * (MAX_FREQ / MIN_FREQ).powf(value)).to_string()
            )),

            gain: FloatParam::new(
                "Gain",
                0.,
                FloatRange::Linear { min: -30., max: 30. }
            )
            .with_unit(" db"),

            mode: EnumParam::new("Filter Mode", FilterMode::AP)
        }
    }
}

#[derive(Default)]
pub struct OnePoleFilter {
    params: Arc<OnePoleParams>,
    filter: OnePole<2>,
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

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        }
    ];

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

        let block_len = buffer.samples();

        let cutoff = MIN_FREQ * (MAX_FREQ / MIN_FREQ).powf(
            self.params.cutoff.unmodulated_plain_value()
        );

        let filter_mode = self.params.mode.unmodulated_plain_value();

        let factor = match filter_mode {
            FilterMode::LSH => -1.,
            FilterMode::HSH => 1.,
            _ => 0.,
        };

        let gain = 10f32.powf(self.params.gain.unmodulated_plain_value() * factor * (1. / 20.));

        self.filter.set_params_smoothed(
            Simd::splat(cutoff),
            Simd::splat(gain),
            block_len
        );

        let get_output = filter_mode.output_function::<2>();

        for mut frame in buffer.iter_samples() {

            self.filter.update_smoothers();

            let mut sample = array::from_fn(
                |i| *unsafe { frame.get_unchecked_mut(i) }
            ).into();

            self.filter.process(sample);

            sample = get_output(&self.filter);

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

        self.filter.set_sample_rate(buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.filter.reset();
    }
}

impl Vst3Plugin for OnePoleFilter {
    const VST3_CLASS_ID: [u8; 16] = *b"one_pole_monkeee";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Filter,
        Vst3SubCategory::Fx,
    ];
}

impl ClapPlugin for OnePoleFilter {
    const CLAP_ID: &'static str = "com.AquaEBM.one_pole_filter";

    const CLAP_DESCRIPTION: Option<&'static str> = Some("Linear one-pole Filter");

    const CLAP_MANUAL_URL: Option<&'static str> = None;

    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Filter,
    ];
}

nih_export_clap!(OnePoleFilter);
nih_export_vst3!(OnePoleFilter);