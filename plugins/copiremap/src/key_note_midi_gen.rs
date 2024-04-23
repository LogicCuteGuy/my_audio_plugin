use nih_plug::params::{BoolParam, IntParam, Params};
use nih_plug::prelude::{IntRange};
use crate::{PluginParams};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nih_plug::audio_setup::BufferConfig;
use crate::audio_process::AudioProcess96;
use crate::note_table::NoteTablesArray;

#[derive(Params)]
pub struct KeyNoteParams {
    #[id = "midi"]
    pub midi: BoolParam,

    #[id = "repeat"]
    pub repeat: BoolParam,

    #[id = "find_off_key"]
    pub find_off_key: IntParam,

    #[id = "round_up"]
    pub round_up: BoolParam,

    #[id = "note_c"]
    pub note_c: BoolParam,

    #[id = "note_c_sharp"]
    pub note_c_sharp: BoolParam,

    #[id = "note_d"]
    pub note_d: BoolParam,

    #[id = "d_sharp"]
    pub d_sharp: BoolParam,

    #[id = "e"]
    pub e: BoolParam,

    #[id = "f"]
    pub f: BoolParam,

    #[id = "f_sharp"]
    pub f_sharp: BoolParam,

    #[id = "g"]
    pub g: BoolParam,

    #[id = "g_sharp"]
    pub g_sharp: BoolParam,

    #[id = "a"]
    pub a: BoolParam,

    #[id = "a_sharp"]
    pub a_sharp: BoolParam,

    #[id = "b"]
    pub b: BoolParam,
}

