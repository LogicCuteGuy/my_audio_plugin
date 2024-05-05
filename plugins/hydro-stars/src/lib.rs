mod dry_wet_mixer;

use std::collections::HashMap;
use std::{sync::Arc, num::NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use atomic_float::AtomicF64;

use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3};
use nih_plug::prelude::*;
use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use nih_plug_slint::plugin_component_handle::{PluginComponentHandle, PluginComponentHandleParameterEvents};
use nih_plug_slint::{WindowAttributes, editor::SlintEditor};
use plugin_canvas::drag_drop::DropOperation;
use plugin_canvas::{LogicalSize, Event, LogicalPosition};
use plugin_canvas::event::EventResponse;
use slint::SharedString;
use crate::dry_wet_mixer::DryWetMixer;

slint::include_modules!();


const MIN_WINDOW_ORDER: usize = 8;
#[allow(dead_code)]
const MIN_WINDOW_SIZE: usize = 1 << MIN_WINDOW_ORDER; // 64
const DEFAULT_WINDOW_ORDER: usize = 9;
#[allow(dead_code)]
const DEFAULT_WINDOW_SIZE: usize = 1 << DEFAULT_WINDOW_ORDER; // 2048
const MAX_WINDOW_ORDER: usize = 10;
const MAX_WINDOW_SIZE: usize = 1 << MAX_WINDOW_ORDER; // 32768

const MIN_OVERLAP_ORDER: usize = 2;
#[allow(dead_code)]
const MIN_OVERLAP_TIMES: usize = 1 << MIN_OVERLAP_ORDER; // 4
const DEFAULT_OVERLAP_ORDER: usize = 4;
#[allow(dead_code)]
const DEFAULT_OVERLAP_TIMES: usize = 1 << DEFAULT_OVERLAP_ORDER; // 16
const MAX_OVERLAP_ORDER: usize = 5;
#[allow(dead_code)]
const MAX_OVERLAP_TIMES: usize = 1 << MAX_OVERLAP_ORDER; // 32

#[derive(Params)]
pub struct PluginParams {

    #[nested(group = "global")]
    pub global: Arc<GlobalParams>,

}

#[derive(Params)]
pub struct GlobalParams {
    #[id = "scale_gui"]
    pub scale_gui: FloatParam,

    #[id = "output"]
    pub output_gain: FloatParam,

    #[id = "dry_wet"]
    pub dry_wet_ratio: FloatParam,

    #[id = "stft_window"]
    pub window_size_order: IntParam,

    #[id = "stft_overlap"]
    pub overlap_times_order: IntParam,

    #[id = "attack"]
    pub compressor_attack_ms: FloatParam,

    #[id = "release"]
    pub compressor_release_ms: FloatParam,

    #[id = "morph"]
    pub morph: BoolParam,
}

impl GlobalParams {
    fn new(update_gui_scale: Arc<AtomicBool>) -> Self {
        Self {
            scale_gui: FloatParam::new("Scale Gui", 1.0, FloatRange::Linear {
                min: 0.50,
                max: 1.5,
            }).with_unit("x").with_callback(
                {
                    let update_gui_scale = update_gui_scale.clone();
                    Arc::new(move |_| {
                        update_gui_scale.store(true, Ordering::Release);
                    })
                }
            ),
            output_gain: FloatParam::new(
                "Output Gain",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: db_to_gain(-50.0),
                    max: db_to_gain(50.0),
                    factor: FloatRange::gain_skew_factor(-50.0, 50.0),
                },
            )
                .with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            // auto_makeup_gain: BoolParam::new("Auto Makeup Gain", true),
            dry_wet_ratio: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit("%")
                .with_smoother(SmoothingStyle::Linear(15.0))
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            window_size_order: IntParam::new(
                "Window Size",
                DEFAULT_WINDOW_ORDER as i32,
                IntRange::Linear {
                    min: MIN_WINDOW_ORDER as i32,
                    max: MAX_WINDOW_ORDER as i32,
                },
            )
                .with_value_to_string(formatters::v2s_i32_power_of_two())
                .with_string_to_value(formatters::s2v_i32_power_of_two()),
            overlap_times_order: IntParam::new(
                "Window Overlap",
                DEFAULT_OVERLAP_ORDER as i32,
                IntRange::Linear {
                    min: MIN_OVERLAP_ORDER as i32,
                    max: MAX_OVERLAP_ORDER as i32,
                },
            )
                .with_value_to_string(formatters::v2s_i32_power_of_two())
                .with_string_to_value(formatters::s2v_i32_power_of_two()),

            compressor_attack_ms: FloatParam::new(
                "Attack",
                150.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
                .with_unit(" ms")
                .with_step_size(0.1),
            compressor_release_ms: FloatParam::new(
                "Release",
                300.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 10_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
                .with_unit(" ms")
                .with_step_size(0.1),
            morph: BoolParam::new("Morph", false),
        }
    }
}
pub struct PluginComponent {
    component: PluginWindow,
    param_map: HashMap<SharedString, ParamPtr>,
    gui_context: Arc<dyn GuiContext>,
}

impl PluginComponent {
    fn new(params: Arc<PluginParams>, gui_context: Arc<dyn GuiContext>) -> Self {
        let component = PluginWindow::new().unwrap();

        let param_map: HashMap<SharedString, _> = params.param_map().iter()
            .map(|(name, param_ptr, _)| {
                (name.clone().into(), *param_ptr)
            })
            .collect();

        Self {
            component,
            param_map,
            gui_context
        }
    }

