use nih_plug::params::{BoolParam, IntParam, Params};
use nih_plug::prelude::{IntRange};
use crate::{CoPiReMapPlugin, PluginParams};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nih_plug::audio_setup::BufferConfig;
use crate::audio_process::AudioProcess;
use crate::note_table::NoteTablesArray;

#[derive(Params)]
pub struct KeyNoteParams {
    #[id = "midi"]
    pub midi: BoolParam,

    #[id = "repeat"]
    pub repeat: BoolParam,

    #[id = "find_off_key"]
    pub find_off_key: IntParam,

    #[id = "mute_not_find_off_key"]
    pub mute_not_find_off_key: BoolParam,

    #[id = "round_up"]
    pub round_up: BoolParam,

    #[id = "c"]
    pub c: BoolParam,

    #[id = "c_sharp"]
    pub c_sharp: BoolParam,

    #[id = "d"]
    pub d: BoolParam,

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
            repeat: BoolParam::new("Repeat", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            find_off_key: IntParam::new("Find Off Key", 1, IntRange::Linear{ min: 0, max: 48 })
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            mute_not_find_off_key: BoolParam::new("Mute Not Find Off Key", false)
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
            c: BoolParam::new("C", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            c_sharp: BoolParam::new("C#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            d: BoolParam::new("D", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            d_sharp: BoolParam::new("D#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            e: BoolParam::new("E", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            f: BoolParam::new("F", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            f_sharp: BoolParam::new("F#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            g: BoolParam::new("G", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            g_sharp: BoolParam::new("G#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            a: BoolParam::new("A", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            a_sharp: BoolParam::new("A#", false)
                .with_callback(
                {
                    let update_key_note = update_key_note.clone();
                    Arc::new(move |_| {
                        update_key_note.store(true, Ordering::Release);
                    })
                }
            ),
            b: BoolParam::new("B", false)
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
    pub note: [bool; 84]
}

impl Default for MidiNote {
    fn default() -> Self {
        let mut note = [false; 84];
        for i in 0..84 {
            note[i] = false;
        }
        Self {
            note
        }
    }
}

impl MidiNote {

    pub fn update(&self, params: &Arc<PluginParams>, audio_process: &mut [AudioProcess; 84], buffer_config: &BufferConfig) {
        let mut notes: [i8; 84] = params.note_table.i2t.load().i84;
        match params.key_note.repeat.value() {
            true => {
                let mut note_on_keys = [false; 84];
                let mut notes_sel: [i8; 84] = [-128; 84];
                for i in 0..84 {
                    let a: u8 = i % 12;
                    match a {
                        0 => { note_on_keys[i as usize] = params.key_note.c.value(); }
                        1 => { note_on_keys[i as usize] = params.key_note.c_sharp.value(); }
                        2 => { note_on_keys[i as usize] = params.key_note.d.value(); }
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
                self.find_off_key(params, &note_on_keys, &mut notes_sel);
                notes = notes_sel;
            }
            false => {
            }
        }
        params.note_table.i2t.store(NoteTablesArray { i84: notes});
        AudioProcess::fn_update_pitch_shift_and_after_bandpass(params, audio_process, buffer_config);
    }

    pub fn update_midi(&self, params: &Arc<PluginParams>, audio_process: &mut [AudioProcess; 84], buffer_config: &BufferConfig) {
        let mut notes: [i8; 84] = params.note_table.im2t.load().i84;
        match params.key_note.repeat.value() {
            true => {
                let mut note_on_keys = [false; 84];
                let mut note_keys = [false; 12];
                let mut notes_sel: [i8; 84] = [-128; 84];
                for i in 0..84 {
                    let a: u8 = i % 12;
                    match self.note[i as usize] {
                        true => { note_keys[a as usize] = true; }
                        false => {}
                    }
                }
                for i in 0..84 {
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
                self.find_off_key(params, &note_on_keys, &mut notes_sel);
                notes = notes_sel;
            }
            false => {
                let mut notes_sel: [i8; 84] = [-128; 84];
                self.find_off_key(params, &self.note, &mut notes_sel);
                notes = notes_sel;
            }
        }
        params.note_table.im2t.store(NoteTablesArray { i84: notes});
        AudioProcess::fn_update_pitch_shift_and_after_bandpass(params, audio_process, buffer_config);
    }

    fn find_off_key(&self, params: &Arc<PluginParams>, note_on_keys: &[bool; 84], notes_sel: &mut [i8; 84]) {
        for i in 0..params.key_note.find_off_key.value() {
            for (j, note_on_key) in note_on_keys.iter().enumerate() {
                match note_on_key {
                    true => {
                        match params.key_note.round_up.value() {
                            true => {
                                notes_sel[j] = 0;
                                if j as i8 - i as i8  > -1 && notes_sel[j - i as usize] == 0 {

                                    notes_sel[j - i as usize] = 0;
                                    notes_sel[j - i as usize] = i as i8;
                                }
                                if j + i as usize <= 83 && notes_sel[j + i as usize] == 0 {
                                    notes_sel[j + i as usize] = 0;
                                    notes_sel[j + i as usize] = (i * -1) as i8;
                                }
                            }
                            false => {
                                let ii = 83 - i;
                                notes_sel[j] = 0;
                                if j as i8 - ii as i8 > -1 && notes_sel[j - ii as usize] == 0 {
                                    notes_sel[j - ii as usize] = 0;
                                    notes_sel[j - ii as usize] = ii as i8;
                                }
                                if j + ii as usize <= 83 && notes_sel[j + ii as usize] == 0 {
                                    notes_sel[j + ii as usize] = 0;
                                    notes_sel[j + ii as usize] = (ii * -1) as i8;
                                }
                            }
                        }

                    }
                    false => {

                    }
                }
            }
        }
        match params.key_note.mute_not_find_off_key.value() {
            true => {

            }
            false => {
                for i in 0..84 {
                    if notes_sel[i] == -128 {
                        notes_sel[i] = 0;
                    }
                }
            }
        }
    }

    pub fn param_update(&self, params: &Arc<PluginParams>, audio_process: &mut [AudioProcess; 84], buffer_config: &BufferConfig) {
        match params.key_note.midi.value() {
            true => self.update_midi(params, audio_process, buffer_config),
            false => self.update(params, audio_process, buffer_config),
        }
    }

}