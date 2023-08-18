use nih_plug::prelude::{util, Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use crate::LfoCoolParams;

const STYLE: &str = r#"
.main {
    background-color: #171717;
}

.param_knob {
    width: 150px;
}

label {
    child-space: 1s;
    font-size: 20;
    color: #d4d4d4;
}

knob {
    width: 100px;
    height: 100px;
}

knob .track {
    background-color: #6366f1;
}

.tick {
    background-color: #737373;
}

"#;

use self::param_knob::ParamKnob;
mod param_knob;

#[derive(Lens)]
struct Data {
    params: Arc<LfoCoolParams>,
}

impl Model for Data {}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (600, 400))
}

pub(crate) fn create(
    params: Arc<LfoCoolParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        cx.add_theme(STYLE);

        Data {
            params: params.clone(),
        }
            .build(cx);

        HStack::new(cx, |cx| {
            // Label::new(cx, "LfoCool")
            //     .font_family(vec![FamilyOwned::Name(String::from(
            //         assets::NOTO_SANS_THIN,
            //     ))])
            //     .font_size(30.0)
            //     .height(Pixels(50.0))
            //     .child_top(Stretch(1.0))
            //     .child_bottom(Pixels(0.0));

            ParamKnob::new(cx, Data::params, |params| &params.plug_params.frequency, false);
            // Label::new(cx, "Gain Modulation");
            ParamKnob::new(cx, Data::params, |params| &params.plug_params.gain_mod, false);
        })
            .row_between(Pixels(0.0))
            .child_left(Stretch(1.0))
            .child_right(Stretch(1.0))
            .class("main");
    })
}