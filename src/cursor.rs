use cairo;
use ui_model::Color;
use ui::{SH, UiMutex};
use shell::{Shell, NvimMode};
use nvim::{RepaintMode, RedrawEvents};
use std::sync::Arc;

use glib;

struct Alpha(f64);

impl Alpha {
    pub fn show(&mut self, step: f64) -> bool {
        self.0 += step;
        if self.0 > 1.0 {
            self.0 = 1.0;
            false
        } else {
            true
        }
    }
    pub fn hide(&mut self, step: f64) -> bool {
        self.0 -= step;
        if self.0 < 0.0 {
            self.0 = 0.0;
            false
        } else {
            true
        }
    }
}

#[derive(PartialEq)]
enum AnimPhase {
    Shown,
    Hide,
    Hidden,
    Show,
    NoFocus,
}

struct State {
    alpha: Alpha,
    anim_phase: AnimPhase,

    timer: Option<glib::SourceId>,
}

impl State {
    fn new() -> State {
        State {
            alpha: Alpha(1.0),
            anim_phase: AnimPhase::Shown,
            timer: None,
        }
    }

    fn reset_to(&mut self, phase: AnimPhase) {
        self.alpha = Alpha(1.0);
        self.anim_phase = phase;
        if let Some(timer_id) = self.timer {
            glib::source_remove(timer_id);
            self.timer = None;
        }
    }
}

pub struct Cursor {
    state: Arc<UiMutex<State>>,
}

impl Cursor {
    pub fn new() -> Cursor {
        Cursor { state: Arc::new(UiMutex::new(State::new())) }
    }

    pub fn start(&mut self) {
        let state = self.state.clone();
        let mut mut_state = self.state.borrow_mut();
        mut_state.reset_to(AnimPhase::Shown);
        mut_state.timer = Some(glib::timeout_add(500, move || anim_step(&state)));
    }

    pub fn reset_state(&mut self) {
        self.start();
    }
    
    pub fn enter_focus(&mut self) {
        self.start();
    }

    pub fn leave_focus(&mut self) {
        self.state.borrow_mut().reset_to(AnimPhase::NoFocus);
    }

    pub fn draw(&self,
                ctx: &cairo::Context,
                shell: &Shell,
                char_width: f64,
                line_height: f64,
                line_y: f64,
                double_width: bool,
                bg: &Color) {

        let current_point = ctx.get_current_point();
        let state = self.state.borrow();
        ctx.set_source_rgba(1.0 - bg.0, 1.0 - bg.1, 1.0 - bg.2, 0.6 * state.alpha.0);

        let cursor_width = if shell.mode == NvimMode::Insert {
            char_width / 5.0
        } else {
            if double_width {
                char_width * 2.0
            } else {
                char_width
            }
        };

        ctx.rectangle(current_point.0, line_y, cursor_width, line_height);
        if state.anim_phase == AnimPhase::NoFocus {
            ctx.stroke();
        } else {
            ctx.fill();
        }
    }
}

fn anim_step(state: &Arc<UiMutex<State>>) -> glib::Continue {
    let moved_state = state.clone();
    let mut mut_state = state.borrow_mut();

    let next_event = match mut_state.anim_phase {
        AnimPhase::Shown => {
            mut_state.anim_phase = AnimPhase::Hide;
            Some(60)
        }
        AnimPhase::Hide => {
            if !mut_state.alpha.hide(0.3) {
                mut_state.anim_phase = AnimPhase::Hidden;

                Some(300)
            } else {
                None
            }
        }
        AnimPhase::Hidden => {
            mut_state.anim_phase = AnimPhase::Show;

            Some(60)
        }
        AnimPhase::Show => {
            if !mut_state.alpha.show(0.3) {
                mut_state.anim_phase = AnimPhase::Shown;

                Some(500)
            } else {
                None
            }
        }
        AnimPhase::NoFocus => None, 
    };

    SHELL!(&shell = {
        let point = shell.model.cur_point();
        shell.on_redraw(&RepaintMode::Area(point));
    });


    if let Some(timeout) = next_event {
        mut_state.timer = Some(glib::timeout_add(timeout, move || anim_step(&moved_state)));

        glib::Continue(false)
    } else {
        glib::Continue(true)
    }

}

impl Drop for Cursor {
    fn drop(&mut self) {
        if let Some(timer_id) = self.state.borrow().timer {
            glib::source_remove(timer_id);
        }
    }
}