    fn drag_event_response(&self, position: &LogicalPosition) -> EventResponse {
        self.component.set_drag_x(position.x as f32);
        self.component.set_drag_y(position.y as f32);

        let drop_area_x = self.component.get_drop_area_x() as f64;
        let drop_area_y = self.component.get_drop_area_y() as f64;
        let drop_area_width = self.component.get_drop_area_width() as f64;
        let drop_area_height = self.component.get_drop_area_height() as f64;

        if position.x >= drop_area_x &&
            position.x <= drop_area_x + drop_area_width &&
            position.y >= drop_area_y &&
            position.y <= drop_area_y + drop_area_height
        {
            EventResponse::DropAccepted(DropOperation::Copy)
        } else {
            EventResponse::Ignored
        }
    }

    fn convert_parameter(&self, id: &str) -> PluginParameter {
        let param_ptr = self.param_map.get(id).unwrap();

        let value = unsafe { param_ptr.unmodulated_normalized_value() };
        let default_value = unsafe { param_ptr.default_normalized_value() };
        let display_value = unsafe { param_ptr.normalized_value_to_string(value, true) };
        let modulated_value = unsafe { param_ptr.modulated_normalized_value() };

        PluginParameter {
            id: id.into(),
            default_value,
            display_value: display_value.into(),
            modulated_value,
            value,
        }
    }

    fn set_parameter(&self, id: &str, parameter: PluginParameter) {
        match id {
            "scale_gui" => {
                self.gui_context.request_resize();
                // self.component.set_scale_gui(parameter)
            },
            "gain" => self.component.set_gain(parameter),
            _ => unimplemented!(),
        }
    }
}

impl PluginComponentHandle for PluginComponent {
    fn window(&self) -> &slint::Window {
        self.component.window()
    }

    fn param_map(&self) -> &HashMap<SharedString, ParamPtr> {
        &self.param_map
    }

    fn on_event(&self, event: &Event) -> EventResponse {
        match event {
            Event::DragEntered { position, data: _ } => {
                self.component.set_dragging(true);
                self.drag_event_response(position)
            },

            Event::DragExited => {
                self.component.set_dragging(false);
                EventResponse::Handled
            },

            Event::DragMoved { position, data: _ } => {
                self.drag_event_response(position)
            },

            Event::DragDropped { position, data: _ } => {
                self.component.set_dragging(false);
                self.drag_event_response(position)
            },

            _ => EventResponse::Ignored,
        }
    }

    fn update_parameter_value(&self, id: &str) {
        let parameter = self.convert_parameter(id);
        self.set_parameter(id, parameter);
    }

    fn update_parameter_modulation(&self, id: &str) {
        self.update_parameter_value(id);
    }

    fn update_all_parameters(&self) {
        for id in self.param_map.keys() {
            self.update_parameter_value(id);
        }
    }
}

impl PluginComponentHandleParameterEvents for PluginComponent {
    fn on_start_parameter_change(&self, mut f: impl FnMut(SharedString) + 'static) {
        self.component.on_start_change(move |parameter| f(parameter.id));
    }