impl KeyNoteParams {
    pub fn new(update_key_note: Arc<AtomicBool>) -> Self {

        Self {
            midi: BoolParam::new("Midi", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            repeat: BoolParam::new("Repeat", true)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            find_off_key: IntParam::new("Find Off Key", 4, IntRange::Linear{ min: 0, max: 72 })
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            round_up: BoolParam::new("Round Up", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            note_c: BoolParam::new("Note C", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            note_c_sharp: BoolParam::new("Note C#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            note_d: BoolParam::new("Note D", true)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            d_sharp: BoolParam::new("Note D#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            e: BoolParam::new("Note E", true)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            f: BoolParam::new("Note F", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            f_sharp: BoolParam::new("Note F#", true)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            g: BoolParam::new("Note G", true)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            g_sharp: BoolParam::new("Note G#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            a: BoolParam::new("Note A", true)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            a_sharp: BoolParam::new("Note A#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            b: BoolParam::new("Note B", true)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
        }
    }
}

pub struct MidiNote {
    pub note: [bool; 96]
}

impl Default for MidiNote {
    fn default() -> Self {
        let mut note = [false; 96];
        for i in 0..96 {
            note[i] = false;
        }
        Self {
            note
        }
    }
}

impl MidiNote {

    pub fn update(&self, params: Arc<PluginParams>, audio_process: &mut Vec<AudioProcess96>, buffer_config: &BufferConfig) {
        let mut notes: [i8; 96] = params.note_table.i2t.load().i96;
        match params.key_note.repeat.value() {
            true => {
                let mut note_on_keys = [false; 96];
                let mut notes_sel: [i8; 96] = [-128; 96];
                for i in 0..96 {
                    let a: u8 = i % 12;
                    match a {
                        0 => { note_on_keys[i as usize] = params.key_note.note_c.value(); }
                        1 => { note_on_keys[i as usize] = params.key_note.note_c_sharp.value(); }
                        2 => { note_on_keys[i as usize] = params.key_note.note_d.value(); }
                        3 => { note_on_keys[i as usize] = params.key_note.d_sharp.value(); }
                        4 => { note_on_keys[i as usize] = params.key_note.e.value(); }
                        5 => { note_on_keys[i as usize] = params.key_note.f.value(); }
                        6 => { note_on_keys[i as usize] = params.key_note.f_sharp.value(); }
                        7 => { note_on_keys[i as usize] = params.key_note.g.value(); }
                        8 => { note_on_keys[i as usize] = params.key_note.g_sharp.value(); }
                        9 => { note_on_keys[i as usize] = params.key_note.a.value(); }
                        10 => { note_on_keys[i as usize] = params.key_note.a_sharp.value(); }
                        11 => { note_on_keys[i as usize] = params.key_note.b.value(); }
                        _ => {}
                    }
                }
                self.find_off_key(params.clone(), &note_on_keys, &mut notes_sel);
                notes = notes_sel;
            }
            false => {
            }
        }
        params.note_table.i2t.store(NoteTablesArray { i96: notes});
        AudioProcess96::fn_update_pitch_shift_and_after_bandpass(params, audio_process, buffer_config, notes);
    }

    pub fn update_midi(&self, params: Arc<PluginParams>, audio_process: &mut Vec<AudioProcess96>, buffer_config: &BufferConfig) {
        let mut notes: [i8; 96] = params.note_table.im2t.load().i96;
        match params.key_note.repeat.value() {
            true => {
                let mut note_on_keys = [false; 96];
                let mut note_keys = [false; 12];
                let mut notes_sel: [i8; 96] = [-128; 96];
                for i in 0..96 {
                    let a: u8 = i % 12;
                    match self.note[i as usize] {
                        true => { note_keys[a as usize] = true; }
                        false => {}
                    }
                }
                for i in 0..96 {
                    let a: u8 = i % 12;
                    match a {
                        0 => { note_on_keys[i as usize] = note_keys[0]; }
                        1 => { note_on_keys[i as usize] = note_keys[1]; }
                        2 => { note_on_keys[i as usize] = note_keys[2]; }
                        3 => { note_on_keys[i as usize] = note_keys[3]; }
                        4 => { note_on_keys[i as usize] = note_keys[4]; }
                        5 => { note_on_keys[i as usize] = note_keys[5]; }
                        6 => { note_on_keys[i as usize] = note_keys[6]; }
                        7 => { note_on_keys[i as usize] = note_keys[7]; }
                        8 => { note_on_keys[i as usize] = note_keys[8]; }
                        9 => { note_on_keys[i as usize] = note_keys[9]; }
                        10 => { note_on_keys[i as usize] = note_keys[10]; }
                        11 => { note_on_keys[i as usize] = note_keys[11]; }
                        _ => {}
                    }
                }
                self.find_off_key(params.clone(), &note_on_keys, &mut notes_sel);
                notes = notes_sel;
            }
            false => {
                let mut notes_sel: [i8; 96] = [-128; 96];
                self.find_off_key(params.clone(), &self.note, &mut notes_sel);
                notes = notes_sel;
            }
        }
        params.note_table.im2t.store(NoteTablesArray { i96: notes});
        AudioProcess96::fn_update_pitch_shift_and_after_bandpass(params, audio_process, buffer_config, notes);
    }

    fn find_off_key(&self, params: Arc<PluginParams>, note_on_keys: &[bool; 96], notes_sel: &mut [i8; 96]) {
        for i in 0..params.key_note.find_off_key.value() {
            match params.key_note.round_up.value() {
                true => {
                    for (j, note_on_key) in note_on_keys.iter().enumerate() {
                        match note_on_key {
                            true => {
                                notes_sel[j] = 0;
                                if j as i8 - i as i8 > -1 && notes_sel[j - i as usize] == -128 {
                                    notes_sel[j - i as usize] = 0;
                                    notes_sel[j - i as usize] = i as i8;
                                }
                                if j + i as usize <= 95 && notes_sel[j + i as usize] == -128 {
                                    notes_sel[j + i as usize] = 0;
                                    notes_sel[j + i as usize] = (i * -1) as i8;
                                }
                            }
                            false => {

                            }
                        }
                    }
                }
                false => {
                    for (j, note_on_key) in note_on_keys.iter().enumerate().rev() {
                        match note_on_key {
                            true => {
                                notes_sel[j] = 0;
                                if j as i8 - i as i8 > -1 && notes_sel[j - i as usize] == -128 {
                                    notes_sel[j - i as usize] = 0;
                                    notes_sel[j - i as usize] = i as i8;
                                }
                                if j + i as usize <= 95 && notes_sel[j + i as usize] == -128 {
                                    notes_sel[j + i as usize] = 0;
                                    notes_sel[j + i as usize] = (i * -1) as i8;
                                }
                            }
                            false => {

                            }
                        }
                    }
                }
            }
        }
    }

    pub fn param_update(&self, params: Arc<PluginParams>, audio_process: &mut Vec<AudioProcess96>, buffer_config: &BufferConfig) {
        match params.key_note.midi.value() {
            true => self.update_midi(params, audio_process, buffer_config),
            false => self.update(params, audio_process, buffer_config),
        }
    }

}