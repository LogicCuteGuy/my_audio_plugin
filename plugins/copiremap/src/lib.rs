mod hertz_calculator;
mod key_note_midi_gen;
mod audio_process;
mod note_table;
mod delay;
mod filter;
mod pitch;

use std::collections::HashMap;
use std::{sync::Arc, num::NonZeroU32};

use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3};
use nih_plug::prelude::*;
use nih_plug_slint::plugin_component_handle::{PluginComponentHandle, PluginComponentHandleParameterEvents};
use nih_plug_slint::{WindowAttributes, editor::SlintEditor};
use plugin_canvas::drag_drop::DropOperation;
use plugin_canvas::{LogicalSize, Event, LogicalPosition};
use plugin_canvas::event::EventResponse;
use slint::SharedString;
use crate::audio_process::{AudioProcess, AudioProcessNot, AudioProcessParams};
use crate::filter::MyFilter;
use crate::hertz_calculator::HZCalculatorParams;
use crate::key_note_midi_gen::{KeyNoteParams, MidiNote};
use crate::note_table::NoteTables;

const DB_MIN: f32 = -80.0;
const DB_MAX: f32 = 20.0;

slint::include_modules!();

#[derive(Params)]
pub struct PluginParams {

    #[persist = "note_table"]
    pub note_table: Arc<NoteTables>,

    #[persist = "audio_process_not"]
    pub audio_process_not: Arc<AudioProcessNot>,

    #[nested(group = "global")]
    pub global: Arc<GlobalParams>,

    #[nested(group = "audio_process")]
    pub audio_process: Arc<AudioProcessParams>,

    #[nested(group = "hz_calculator")]
    pub hz_calculator: Arc<HZCalculatorParams>,

    #[nested(group = "key_note")]
    pub key_note: Arc<KeyNoteParams>,

}

#[derive(Params)]
pub struct GlobalParams {

    #[id = "bypass"]
    pub bypass: BoolParam,

    #[id = "wet_gain"]
    pub wet_gain: FloatParam,

    #[id = "global_threshold"]
    pub global_threshold: FloatParam,

    #[id = "low_note_off"]
    pub low_note_off: IntParam,

    #[id = "high_note_off"]
    pub high_note_off: IntParam,

    #[id = "low_note_off_mute"]
    pub low_note_off_mute: BoolParam,

    #[id = "high_note_off_mute"]
    pub high_note_off_mute: BoolParam,

}

impl Default for GlobalParams {
    fn default() -> Self {
        Self {
            bypass: BoolParam::new("Bypass", false)
                .with_value_to_string(formatters::v2s_bool_bypass())
                .with_string_to_value(formatters::s2v_bool_bypass()),
            wet_gain: FloatParam::new("Wet Gain", db_to_gain(0.0), FloatRange::Skewed {
                min: db_to_gain(-30.0),
                max: db_to_gain(20.0),
                factor: FloatRange::gain_skew_factor(-30.0, 20.0),
            }).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            global_threshold: FloatParam::new("Global Threshold", db_to_gain(0.0), FloatRange::Linear {
                min: db_to_gain(-60.0),
                max: db_to_gain(0.0),
            }).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            low_note_off: IntParam::new(
                "Low Note Off",
                0,
                IntRange::Linear {
                    min: 0,
                    max: 127,
                },
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter()),
            high_note_off: IntParam::new(
                "High Note Off",
                127,
                IntRange::Linear {
                    min: 0,
                    max: 127,
                }
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter()),
            low_note_off_mute: BoolParam::new(
                "Low Note Off Mute",
                false,
            ),
            high_note_off_mute: BoolParam::new(
                "High Note Off Mute",
                false,
            ),
        }
    }
}

pub struct PluginComponent {
    component: PluginWindow,
    param_map: HashMap<SharedString, ParamPtr>,
}

