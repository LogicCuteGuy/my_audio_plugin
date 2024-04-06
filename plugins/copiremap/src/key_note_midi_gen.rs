use nih_plug::params::{BoolParam, FloatParam, IntParam, Params};
use nih_plug::prelude::{IntRange};
use crate::audio_process::{AudioProcessNot, AudioProcessParams};
use crate::note_table::NoteTables;
use crate::{PluginParams};
use std::sync::Arc;

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

impl Default for KeyNoteParams {
    fn default() -> Self {
        Self {
            midi: BoolParam::new("Midi", false),
            repeat: BoolParam::new("Repeat", false),
            find_off_key: IntParam::new("Find Off Key", 1, IntRange::Linear{ min: 0, max: 48 }),
            mute_not_find_off_key: BoolParam::new("Mute Not Find Off Key", false),
            round_up: BoolParam::new("Round Up", false),
            c: BoolParam::new("C", false),
            c_sharp: BoolParam::new("C#", false),
            d: BoolParam::new("D", false),
            d_sharp: BoolParam::new("D#", false),
            e: BoolParam::new("E", false),
            f: BoolParam::new("F", false),
            f_sharp: BoolParam::new("F#", false),
            g: BoolParam::new("G", false),
            g_sharp: BoolParam::new("G#", false),
            a: BoolParam::new("A", false),
            a_sharp: BoolParam::new("A#", false),
            b: BoolParam::new("B", false),
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&self, params: &Arc<PluginParams>) {
        let mut notes: [i8; 84] = params.note_table.i2t;
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
        params.note_table.i2t = notes;
    }

    pub fn update_midi(&self, params: &Arc<PluginParams>) {
        let mut notes: [i8; 84] = params.note_table.im2t;
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
        params.note_table.im2t = notes;
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

    pub fn param_update(&self, params: &Arc<PluginParams>) {
        match params.key_note.midi.value() {
            true => self.update_midi(params),
            false => self.update(params),
        }
    }

}