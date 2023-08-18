use nih_plug::prelude::*;
use atomic_float::AtomicF32;
use std::f32::consts;
use std::sync::Arc;

use nih_plug_vizia::ViziaState;

mod editor;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

pub struct LfoCool {
    params: Arc<LfoCoolParams>,

    // In range [0, 1]
    current_phase_tau: f32,
    phase_tau_delta: f32
}

#[derive(Params)]
struct LfoCoolParams {
    /// The parameter's ID is used to identify the parameter in the wrapped plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.

    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
    #[nested(group = "plug-p")]
    pub plug_params: PlugParams,
}

#[derive(Params)]
struct PlugParams {
    #[id = "frequency"]
    pub frequency: FloatParam,
    #[id = "gain_mod"]
    pub gain_mod: FloatParam,
}

impl Default for PlugParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            frequency: FloatParam::new(
                "Frequency",
                0.,
                FloatRange::Linear {
                    min: 0.,
                    max: 100.,
                },
            )
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(2)),

            gain_mod: FloatParam::new(
                "Gain mod depth",
                util::db_to_gain(-60.0),
                FloatRange::Skewed {
                    min: util::db_to_gain( -60.),
                    max: util::db_to_gain(0.),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor( -60., 0.0),
                },
            )
                // Because the gain parameter is stored as linear gain instead of storing the value
                // as decibels, we need logarithmic smoothing
                .with_smoother(SmoothingStyle::Logarithmic(50.0))
                .with_unit(" dB")
                // There are many predefined formatters we can use here. If the gain was stored as
                // decibels instead of as a linear gain value, we could have also used the
                // `.with_step_size(0.1)` function to get internal rounding.
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Default for LfoCool {
    fn default() -> Self {
        Self {
            params: Arc::new(LfoCoolParams::default()),
            current_phase_tau: 0.,
            phase_tau_delta: 0.
        }
    }
}

impl Default for LfoCoolParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            plug_params: PlugParams::default()
        }
    }
}

impl Plugin for LfoCool {
    const NAME: &'static str = "LFOCool";
    const VENDOR: &'static str = "Starburst Audio";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "sylveon_ari@hotmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];


    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        true
    }

    fn editor(&self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.params.editor_state.clone(),
        )
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.phase_tau_delta = self.params.plug_params.frequency.smoothed.next() * consts::PI / _context.transport().sample_rate;
        for channel_samples in buffer.iter_samples() {
            // Smoothing is optionally built into the parameters themselves
            let gain: f32 = if self.params.plug_params.gain_mod.modulated_normalized_value() == 0. {
                0.
            } else { self.params.plug_params.gain_mod.smoothed.next().to_f32() };

            for sample in channel_samples {
                *sample *= {
                    let sin_sample: f32 = (1. + self.current_phase_tau.sin()) / 2.;
                    self.current_phase_tau += self.phase_tau_delta;
                    if self.current_phase_tau >= consts::TAU {
                        self.current_phase_tau -= consts::TAU;
                    }
                    1. - gain + (gain) * sin_sample
                }
            }
        }

        // TODO: Also update potential UI visualizers here

        ProcessStatus::Normal
    }
}

impl ClapPlugin for LfoCool {
    const CLAP_ID: &'static str = "com.starburstaudio.lfo-cool";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Plug-in for LFO modulation");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for LfoCool {
    const VST3_CLASS_ID: [u8; 16] = *b"StarburstLfoCool";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(LfoCool);
nih_export_vst3!(LfoCool);
