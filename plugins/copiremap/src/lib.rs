mod hertz_calculator;
mod key_note_midi_gen;
mod audio_process;
mod note_table;
mod delay;
mod filter;
mod pitch;

use std::collections::HashMap;
use std::{sync::Arc, num::NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};

use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3};
use nih_plug::prelude::*;
use nih_plug_slint::plugin_component_handle::{PluginComponentHandle, PluginComponentHandleParameterEvents};
use nih_plug_slint::{WindowAttributes, editor::SlintEditor};
use plugin_canvas::drag_drop::DropOperation;
use plugin_canvas::{LogicalSize, Event, LogicalPosition};
use plugin_canvas::event::EventResponse;
use slint::SharedString;
use iir_filters::filter_design::FilterType;
use crate::audio_process::{AudioProcess, AudioProcessNot, AudioProcessParams};
use crate::delay::{Delay, latency_average};
use crate::filter::MyFilter;
use crate::hertz_calculator::{hz_cal_clh};
use crate::key_note_midi_gen::{KeyNoteParams, MidiNote};
use crate::note_table::NoteTables;

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

    #[id = "order"]
    pub order: IntParam,

    #[id = "hz_center"]
    pub hz_center: FloatParam,

    #[id = "hz_tuning"]
    pub hz_tuning: FloatParam,
}