    fn on_parameter_changed(&self, mut f: impl FnMut(SharedString, f32) + 'static) {
        self.component.on_changed(move |parameter, value| f(parameter.id, value));
    }

    fn on_end_parameter_change(&self, mut f: impl FnMut(SharedString) + 'static) {
        self.component.on_end_change(move |parameter| f(parameter.id));
    }

    fn on_set_parameter_string(&self, mut f: impl FnMut(SharedString, SharedString) + 'static) {
        self.component.on_set_string(move |parameter, string| f(parameter.id, string));
    }
}

pub struct HydroStars {
    params: Arc<PluginParams>,

    update_gui_scale: Arc<AtomicBool>,
    user_scale: Arc<AtomicF64>,

    buffer_config: BufferConfig,
    stft: util::StftHelper<1>,
    window_function: Vec<f32>,
    dry_wet_mixer: DryWetMixer,
    plan_for_order: Option<[Plan; MAX_WINDOW_ORDER - MIN_WINDOW_ORDER + 1]>,
    complex_fft_buffer: Vec<Complex32>,
}



struct Plan {
    /// The algorithm for the FFT operation.
    r2c_plan: Arc<dyn RealToComplex<f32>>,
    /// The algorithm for the IFFT operation.
    c2r_plan: Arc<dyn ComplexToReal<f32>>,
}

impl Default for HydroStars {
    fn default() -> Self {

        let update_gui_scale = Arc::new(AtomicBool::new(false));
        Self {
            params: Arc::new(PluginParams {
                global: Arc::new(GlobalParams::new(update_gui_scale.clone())),
            }),
            update_gui_scale,
            user_scale: Arc::new(AtomicF64::new(1.0)),
            buffer_config: BufferConfig {
                sample_rate: 1.0,
                min_buffer_size: None,
                max_buffer_size: 0,
                process_mode: ProcessMode::Realtime,
            },
            stft: util::StftHelper::new(2, MAX_WINDOW_SIZE, 0),
            window_function: Vec::with_capacity(MAX_WINDOW_SIZE),
            dry_wet_mixer: DryWetMixer::new(0, 0, 0),
            plan_for_order: None,
            complex_fft_buffer: Vec::with_capacity(MAX_WINDOW_SIZE / 2 + 1),
        }
    }
}

impl Plugin for HydroStars {
    const NAME: &'static str = "HydroStars";
    const VENDOR: &'static str = "LogicCuteGuy";

    const URL: &'static str = "hydrostars.logiccuteguy.com";
    const EMAIL: &'static str = "contact@logiccuteguy.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            aux_input_ports: &[new_nonzero_u32(2)],

            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    const HARD_REALTIME_ONLY: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let window_attributes = WindowAttributes::new(
            LogicalSize::new(800.0, 600.0),
            self.user_scale.clone(),
        );

        let editor = SlintEditor::new(
            window_attributes,
            {
                let params = self.params.clone();
                move |_window, gui_context| {
                    PluginComponent::new(params.clone(), gui_context.clone())
                }
            },
        );

        Some(Box::new(editor))
    }

