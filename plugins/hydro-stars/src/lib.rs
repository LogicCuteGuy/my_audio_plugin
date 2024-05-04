mod dry_wet_mixer;

use std::collections::HashMap;
use std::{sync::Arc, num::NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use atomic_float::AtomicF64;

use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3};
use nih_plug::prelude::*;
use realfft::num_complex::Complex32;
use realfft::{ComplexToReal, RealToComplex};
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
            user_scale: Arc::new(AtomicF64::new(1.0))
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

            ..AudioIOLayout::const_default()
        }
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

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>
    ) -> bool
    {
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer<'_>,
        _aux: &mut AuxiliaryBuffers<'_>,
        _context: &mut impl ProcessContext<Self>
    ) -> ProcessStatus
    {
        if self
            .update_gui_scale
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.user_scale.store(self.params.global.scale_gui.value() as f64, Ordering::Release);
        }
        for channel in buffer.as_slice() {
            for sample in channel.iter_mut() {
            }
        }

        ProcessStatus::Normal
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