impl PluginComponent {
    fn new(params: Arc<PluginParams>) -> Self {
        let component = PluginWindow::new().unwrap();
        let param_map: HashMap<SharedString, _> = params.param_map().iter()
            .map(|(name, param_ptr, _)| {
                (name.clone().into(), *param_ptr)
            })
            .collect();

        Self {
            component,
            param_map,
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
        let param_ptr = self.param_map.get(id.into()).unwrap();

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
        self.component.on_start_change(move |parameter| f(parameter.id.into()));
    }

    fn on_parameter_changed(&self, mut f: impl FnMut(SharedString, f32) + 'static) {
        self.component.on_changed(move |parameter, value| f(parameter.id.into(), value));
    }

    fn on_end_parameter_change(&self, mut f: impl FnMut(SharedString) + 'static) {
        self.component.on_end_change(move |parameter| f(parameter.id.into()));
    }

    fn on_set_parameter_string(&self, mut f: impl FnMut(SharedString, SharedString) + 'static) {
        self.component.on_set_string(move |parameter, string| f(parameter.id.into(), string));
    }
}

pub struct CoPiReMapPlugin {
    params: Arc<PluginParams>,
    buffer_config: BufferConfig,
    midi_note: MidiNote,
    audio_process: [AudioProcess; 128],
    lpf: MyFilter,
    hpf: MyFilter
}

impl Default for CoPiReMapPlugin {
    fn default() -> Self {
        let params = Arc::new(PluginParams {
            note_table: Arc::new(NoteTables::default()),
            audio_process_not: Arc::new(AudioProcessNot { pitch_shift_window_duration_ms: 2 }),
            global: Arc::new(GlobalParams::default()),
            audio_process: Arc::new(AudioProcessParams::default()),
            hz_calculator: Arc::new(Default::default()),
            key_note: Arc::new(Default::default()),
        });

        Self {
            params,
            buffer_config: BufferConfig {
                sample_rate: 1.0,
                min_buffer_size: None,
                max_buffer_size: 0,
                process_mode: ProcessMode::Realtime,
            },
            midi_note: MidiNote::default(),
            audio_process: [AudioProcess::default(); 128],
            lpf: MyFilter::default(),
            hpf: MyFilter::default(),
        }
    }
}

impl Plugin for CoPiReMapPlugin {
    type BackgroundTask = ();
    type SysExMessage = ();

    const NAME: &'static str = "CoPiReMap";
    const VENDOR: &'static str = "LogicCuteGuy";
    const URL: &'static str = "copiremap.logiccuteguy.com";
    const EMAIL: &'static str = "contact@logiccuteguy.com";
    const VERSION: &'static str = "0.0";

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            ..AudioIOLayout::const_default()
        }
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    const HARD_REALTIME_ONLY: bool = false;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>
    ) -> bool
    {
        self.buffer_config = *buffer_config;
        true
    }

    fn reset(&mut self) {

    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let window_attributes = WindowAttributes::new(
            LogicalSize::new(800.0, 600.0),
            0.75,
        );

        let editor = SlintEditor::new(
            window_attributes,
            {
                let params = self.params.clone();
                move |_window, _gui_context| PluginComponent::new(params.clone())
            },
        );

        Some(Box::new(editor))
    }

    fn process(
        &mut self,
        buffer: &mut Buffer<'_>,
        _aux: &mut AuxiliaryBuffers<'_>,
        context: &mut impl ProcessContext<Self>
    ) -> ProcessStatus
    {
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn {
                    timing,
                    voice_id,
                    channel,
                    note,
                    velocity,
                } => self.midi_note.note[note as usize] = true,
                NoteEvent::NoteOff {
                    timing,
                    voice_id,
                    channel,
                    note,
                    velocity,
                } => self.midi_note.note[note as usize] = false,
                _ => (),
            }
        }

        for channel in buffer.as_slice() {
            for sample in channel.iter_mut() {

            }
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for CoPiReMapPlugin {
    const CLAP_ID: &'static str = "com.logiccuteguy.copiremap";
    const CLAP_DESCRIPTION: Option<&'static str> = None;
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Filter,
        ClapFeature::Equalizer,
    ];
}

impl Vst3Plugin for CoPiReMapPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"CoPiReMapPlugins";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx, Vst3SubCategory::Filter, Vst3SubCategory::Eq];
}

nih_export_clap!(CoPiReMapPlugin);
nih_export_vst3!(CoPiReMapPlugin);