    fn reset(&mut self) {
        self.dry_wet_mixer.reset();
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>
    ) -> bool
    {
        // Needed to update the compressors later
        self.buffer_config = *buffer_config;

        // This plugin can accept a variable number of audio channels, so we need to resize
        // channel-dependent data structures accordingly
        let num_output_channels = audio_io_layout
            .main_output_channels
            .expect("Plugin does not have a main output")
            .get() as usize;
        if self.stft.num_channels() != num_output_channels {
            self.stft = util::StftHelper::new(self.stft.num_channels(), MAX_WINDOW_SIZE, 0);
        }
        self.dry_wet_mixer.resize(
            num_output_channels,
            buffer_config.max_buffer_size as usize,
            MAX_WINDOW_SIZE,
        );

        // Planning with RustFFT is very fast, but it will still allocate we we'll plan all of the
        // FFTs we might need in advance
        if self.plan_for_order.is_none() {
            let mut planner = RealFftPlanner::new();
            let plan_for_order: Vec<Plan> = (MIN_WINDOW_ORDER..=MAX_WINDOW_ORDER)
                .map(|order| Plan {
                    r2c_plan: planner.plan_fft_forward(1 << order),
                    c2r_plan: planner.plan_fft_inverse(1 << order),
                })
                .collect();
            self.plan_for_order = Some(
                plan_for_order
                    .try_into()
                    .unwrap_or_else(|_| panic!("Mismatched plan orders")),
            );
        }

        let window_size = self.window_size();
        self.resize_for_window(window_size);
        context.set_latency_samples(self.stft.latency_samples());
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer<'_>,
        _aux: &mut AuxiliaryBuffers<'_>,
        context: &mut impl ProcessContext<Self>
    ) -> ProcessStatus
    {
        if self
            .update_gui_scale
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.user_scale.store(self.params.global.scale_gui.value() as f64, Ordering::Release);
        }
        // If the window size has changed since the last process call, reset the buffers and chance
        // our latency. All of these buffers already have enough capacity so this won't allocate.
        let window_size = self.window_size();
        let overlap_times = self.overlap_times();
        if self.window_function.len() != window_size {
            self.resize_for_window(window_size);
            context.set_latency_samples(self.stft.latency_samples());
        }

        // These plans have already been made during initialization we can switch between versions
        // without reallocating
        let fft_plan = &mut self.plan_for_order.as_mut().unwrap()
            [self.params.global.window_size_order.value() as usize - MIN_WINDOW_ORDER];
        let num_bins = self.complex_fft_buffer.len();
        // The Hann window function spreads the DC signal out slightly, so we'll clear all 0-20 Hz
        // bins for this. With small window sizes you probably don't want this as it would result in
        // a significant low-pass filter. When it's disabled, the DC bin will also be compressed.
        let first_non_dc_bin_idx =
            (20.0 / ((self.buffer_config.sample_rate / 2.0) / num_bins as f32)).floor() as usize
                + 1;

        // The overlap gain compensation is based on a squared Hann window, which will sum perfectly
        // at four times overlap or higher. We'll apply a regular Hann window before the analysis
        // and after the synthesis.
        let gain_compensation: f32 =
            ((overlap_times as f32 / 4.0) * 1.5).recip() / window_size as f32;

        // We'll apply the square root of the total gain compensation at the DFT and the IDFT
        // stages. That way the compressor threshold values make much more sense. This version of
        // Spectral Compressor does not have in input gain option and instead has the curve
        // threshold option. When sidechaining is enabled this is used to gain up the sidechain
        // signal instead.
        let input_gain = gain_compensation.sqrt();
        let output_gain = self.params.global.output_gain.value() * gain_compensation.sqrt();
        // TODO: Auto makeup gain

        // This is mixed in later with latency compensation applied
        self.dry_wet_mixer.write_dry(buffer);
        ProcessStatus::Normal
    }
}

impl HydroStars {
    fn window_size(&self) -> usize {
        1 << self.params.global.window_size_order.value() as usize
    }

    fn overlap_times(&self) -> usize {
        1 << self.params.global.overlap_times_order.value() as usize
    }

    /// `window_size` should not exceed `MAX_WINDOW_SIZE` or this will allocate.
    fn resize_for_window(&mut self, window_size: usize) {
        // The FFT algorithms for this window size have already been planned in
        // `self.plan_for_order`, and all of these data structures already have enough capacity, so
        // we just need to change some sizes.
        self.stft.set_block_size(window_size);
        self.window_function.resize(window_size, 0.0);
        util::window::hann_in_place(&mut self.window_function);
        self.complex_fft_buffer
            .resize(window_size / 2 + 1, Complex32::default());
        
    }
}

impl ClapPlugin for HydroStars {
    const CLAP_ID: &'static str = "com.logiccuteguy.hydrostars";
    const CLAP_DESCRIPTION: Option<&'static str> = None;
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Filter,
        ClapFeature::Equalizer,
        ClapFeature::Custom("spectral"),
    ];
}

impl Vst3Plugin for HydroStars {
    const VST3_CLASS_ID: [u8; 16] = *b"HydroStarsPlugin";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Filter,
        Vst3SubCategory::Eq,
        Vst3SubCategory::Custom("Spectral")];
}

nih_export_clap!(HydroStars);
nih_export_vst3!(HydroStars);
