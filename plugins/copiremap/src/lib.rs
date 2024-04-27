mod hertz_calculator;
mod key_note_midi_gen;
mod audio_process;
mod note_table;
mod delay;
mod filter;
mod pitch;
mod gate;

use std::collections::HashMap;
use std::{sync::Arc, num::NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};

use nih_plug::util::db_to_gain;
use nih_plug::{nih_export_clap, nih_export_vst3};
use nih_plug::params::persist::PersistentField;
use nih_plug::prelude::*;
use nih_plug_slint::plugin_component_handle::{PluginComponentHandle, PluginComponentHandleParameterEvents};
use nih_plug_slint::{WindowAttributes, editor::SlintEditor};
use plugin_canvas::drag_drop::DropOperation;
use plugin_canvas::{LogicalSize, Event, LogicalPosition};
use plugin_canvas::event::EventResponse;
use slint::SharedString;
use simple_eq::design::Curve;
use crate::audio_process::{AudioProcess96, AudioProcessParams};
use crate::delay::{Delay, latency_average96};
use crate::filter::MyFilter;
use crate::gate::MyGate;
use crate::hertz_calculator::hz_cal_clh;
use crate::key_note_midi_gen::{KeyNoteParams, MidiNote};
use crate::note_table::NoteTables;

slint::include_modules!();

#[derive(Params)]
pub struct PluginParams {

    #[persist = "note_table"]
    pub note_table: Arc<NoteTables>,

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

    #[id = "dry_gain"]
    pub dry_gain: FloatParam,

    #[id = "lhf_gain"]
    pub lhf_gain: FloatParam,

    #[id = "global_threshold"]
    pub global_threshold: FloatParam,

    #[id = "global_threshold_flip"]
    pub global_threshold_flip: BoolParam,

    #[id = "global_threshold_attack"]
    pub global_threshold_attack: FloatParam,

    #[id = "global_threshold_release"]
    pub global_threshold_release: FloatParam,

    #[id = "low_note_off"]
    pub low_note_off: IntParam,

    #[id = "high_note_off"]
    pub high_note_off: IntParam,

    #[id = "low_note_off_mute"]
    pub low_note_off_mute: BoolParam,

