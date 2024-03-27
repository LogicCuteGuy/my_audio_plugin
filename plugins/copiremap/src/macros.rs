use nih_plug::params::{FloatParam, Params};
use nih_plug::prelude::FloatRange;

#[derive(Params)]
pub struct MacroParams {
    #[id = "macro_1"]
    pub macro1: FloatParam,

    #[id = "macro2"]
    pub macro2: FloatParam,

    #[id = "macro3"]
    pub macro3: FloatParam,

    #[id = "macro4"]
    pub macro4: FloatParam,

    #[id = "macro5"]
    pub macro5: FloatParam,

    #[id = "macro6"]
    pub macro6: FloatParam,

    #[id = "macro7"]
    pub macro7: FloatParam,

    #[id = "macro8"]
    pub macro8: FloatParam,

    #[id = "macro9"]
    pub macro9: FloatParam,

    #[id = "macro10"]
    pub macro10: FloatParam,

    #[id = "macro11"]
    pub macro11: FloatParam,

    #[id = "macro12"]
    pub macro12: FloatParam,

    #[id = "macro13"]
    pub macro13: FloatParam,

    #[id = "macro14"]
    pub macro14: FloatParam,

    #[id = "macro15"]
    pub macro15: FloatParam,

}

impl Default for MacroParams {
    fn default() -> Self {
        MacroParams {
            macro1: FloatParam::new("Macro 1", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro2: FloatParam::new("Macro 2", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro3: FloatParam::new("Macro 3", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro4: FloatParam::new("Macro 4", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro5: FloatParam::new("Macro 5", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro6: FloatParam::new("Macro 6", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro7: FloatParam::new("Macro 7", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro8: FloatParam::new("Macro 8", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro9: FloatParam::new("Macro 9", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro10: FloatParam::new("Macro 10", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro11: FloatParam::new("Macro 11", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro12: FloatParam::new("Macro 12", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro13: FloatParam::new("Macro 13", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro14: FloatParam::new("Macro 14", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
            macro15: FloatParam::new("Macro 15", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 }),
        }
    }
}