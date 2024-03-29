use nih_plug::params::{BoolParam, FloatParam, IntParam, Params};

#[derive(Params)]
pub struct KeyNoteParams {
    #[id = "midi"]
    pub midi: BoolParam,

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