impl GlobalParams {
    fn new(update_lowpass: Arc<AtomicBool>, update_highpass: Arc<AtomicBool>, update_bpf_center_hz: Arc<AtomicBool>, update_pitch_shift_and_after_bandpass: Arc<AtomicBool>) -> Self {
        Self {
            bypass: BoolParam::new("Bypass", false)
                .with_value_to_string(formatters::v2s_bool_bypass())
                .with_string_to_value(formatters::s2v_bool_bypass())
                .make_bypass(),
            wet_gain: FloatParam::new("Wet Gain", db_to_gain(0.0), FloatRange::Skewed {
                min: db_to_gain(-24.0),
                max: db_to_gain(12.0),
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
                24,
                IntRange::Linear {
                    min: 24,
                    max: 107,
                },
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter())
                .with_callback(
                {
                    Arc::new(move |_| {
                        update_lowpass.store(true, Ordering::Release);
                    })
                }
            ),
            high_note_off: IntParam::new(
                "High Note Off",
                107,
                IntRange::Linear {
                    min: 24,
                    max: 107,
                }
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter())
                .with_callback(
                    {
                        Arc::new(move |_| {
                            update_highpass.store(true, Ordering::Release);
                        })
                    }
                ),
            low_note_off_mute: BoolParam::new(
                "Low Note Off Mute",
                false,
            ),
            high_note_off_mute: BoolParam::new(
                "High Note Off Mute",
                false,
            ),
            order: IntParam::new(
                "Order",
                5,
                IntRange::Linear {
                    min: 0,
                    max: 15,
                },
            ).with_callback(
                {
                    Arc::new(move |_| {
                        update_lowpass.store(true, Ordering::Release);
                        update_highpass.store(true, Ordering::Release);
                        update_bpf_center_hz.store(true, Ordering::Release);
                    })
                }
            ),
            hz_center: FloatParam::new("Hz Center", 440.0, FloatRange::Linear{ min: 415.3046976, max: 466.1637615 })
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
                .with_string_to_value(formatters::s2v_f32_hz_then_khz())
                .with_callback(
                {
                    Arc::new(move |_| {
                        update_bpf_center_hz.store(true, Ordering::Release);
                    })
                }
            ),
            hz_tuning: FloatParam::new("Hz Tuning", 440.0, FloatRange::Linear{ min: 415.3046976, max: 466.1637615 })
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
                .with_string_to_value(formatters::s2v_f32_hz_then_khz())
                .with_callback(
                {
                    Arc::new(move |_| {
                        update_lowpass.store(true, Ordering::Release);
                        update_highpass.store(true, Ordering::Release);
                        update_pitch_shift_and_after_bandpass.store(true, Ordering::Release);
                    })
                }
            )
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
    audio_process: [AudioProcess; 84],
    lpf: MyFilter,
    hpf: MyFilter,
    delay: Delay,

    update_lowpass: Arc<AtomicBool>,
    update_highpass: Arc<AtomicBool>,

    update_pitch_shift_and_after_bandpass: Arc<AtomicBool>,
    update_pitch_shift_over_sampling: Arc<AtomicBool>,
    update_bpf_center_hz: Arc<AtomicBool>,
}

impl Default for CoPiReMapPlugin {
    fn default() -> Self {
        let update_lowpass = Arc::new(AtomicBool::new(false));
        let update_highpass = Arc::new(AtomicBool::new(false));

        let update_pitch_shift_and_after_bandpass = Arc::new(AtomicBool::new(false));
        let update_pitch_shift_over_sampling = Arc::new(AtomicBool::new(false));
        let update_bpf_center_hz = Arc::new(AtomicBool::new(false));

        Self {
            params: Arc::new(PluginParams {
                note_table: Arc::new(NoteTables::default()),
                audio_process_not: Arc::new(AudioProcessNot { pitch_shift_window_duration_ms: 2 }),
                global: Arc::new(GlobalParams::new(update_lowpass.clone(), update_highpass.clone(), update_bpf_center_hz.clone(), update_pitch_shift_and_after_bandpass.clone())),
                audio_process: Arc::new(AudioProcessParams::new(update_pitch_shift_and_after_bandpass.clone(), update_pitch_shift_over_sampling.clone())),
                key_note: Arc::new(Default::default()),
            }),
            buffer_config: BufferConfig {
                sample_rate: 1.0,
                min_buffer_size: None,
                max_buffer_size: 0,
                process_mode: ProcessMode::Realtime,
            },
            midi_note: MidiNote::default(),
            audio_process: [AudioProcess::default(); 84],
            lpf: MyFilter::default(),
            hpf: MyFilter::default(),
            delay: Delay::default(),
            update_lowpass,
            update_highpass,
            update_pitch_shift_and_after_bandpass,
            update_pitch_shift_over_sampling,
            update_bpf_center_hz
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
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),

            ..AudioIOLayout::const_default()
        }
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

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
        for (i, audio_process) in self.audio_process.iter_mut().enumerate() {
            audio_process.setup(&self, (i + 24) as u8);
        }
        true
    }

    fn reset(&mut self) {
        for mut ap in self.audio_process {
            ap.reset();
        }
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
        match self.params.global.bypass.value() {
            true => {
                if self.delay.get_latency() != 0 {
                    self.delay.set_delay(0);
                    context.set_latency_samples(0);
                }
            }
            false => {
                let latency = latency_average(&self.audio_process);
                if self.delay.get_latency() != latency {
                    self.delay.set_delay(latency);
                    for ap in self.audio_process.iter_mut() {
                        ap.set_delay(latency)
                    }
                    context.set_latency_samples(latency);
                }
                if self
                    .update_lowpass
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    self.update_lpf()
                }
                if self
                    .update_lowpass
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    self.update_hpf()
                }
                while let Some(event) = context.next_event() {
                    match event {
                        NoteEvent::NoteOn {
                            timing,
                            voice_id,
                            channel,
                            note,
                            velocity,
                        } => if note >= 24 || note < 108 { self.midi_note.note[note as usize - 24] = true },
                        NoteEvent::NoteOff {
                            timing,
                            voice_id,
                            channel,
                            note,
                            velocity,
                        } => if note >= 24 || note < 108 { self.midi_note.note[note as usize - 24] = false },
                        _ => (),
                    }
                }
                for channel in buffer.as_slice() {
                    let mut audio: [f32; 2] = [*channel.get(0).unwrap(), *channel.get(1).unwrap()];
                    let delay = self.delay.process(audio);
                    let lpf = self.lpf.process(delay);
                    let hpf = self.hpf.process(delay);
                    let lpf_mute = match self.params.global.low_note_off_mute.value() { true => [0.0, 0.0], false => lpf };
                    let hpf_mute = match self.params.global.high_note_off_mute.value() { true => [0.0, 0.0], false => hpf };
                    if audio[0] >= self.params.global.global_threshold.value() || audio[1] >= self.params.global.global_threshold.value() {
                        let mut audio_process: [f32; 2] = [0.0; 2];
                        for (i, ap) in self.audio_process.iter_mut().enumerate() {
                            if i >= self.params.global.low_note_off.value() as usize - 24 && i <= self.params.global.high_note_off.value() as usize - 24 {
                                let af = ap.process(audio, &self);
                                audio_process[0] *= af[0];
                                audio_process[1] *= af[1];
                            }
                        }
                        audio_process[0] *= self.params.global.wet_gain.value();
                        audio_process[1] *= self.params.global.wet_gain.value();
                        audio[0] = audio_process[0] + lpf_mute[0] + hpf_mute[0];
                        audio[1] = audio_process[1] + lpf_mute[1] + hpf_mute[1];
                    } else {
                        audio[0] = delay[0] + lpf_mute[0] + hpf_mute[0];
                        audio[1] = delay[1] + lpf_mute[1] + hpf_mute[1];
                    }
                    *channel.get_mut(0).unwrap() = audio[0];
                    *channel.get_mut(1).unwrap() = audio[1];
                }
            }
        }
        ProcessStatus::Normal
    }
}

impl CoPiReMapPlugin {

    pub fn update_lpf(&mut self) {
        let mut lowpass: f32 = 0.0;
        hz_cal_clh(self.params.global.low_note_off.value() as u8, &mut 0.0, &mut lowpass, &mut 0.0, self.params.global.hz_tuning.value());
        self.lpf.set_filter(self.params.global.order.value() as u8, FilterType::LowPass(lowpass), self.buffer_config.sample_rate)
    }

    pub fn update_hpf(&mut self) {
        let mut highpass: f32 = 0.0;
        hz_cal_clh(self.params.global.low_note_off.value() as u8, &mut 0.0, &mut 0.0, &mut highpass, self.params.global.hz_tuning.value());
        self.hpf.set_filter(self.params.global.order.value() as u8, FilterType::HighPass(highpass), self.buffer_config.sample_rate)
    }

    pub fn update_psaabp(&mut self) {

    }

    pub fn update_psos(&mut self) {

    }

    pub fn update_bpfchz(&mut self) {

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