    #[id = "high_note_off_mute"]
    pub high_note_off_mute: BoolParam,

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
                factor: FloatRange::gain_skew_factor(-24.0, 12.0),
            }).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            dry_gain: FloatParam::new("Dry Gain", db_to_gain(-24.0), FloatRange::Skewed {
                min: db_to_gain(-60.0),
                max: db_to_gain(6.0),
                factor: FloatRange::gain_skew_factor(-60.0, 6.0),
            }).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            lhf_gain: FloatParam::new("Low/HighPass Gain", db_to_gain(0.0), FloatRange::Skewed {
                min: db_to_gain(-48.0),
                max: db_to_gain(12.0),
                factor: FloatRange::gain_skew_factor(-48.0, 12.0),
            }).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            global_threshold: FloatParam::new("Global Threshold", db_to_gain(0.0), FloatRange::Linear {
                min: db_to_gain(-100.0),
                max: db_to_gain(0.0),
            }).with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
                .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            global_threshold_flip: BoolParam::new("Global Threshold Flip", true),
            global_threshold_attack: FloatParam::new("Global Threshold Attack", 1.0, FloatRange::Linear {
                min: 0.1,
                max: 5.0,
            }).with_unit("ms.mb"),
            global_threshold_release: FloatParam::new("Global Threshold Release", 1.0, FloatRange::Linear {
                min: 0.1,
                max: 5.0,
            }).with_unit("ms.mb"),
            low_note_off: IntParam::new(
                "Low Note Off",
                36,
                IntRange::Linear {
                    min: 36,
                    max: 131,
                },
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter())
                .with_callback(
                {
                    let update_lowpass = update_lowpass.clone();
                    Arc::new(move |_| {
                        update_lowpass.store(true, Ordering::Release);
                    })
                }
            ),
            high_note_off: IntParam::new(
                "High Note Off",
                131,
                IntRange::Linear {
                    min: 36,
                    max: 131,
                }
            ).with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_string_to_value(formatters::s2v_i32_note_formatter())
                .with_callback(
                    {
                        let update_highpass = update_highpass.clone();
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
            hz_center: FloatParam::new("Hz Center", 440.0, FloatRange::Linear{ min: 415.3046976, max: 466.1637615 })
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
                .with_string_to_value(formatters::s2v_f32_hz_then_khz())
                .with_callback(
                {
                    let update_bpf_center_hz = update_bpf_center_hz.clone();
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
                    let update_pitch_shift_and_after_bandpass = update_pitch_shift_and_after_bandpass.clone();
                    Arc::new(move |_| {
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
            "midi" => self.component.set_gain(parameter),
            _ => (),
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

    fn on_event(&self, _event: &Event) -> EventResponse {
        EventResponse::Ignored
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

pub struct CoPiReMapPlugin {
    params: Arc<PluginParams>,
    buffer_config: BufferConfig,
    midi_note: MidiNote,
    audio_process96: Vec<AudioProcess96>,
    lpf: MyFilter,
    hpf: MyFilter,
    delay: Delay,
    gate: MyGate,

    update_lowpass: Arc<AtomicBool>,
    update_highpass: Arc<AtomicBool>,

    update_pitch_shift_and_after_bandpass: Arc<AtomicBool>,
    update_pitch_shift_over_sampling: Arc<AtomicBool>,
    update_pitch_shift_window_duration_ms: Arc<AtomicBool>,
    update_bpf_center_hz: Arc<AtomicBool>,
    set_pitch_shift_12_node: Arc<AtomicBool>,

    update_key_note: Arc<AtomicBool>,
    update_key_note_12: Arc<AtomicBool>,
}

impl Default for CoPiReMapPlugin {
    fn default() -> Self {
        let update_lowpass = Arc::new(AtomicBool::new(false));
        let update_highpass = Arc::new(AtomicBool::new(false));

        let update_pitch_shift_and_after_bandpass = Arc::new(AtomicBool::new(false));
        let update_pitch_shift_over_sampling = Arc::new(AtomicBool::new(false));
        let update_pitch_shift_window_duration_ms = Arc::new(AtomicBool::new(false));
        let update_bpf_center_hz = Arc::new(AtomicBool::new(false));
        let set_pitch_shift_12_node = Arc::new(AtomicBool::new(false));

        let update_key_note = Arc::new(AtomicBool::new(false));
        let update_key_note_12 = Arc::new(AtomicBool::new(false));

        let mut audio_process96 = Vec::with_capacity(96);
        for _ in 0..96 {
            audio_process96.push(AudioProcess96::default());
        };

        Self {
            params: Arc::new(PluginParams {
                note_table: Arc::new(NoteTables::default()),
                global: Arc::new(GlobalParams::new(update_lowpass.clone(), update_highpass.clone(), update_bpf_center_hz.clone(), update_pitch_shift_and_after_bandpass.clone())),
                audio_process: Arc::new(AudioProcessParams::new(update_pitch_shift_over_sampling.clone(), update_pitch_shift_window_duration_ms.clone(), update_pitch_shift_and_after_bandpass.clone(), update_bpf_center_hz.clone(), set_pitch_shift_12_node.clone())),
                key_note: Arc::new(KeyNoteParams::new(update_key_note.clone(), update_key_note_12.clone())),
            }),
            buffer_config: BufferConfig {
                sample_rate: 1.0,
                min_buffer_size: None,
                max_buffer_size: 0,
                process_mode: ProcessMode::Realtime,
            },
            midi_note: MidiNote::default(),
            audio_process96,
            lpf: MyFilter::default(),
            hpf: MyFilter::default(),
            delay: Delay::default(),
            gate: MyGate::new(),
            update_lowpass,
            update_highpass,
            update_pitch_shift_and_after_bandpass,
            update_pitch_shift_over_sampling,
            update_pitch_shift_window_duration_ms,
            update_bpf_center_hz,
            set_pitch_shift_12_node,
            update_key_note,
            update_key_note_12
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
        let mut lowpass: f32 = 0.0;
        hz_cal_clh((self.params.global.low_note_off.value() - 36) as u8, 0, &mut lowpass, self.params.global.hz_tuning.value(), !self.params.audio_process.pitch_shift.value());
        self.lpf.set(Curve::Lowpass, lowpass, 1.0, 0.0, buffer_config.sample_rate);
        let mut highpass: f32 = 0.0;
        hz_cal_clh((self.params.global.high_note_off.value() - 36) as u8, 0, &mut highpass, self.params.global.hz_tuning.value(), !self.params.audio_process.pitch_shift.value());
        self.hpf.set(Curve::Highpass, highpass, 1.0, 0.0, buffer_config.sample_rate);
        for (i, audio_process) in self.audio_process96.iter_mut().enumerate() {
            audio_process.setup(self.params.clone(), i as u8, &self.buffer_config);
        }
        self.midi_note.param_update(self.params.clone(), &mut self.audio_process96, &self.buffer_config);
        true
    }

    fn reset(&mut self) {
        for ap in self.audio_process96.iter_mut() {
            ap.reset();
        }
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let window_attributes = WindowAttributes::new(
            LogicalSize::new(800.0, 500.0),
            1.0,
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
                let latency = latency_average96(&self.audio_process96);
                if self.delay.get_latency() != latency {
                    self.delay.set_delay(latency);
                    for ap in self.audio_process96.iter_mut() {
                        ap.set_delay(latency)
                    }
                    context.set_latency_samples(latency);
                }
                if self
                    .update_lowpass
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let mut lowpass: f32 = 0.0;
                    let low_note = self.params.global.low_note_off.value() as usize - 36;
                    hz_cal_clh(low_note as u8, 0, &mut lowpass, self.params.global.hz_tuning.value(), !self.params.audio_process.pitch_shift.value());
                    self.lpf.set_frequency(lowpass);
                    self.audio_process96.iter_mut().filter(|ap| ap.note >= low_note as u8 && ap.note < ((low_note + 12) as u8)).for_each(
                        |ap| {
                            ap.set_pitch_shift_12_node(self.params.clone(), &self.buffer_config);
                        }
                    );
                }
                if self
                    .update_highpass
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let mut highpass: f32 = 0.0;
                    hz_cal_clh((self.params.global.high_note_off.value() - 36) as u8, 0, &mut highpass, self.params.global.hz_tuning.value(), !self.params.audio_process.pitch_shift.value());
                    self.hpf.set_frequency(highpass);
                }
                if self
                    .update_pitch_shift_and_after_bandpass
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    let note_table = match self.params.key_note.midi.value() {
                        true => self.params.note_table.im2t.load().i96,
                        false => self.params.note_table.i2t.load().i96,
                    };
                    self.update_bpf_center_hz.set(false);
                    AudioProcess96::fn_update_pitch_shift_and_after_bandpass(self.params.clone(), &mut self.audio_process96, &self.buffer_config, note_table);
                }
                if self
                    .update_bpf_center_hz
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok() {
                    for ap in self.audio_process96.iter_mut() {
                        ap.set_bpf_center_hz(self.params.clone(), &self.buffer_config);
                    }
                }
                if self
                    .update_pitch_shift_over_sampling
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    for ap in self.audio_process96.iter_mut() {
                        ap.set_pitch_shift_over_sampling(self.params.clone());
                    }
                }
                if self
                    .update_pitch_shift_window_duration_ms
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    for ap in self.audio_process96.iter_mut() {
                        ap.set_pitch_shift_window_duration_ms(self.params.clone(), &self.buffer_config);
                    }
                }
                if self
                    .set_pitch_shift_12_node
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    for ap in self.audio_process96.iter_mut() {
                        ap.set_pitch_shift_12_node(self.params.clone(), &self.buffer_config);
                    }
                }
                if self
                    .update_key_note
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    self.midi_note.param_update(self.params.clone(), &mut self.audio_process96, &self.buffer_config);
                }
                if self
                    .update_key_note_12
                    .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
                {
                    self.midi_note.update(self.params.clone(), &mut self.audio_process96, &self.buffer_config);
                }
                while let Some(event) = context.next_event() {
                    match event {
                        NoteEvent::NoteOn {
                            timing: _timing,
                            voice_id: _voice_id,
                            channel: _channel,
                            note,
                            velocity: _velocity,
                        } => if note >= 24 || note <= 119 {
                            self.midi_note.note[note as usize - 12] = true;
                            match self.params.key_note.midi.value() {
                                true => self.midi_note.param_update(self.params.clone(), &mut self.audio_process96, &self.buffer_config),
                                false => {},
                            }
                        },
                        NoteEvent::NoteOff {
                            timing: _timing,
                            voice_id: _voice_id,
                            channel: _channel,
                            note,
                            velocity: _velocity,
                        } => if note >= 24 || note <= 119 {
                            self.midi_note.note[note as usize - 12] = false;
                            match self.params.key_note.midi.value() {
                                true => self.midi_note.param_update(self.params.clone(), &mut self.audio_process96, &self.buffer_config),
                                false => {},
                            }
                        },
                        _ => (),
                    }
                }
                let mut pitch: [f32; 12] = [0.0; 12];
                let mut audio_process: f32 = 0.0;
                for (i, channel) in buffer.as_slice().iter_mut().enumerate() {
                    let size = channel.len();
                    for sample in channel.iter_mut() {
                        let delay = self.delay.process(*sample, i);
                        let gate_on: (bool, bool) = self.gate.update_fast_param(*sample, &self.buffer_config, self.params.global.global_threshold.value(), self.params.global.global_threshold_attack.value(), self.params.global.global_threshold_release.value(), size);
                        let gate1 = (gate_on.0 && !self.params.global.global_threshold_flip.value()) || (gate_on.1 && self.params.global.global_threshold_flip.value());
                        if gate1 {
                            let lpf_mute = match self.params.global.low_note_off_mute.value() { true => 0.0, false => self.lpf.process(delay, i) };
                            let hpf_mute = match self.params.global.high_note_off_mute.value() { true => 0.0, false => self.hpf.process(delay, i) };
                            match self.params.audio_process.pitch_shift_12_node.value() {
                                true => {
                                    let low_note = self.params.global.low_note_off.value() as usize - 36;
                                    let mut index = low_note % 12;
                                    self.audio_process96.iter_mut().filter(|ap| ap.note >= low_note as u8 && ap.note <= (self.params.global.high_note_off.value() as usize - 36) as u8).for_each(
                                        |ap| {
                                            if index >= 12 {
                                                index = 0;
                                            }
                                            let input_param: f32 = if ap.note_pitch == 0 { self.params.audio_process.in_key_gain.value() } else if ap.note_pitch == -128 { self.params.audio_process.off_key_gain.value() } else if !self.params.audio_process.pitch_shift.value() { self.params.audio_process.off_key_gain.value() } else { self.params.audio_process.tuning_gain.value() };
                                            if ap.tuning.is_some() {
                                                pitch[index] = ap.process(*sample, self.params.clone(), i, input_param, &self.buffer_config, size);
                                            }
                                            if input_param > db_to_gain(-60.0) {
                                                audio_process += ap.process_bpf(pitch[index], i, input_param, self.params.clone());
                                                // println!("Work {}, {}", ii, ap.note);
                                            }
                                            index += 1;
                                        }
                                    );
                                }
                                false => {
                                    self.audio_process96.iter_mut().filter(|ap| ap.note >= (self.params.global.low_note_off.value() as usize - 36) as u8 && ap.note <= (self.params.global.high_note_off.value() as usize - 36) as u8).for_each(
                                        |ap| {
                                            let input_param: f32 = if ap.note_pitch == 0 { self.params.audio_process.in_key_gain.value() } else if ap.note_pitch == -128 { self.params.audio_process.off_key_gain.value() } else if !self.params.audio_process.pitch_shift.value() { self.params.audio_process.off_key_gain.value() } else { self.params.audio_process.tuning_gain.value() };
                                            audio_process += ap.process(*sample, self.params.clone(), i, input_param, &self.buffer_config, size);
                                        }
                                    );
                                }
                            }
                            *sample = ((audio_process * self.params.global.wet_gain.value()) + (delay * self.params.global.dry_gain.value()) + ((lpf_mute + hpf_mute) * self.params.global.lhf_gain.value())) * if self.params.global.global_threshold_flip.value() {self.gate.get_param_inv()} else {self.gate.get_param()};
                            audio_process = 0.0;
                        }
                        let gate2 = (gate_on.1 && !self.params.global.global_threshold_flip.value()) || (gate_on.0 && self.params.global.global_threshold_flip.value());
                        if gate2 {
                            *sample = (delay * if !self.params.global.global_threshold_flip.value() {self.gate.get_param_inv()} else {self.gate.get_param()}) + if gate1 { *sample } else { 0.0 };
                        }
                    }
                }
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
