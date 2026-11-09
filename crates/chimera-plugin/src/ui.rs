use super::*;
use chimera::anatomy::{
    geom::{Vec2, catmull_rom, v},
    human::{HumanArtic, HumanFrame, HumanVocalTract, NEUTRAL},
};
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, Pos2, Stroke, Vec2 as EVec},
    widgets,
};
const GREEN: Color32 = Color32::from_rgb(112, 245, 182);
const BLUE: Color32 = Color32::from_rgb(89, 159, 217);
const PINK: Color32 = Color32::from_rgb(248, 144, 190);
const AMBER: Color32 = Color32::from_rgb(255, 199, 114);

struct UiState {
    draft: String,
    local: bool,
    octave: i32,
    models: [HumanVocalTract; 3],
    frames: [HumanFrame; 3],
    cached_artic: [Option<HumanArtic>; 3],
    wire: bool,
    tab: usize,
    drag: Option<(usize, f32, f32, EVec)>,
    vowels: [ArticPatch; 5],
    preset_name: String,
    preset_names: Vec<String>,
    status: String,
    undo: Vec<presets::Snapshot>,
    redo: Vec<presets::Snapshot>,
    before: Option<presets::Snapshot>,
    lock_pitch: bool,
    lock_lyrics: bool,
    random: u64,
    selected_syllable: usize,
}
impl UiState {
    fn new(params: &Controls) -> Self {
        Self {
            draft: params.lyrics.lock().unwrap().clone(),
            preset_name: params.lab.session.lock().unwrap().preset_name.clone(),
            preset_names: presets::list(),
            status: String::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            before: None,
            lock_pitch: true,
            lock_lyrics: true,
            random: 0xC01DFACE,
            selected_syllable: 0,
            local: false,
            octave: 4,
            models: [BodyMix::MALE, BodyMix::FEMALE, BodyMix::CHILD]
                .map(|b| HumanVocalTract::new(b.body())),
            frames: std::array::from_fn(|_| HumanFrame::with_capacity()),
            cached_artic: [None; 3],
            wire: true,
            tab: 0,
            drag: None,
            vowels: ["ɑ", "ɛ", "i", "o", "u"].map(|v| vowel_artic(v, BodyMix::MALE).unwrap()),
        }
    }
}

pub(super) fn editor(
    p: Arc<Controls>,
    shared: Arc<Shared>,
    panic: Arc<AtomicBool>,
    executor: AsyncExecutor<ChimeraPlugin>,
) -> Option<Box<dyn Editor>> {
    let state = UiState::new(&p);
    let close_state = shared.clone();
    let inner = create_egui_editor(
        p.editor.clone(),
        state,
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.panel_fill = Color32::from_rgb(4, 9, 13);
            style.visuals.window_fill = Color32::from_rgb(7, 14, 19);
            style.visuals.override_text_color = Some(Color32::from_rgb(202, 224, 229));
            style.visuals.selection.bg_fill = Color32::from_rgb(22, 80, 67);
            style.visuals.widgets.active.bg_fill = Color32::from_rgb(27, 102, 80);
            style.spacing.item_spacing = EVec::new(10.0, 7.0);
            style.spacing.slider_width = 240.0;
            ctx.set_style(style);
        },
        move |ctx, setter, state| {
            ctx.set_zoom_factor(p.lab.ui_scale.value());
            if ctx.input(|i| i.pointer.any_pressed()) {
                state.before = Some(presets::Snapshot::capture(&p, "Undo"));
            }
            nih_plug_egui::resizable_window::ResizableWindow::new("lab-window").min_size(EVec::new(940.,640.)).show(ctx,&p.editor, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false,false]).show(ui,|ui| {
                    ui.set_min_width(900.);
                    ui.horizontal(|ui| {
                        ui.heading(egui::RichText::new("CHIMERA").size(25.).color(GREEN));
                        ui.label(egui::RichText::new("LAB 04").monospace().color(BLUE));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                            if ui.button("Stop sound").on_hover_text("Panic: release notes, mouth percussion and preview").clicked(){panic.store(true,Ordering::Release);}
                            let cpu=f32::from_bits(shared.cpu_load.load(Ordering::Relaxed));
                            ui.label(format!("Audio {:.1}%  ·  {} voices",cpu,shared.voices.load(Ordering::Relaxed)))
                                .on_hover_text("Smoothed audio callback time / available buffer time. This is not total system CPU and excludes drawing the editor.");
                        });
                    });
                    preset_bar(ui,setter,&p,&shared,state,&executor);
                    ui.add_space(6.);
                    ui.horizontal(|ui|{
                        ui.label("Play mode");
                        for (mode,label,help) in [(Mode::Vowels,"Vowels","Play the mouth like a synthesizer"),(Mode::Loop,"Words on the beat","Lyrics follow Live's transport; polyphonic mode needs held MIDI notes"),(Mode::Syllables,"Words from keys","Each note or simultaneous chord starts the next syllable")]{
                            if ui.selectable_label(p.mode.value()==mode,label).on_hover_text(help).clicked(){set(setter,&p.mode,mode);}
                        }
                        ui.separator();
                        let mut poly=p.lab.poly.value();
                        if ui.toggle_value(&mut poly,"Poly choir").on_hover_text("Independent notes and releases. Switch modes between phrases; held notes are cleared.").changed(){set(setter,&p.lab.poly,poly);}
                        if poly { voice_limit(ui,setter,&p); }
                    });
                    if p.mode.value()!=Mode::Vowels {
                        ui.add_space(4.);
                        ui.horizontal(|ui|{
                            let field=ui.add(egui::TextEdit::singleline(&mut state.draft).desired_width(540.).hint_text("Type English words").char_limit(240));
                            if ui.button("Apply words").clicked() || (field.has_focus() && ui.input(|i|i.key_pressed(egui::Key::Enter))){
                                *p.lyrics.lock().unwrap()=state.draft.clone();executor.execute_background(Task::Compile);field.surrender_focus();
                            }
                            if ui.button("Rewind").clicked(){shared.rewind.store(true,Ordering::Release);}
                            if p.mode.value()==Mode::Loop && !p.lab.poly.value(){
                                let preview=shared.preview.load(Ordering::Relaxed);
                                if ui.button(if preview {"Stop preview"}else{"Preview"}).clicked(){shared.preview.store(!preview,Ordering::Release);shared.rewind.store(true,Ordering::Release);}
                            }
                        });
                        ui.horizontal_wrapped(|ui|{
                            if let Ok(bank)=shared.bank.try_lock(){
                                let active=shared.active_syllable.load(Ordering::Relaxed);
                                for i in 0..bank.len {ui.label(egui::RichText::new(bank.labels[i].as_str()).color(if i==active{GREEN}else{Color32::GRAY}));}
                            }
                        });
                    }
                    if let Ok(error)=shared.error.try_lock(){if !error.is_empty(){ui.colored_label(PINK,error.as_str());}}
                    ui.separator();
                    ui.horizontal(|ui|{
                        for (i,label) in ["Play","Mouth","Textures","Motion","Words","Choir"].iter().enumerate(){
                            if ui.selectable_label(state.tab==i,*label).clicked(){state.tab=i;}
                        }
                    });
                    ui.add_space(8.);
                    ui.horizontal_top(|ui|{
                        ui.vertical(|ui|{
                            ui.set_width(335.);
                            ui.horizontal(|ui|{ui.label(egui::RichText::new("SHAPE THE VOICE").color(BLUE));ui.checkbox(&mut state.wire,"Wireframe");});
                            anatomy(ui,setter,&p,&shared,state,if p.lab.poly.value(){0}else{p.lab.voice.value().round()as usize},335.);
                            ui.small("Drag the colored handles to move the tongue, lips and throat.");
                            ui.horizontal(|ui|{
                                for (i,label) in ["AH","EH","EE","OH","OO"].iter().enumerate(){if ui.selectable_label((p.vowel.value()-i as f32).abs()<0.01,*label).clicked(){set(setter,&p.vowel,i as f32);}}
                                if ui.small_button("Reset mouth").clicked(){for param in [&p.tongue_x,&p.tongue_y,&p.tip_x,&p.tip_y,&p.jaw,&p.lips,&p.lip_prot,&p.larynx,&p.root,&p.twang,&p.nasal]{set(setter,param,0.);}}
                            });
                            ui.add_space(12.);
                            let level=f32::from_bits(shared.level.load(Ordering::Relaxed));
                            ui.add(egui::ProgressBar::new(level.sqrt().clamp(0.,1.)).fill(GREEN).text(if level>1e-5{format!("Output  {:.1} dBFS",20.*level.log10())}else{"Output —".into()}));
                            ui.add(widgets::ParamSlider::for_param(&p.gain,setter));
                            ui.small("Output level");
                        });
                        ui.separator();
                        ui.vertical(|ui|{
                            ui.set_width(535.);
                            match state.tab {
                                0=>{
                                    ui.heading("Make it your voice");
                                    ui.small("Start with these four controls, then explore Mouth or Textures.");
                                    ui.add_space(10.);
                                    sliders(ui,setter,&[&p.lab.air,&p.lab.effort,&p.lab.size,&p.lab.movement]);
                                    ui.add_space(12.);
                                    ui.collapsing("Tone & envelope",|ui|{sliders(ui,setter,&[&p.vowel,&p.breath,&p.brightness,&p.tension,&p.nasal,&p.vibrato,&p.glide,&p.attack,&p.release]);});
                                    if p.mode.value()==Mode::Loop {ui.add_space(10.);ui.label("Phrase rhythm");ui.add(widgets::ParamSlider::for_param(&p.step_beats,setter));if !p.lab.poly.value(){ui.label("Default pitch");ui.add(widgets::ParamSlider::for_param(&p.root_note,setter));}}
                                    ui.add_space(12.);
                                    ui.label(match p.mode.value(){Mode::Vowels=>"Play MIDI notes to sing a vowel.",Mode::Loop=>if p.lab.poly.value(){"Start Live and hold a chord. Each note sings the same phrase."}else{"Start Live to sing the phrase in time."},Mode::Syllables=>"Play notes to advance the words. Hold to stretch each vowel."});
                                }
                                1=>{ui.heading("Shape the mouth");ui.small("Fine control of the same handles shown on the left.");sliders(ui,setter,&[&p.tongue_x,&p.tongue_y,&p.jaw,&p.lips,&p.larynx]);ui.collapsing("Tongue tip, root & resonance",|ui|{sliders(ui,setter,&[&p.tip_x,&p.tip_y,&p.lip_prot,&p.root,&p.twang,&p.nasal]);});}
                                _=>lab_panel(ui,setter,&p,&shared,state,&executor),
                            }
                        });
                    });
                    ui.separator();
                    ui.horizontal(|ui|{
                        let bpm=f32::from_bits(shared.bpm.load(Ordering::Relaxed));ui.small(format!("{bpm:.1} BPM · {}",if shared.playing.load(Ordering::Relaxed){"Playing"}else{"Stopped"}));
                        ui.menu_button("Keyboard & display",|ui|{
                            ui.checkbox(&mut state.local,"Local audition keys");
                            ui.small("Normally leave this off: enable Live's Computer MIDI Keyboard and arm the track.");
                            if state.local {ui.small("Local keys do not record MIDI into Live.");ui.horizontal(|ui|{if ui.button("Octave −").clicked(){state.octave=(state.octave-1).max(2);}ui.label(format!("C{}",state.octave-1));if ui.button("Octave +").clicked(){state.octave=(state.octave+1).min(6);}});}
                            ui.label("Interface scale");ui.add(widgets::ParamSlider::for_param(&p.lab.ui_scale,setter));
                        });
                        ui.small("Classic sound preserved · new layers are optional");
                    });
                });
            });
            if ctx.input(|i| i.pointer.any_released()) {
                if let Some(before) = state.before.take() {
                    if before.fingerprint() != presets::Snapshot::capture(&p, "").fingerprint() {
                        state.undo.push(before);
                        if state.undo.len() > 32 {
                            state.undo.remove(0);
                        }
                        state.redo.clear();
                    }
                }
            }
            ctx.data_mut(|d| {
                d.insert_temp(egui::Id::new("capture-unfocused-keyboard"), state.local)
            });
            let wants_text = ctx.wants_keyboard_input();
            let mask = if state.local && !wants_text && ctx.input(|i| i.focused) {
                ctx.input(|i| {
                    use egui::Key::*;
                    [A, W, S, E, D, F, T, G, Y, H, U, J, K, O, L, P, Semicolon]
                        .iter()
                        .enumerate()
                        .fold(0, |bits, (n, key)| {
                            if i.key_down(*key) {
                                bits | (1 << n)
                            } else {
                                bits
                            }
                        })
                })
            } else {
                0
            };
            shared
                .local_base
                .store((state.octave * 12) as u32, Ordering::Relaxed);
            shared.local_keys.store(mask, Ordering::Release);
        },
    )?;
    Some(Box::new(ReleaseEditor {
        inner,
        shared: close_state,
    }))
}
fn set<P: Param>(setter: &ParamSetter, param: &P, value: P::Plain) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value);
    setter.end_set_parameter(param);
}
fn beam(p: &egui::Painter, points: Vec<Pos2>, color: Color32, width: f32) {
    if points.len() < 2 {
        return;
    }
    p.add(egui::Shape::line(
        points.clone(),
        Stroke::new(width + 4.0, color.gamma_multiply(0.08)),
    ));
    p.add(egui::Shape::line(points, Stroke::new(width, color)));
}
fn anatomy(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    params: &Controls,
    shared: &Shared,
    state: &mut UiState,
    voice: usize,
    width: f32,
) {
    let factor = width / 500.;
    let (rect, _) = ui.allocate_exact_size(EVec::new(width, 495.0 * factor), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(1, 5, 8));
    let scale = 16.2_f32 * factor;
    let origin = Pos2::new(rect.left() + 220.0 * factor, rect.bottom() - 82.0 * factor);
    let screen = |p: Vec2| Pos2::new(origin.x + p.x as f32 * scale, origin.y - p.y as f32 * scale);
    let live = shared.sounding.load(Ordering::Relaxed);
    let artic = if live {
        HumanArtic::from_array(
            &(if voice == 0 {
                &shared.articulation
            } else {
                &shared.layer_articulation[voice - 1]
            })
            .each_ref()
            .map(|x| f32::from_bits(x.load(Ordering::Relaxed)) as f64),
        )
    } else {
        params.base_artic(&state.vowels).apply_to(&NEUTRAL)
    };
    if state.cached_artic[voice] != Some(artic) {
        state.models[voice].compute(&artic, &mut state.frames[voice]);
        state.cached_artic[voice] = Some(artic);
    }
    let f = &state.frames[voice];
    // Profile landmarks adapted from the upstream profile.ts; airway geometry below is the real model.
    let outline = [
        (-6.6, -4.6),
        (-6.8, 4.8),
        (-8.3, 11.9),
        (-8.7, 15.4),
        (-6.9, 19.6),
        (-2.2, 22.7),
        (3.0, 22.9),
        (6.9, 20.5),
        (7.9, 18.1),
        (7.7, 15.0),
        (8.35, 13.9),
        (8.95, 12.6),
        (9.25, 11.65),
        (8.75, 11.05),
        (7.05, 10.6),
    ];
    let contour = catmull_rom(&outline.map(|(x, y)| v(x, y)), 4);
    beam(
        &painter,
        contour.iter().copied().map(screen).collect(),
        BLUE.gamma_multiply(0.62),
        1.15,
    );
    let chin = [
        v(7.6, 7.2 - artic.jaw),
        v(8.0, 4.6 - artic.jaw * 0.6),
        v(7.25, 3.35 - artic.jaw * 0.4),
        v(3.0, 1.7),
        v(2.38, -1.5),
        v(2.3, -4.6),
    ];
    beam(
        &painter,
        catmull_rom(&chin, 5).into_iter().map(screen).collect(),
        BLUE.gamma_multiply(0.62),
        1.1,
    );
    if state.wire {
        // Cranial contours are a visual shell; the throat cross-sections use measured area.
        for j in 0..10 {
            let y = 12.0 + j as f64 * 1.04;
            let ry = ((y - 17.0) / 6.4).clamp(-0.99, 0.99);
            let width = 8.1 * (1.0 - ry * ry).sqrt();
            let pts: Vec<_> = (0..65)
                .map(|i| {
                    let a = i as f64 / 64.0 * std::f64::consts::TAU;
                    screen(v(-0.1 + width * a.cos(), y + 0.52 * a.sin()))
                })
                .collect();
            beam(&painter, pts, BLUE.gamma_multiply(0.18), 0.6);
        }
    }
    for y in [-3.0, -1.2, 0.6, 2.4, 4.2, 6.0, 7.8, 9.6] {
        let pts = [
            v(-4.6, y),
            v(-2.5, y),
            v(-2.3, y + 1.2),
            v(-4.6, y + 1.3),
            v(-4.6, y),
        ];
        beam(
            &painter,
            pts.into_iter().map(screen).collect(),
            BLUE.gamma_multiply(0.3),
            0.8,
        );
    }
    let eye: Vec<_> = (0..49)
        .map(|i| {
            let a = i as f64 / 48.0 * std::f64::consts::TAU;
            screen(v(6.5 + 1.1 * a.cos(), 15.4 + 0.95 * a.sin()))
        })
        .collect();
    beam(&painter, eye, BLUE.gamma_multiply(0.5), 0.8);
    for (points, color) in [
        (&f.outer, BLUE),
        (&f.inner, GREEN),
        (&f.tongue, GREEN),
        (&f.velum, AMBER),
        (&f.upper_lip, PINK),
        (&f.lower_lip, PINK),
        (&f.upper_teeth, Color32::GRAY),
        (&f.lower_teeth, Color32::GRAY),
        (&f.jaw_bone, PINK),
        (&f.epiglottis, AMBER),
        (&f.nasal_cavity, BLUE),
    ] {
        beam(
            &painter,
            points.iter().copied().map(screen).collect(),
            color.gamma_multiply(0.85),
            1.0,
        );
    }
    let level = f32::from_bits(shared.level.load(Ordering::Relaxed));
    for sample in &f.grid {
        beam(
            &painter,
            vec![screen(sample.inner), screen(sample.outer)],
            AMBER.gamma_multiply((0.15 + level * 2.0).min(0.8)),
            0.7,
        );
        if state.wire {
            let a = screen(sample.inner);
            let b = screen(sample.outer);
            let mid = (a + b.to_vec2()) * 0.5;
            let half = (b - a) * 0.5;
            let depth = (sample.area.max(0.01).sqrt() as f32 * scale * 0.28).min(13.0);
            let pts = (0..33)
                .map(|i| {
                    let t = i as f32 / 32.0 * std::f32::consts::TAU;
                    mid + half * t.cos() + EVec::new(depth * t.sin(), depth * 0.2 * t.sin())
                })
                .collect();
            beam(&painter, pts, GREEN.gamma_multiply(0.22), 0.6);
        }
    }
    let lips = f.upper_lip.get(6).copied().unwrap_or(v(7.0, 8.0));
    let velum = f
        .velum
        .get(f.velum.len() / 2)
        .copied()
        .unwrap_or(v(1.0, 9.6));
    let handles = [
        (f.tongue_center, "TONGUE", GREEN),
        (f.tip, "TIP", GREEN),
        (v(6.5, 4.8 - artic.jaw), "JAW", PINK),
        (lips, "LIPS", PINK),
        (velum, "PALATE", AMBER),
        (v(f.glottis.x + 1.3, f.glottis.y), "LARYNX", AMBER),
    ];
    for (id, (point, label, color)) in handles.into_iter().enumerate() {
        let center = screen(point);
        let response = ui.interact(
            egui::Rect::from_center_size(center, EVec::splat(24.0)),
            ui.id().with(("tract", voice, id)),
            egui::Sense::drag(),
        );
        let (p1, p2) = match id {
            0 => (&params.tongue_x, Some(&params.tongue_y)),
            1 => (&params.tip_x, Some(&params.tip_y)),
            2 => (&params.jaw, None),
            3 => (&params.lips, Some(&params.lip_prot)),
            4 => (&params.nasal, None),
            _ => (&params.larynx, None),
        };
        if response.drag_started() {
            state.drag = Some((id, p1.value(), p2.map_or(0.0, |p| p.value()), EVec::ZERO));
            setter.begin_set_parameter(p1);
            if let Some(p) = p2 {
                setter.begin_set_parameter(p);
            }
        }
        if response.dragged() {
            if let Some((drag_id, a, b, movement)) = state.drag.as_mut() {
                if *drag_id == id {
                    // egui reports movement since the last frame, not since mouse-down.
                    *movement += response.drag_delta();
                    let delta = *movement / scale;
                    let (dx, dy) = match id {
                        0 | 1 => (delta.x / 0.86, -delta.y),
                        2 => (delta.y, 0.0),
                        3 => (delta.y * 1.2, delta.x),
                        4 => (delta.y * 0.8, 0.0),
                        _ => (-delta.y, 0.0),
                    };
                    setter.set_parameter(
                        p1,
                        p1.preview_plain(p1.preview_normalized(*a + dx).clamp(0.0, 1.0)),
                    );
                    if let Some(p) = p2 {
                        setter.set_parameter(
                            p,
                            p.preview_plain(p.preview_normalized(*b + dy).clamp(0.0, 1.0)),
                        );
                    }
                }
            }
        }
        if response.drag_stopped() {
            setter.end_set_parameter(p1);
            if let Some(p) = p2 {
                setter.end_set_parameter(p);
            }
            state.drag = None;
        }
        painter.circle_stroke(
            center,
            if response.hovered() || response.dragged() {
                7.0
            } else {
                4.5
            },
            Stroke::new(1.4, color),
        );
        painter.circle_filled(center, 2.0, color);
        if response.hovered() || response.dragged() {
            painter.text(
                center + EVec::new(12.0, -12.0),
                egui::Align2::LEFT_BOTTOM,
                label,
                egui::FontId::monospace(12.0),
                color,
            );
        }
    }
    painter.text(
        rect.left_top() + EVec::new(12.0, 12.0),
        egui::Align2::LEFT_TOP,
        ["ADULT / MALE", "ADULT / FEMALE", "SMALL VOICE"][voice],
        egui::FontId::monospace(11.0),
        BLUE,
    );
    painter.text(
        rect.left_bottom() + EVec::new(12.0, -48.0),
        egui::Align2::LEFT_BOTTOM,
        format!("TRACT {:.1} cm   MOUTH {:.2} cm²", f.length, f.lip_area),
        egui::FontId::monospace(12.0),
        GREEN,
    );
    painter.text(
        rect.left_bottom() + EVec::new(12.0, -25.0),
        egui::Align2::LEFT_BOTTOM,
        format!(
            "PITCH {:.1} Hz   NASAL PORT {:.2} cm²",
            f32::from_bits(shared.pitch.load(Ordering::Relaxed)),
            f.velum_port
        ),
        egui::FontId::monospace(12.0),
        AMBER,
    );
}

// Closing the editor can happen without an Unfocused frame. Release local notes explicitly.
struct ReleaseEditor {
    inner: Box<dyn Editor>,
    shared: Arc<Shared>,
}
struct ReleaseHandle {
    _inner: Box<dyn std::any::Any + Send>,
    shared: Arc<Shared>,
}
impl Drop for ReleaseHandle {
    fn drop(&mut self) {
        self.shared.local_keys.store(0, Ordering::Release);
        self.shared.preview.store(false, Ordering::Release);
    }
}
impl Editor for ReleaseEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        Box::new(ReleaseHandle {
            _inner: self.inner.spawn(parent, context),
            shared: self.shared.clone(),
        })
    }
    fn size(&self) -> (u32, u32) {
        self.inner.size()
    }
    fn set_scale_factor(&self, scale: f32) -> bool {
        self.inner.set_scale_factor(scale)
    }
    fn param_value_changed(&self, id: &str, value: f32) {
        self.inner.param_value_changed(id, value);
    }
    fn param_modulation_changed(&self, id: &str, value: f32) {
        self.inner.param_modulation_changed(id, value);
    }
    fn param_values_changed(&self) {
        self.inner.param_values_changed();
    }
}

fn preset_bar(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    p: &Controls,
    shared: &Shared,
    state: &mut UiState,
    executor: &AsyncExecutor<ChimeraPlugin>,
) {
    ui.horizontal_wrapped(|ui|{
        let mut chosen=None;
        egui::ComboBox::from_id_salt("factory").selected_text("Factory voices").show_ui(ui,|ui|{for(i,name)in presets::FACTORY.iter().enumerate(){if ui.selectable_label(false,*name).clicked(){chosen=Some(presets::factory(i));}}});
        egui::ComboBox::from_id_salt("user-presets").selected_text("Saved voices").show_ui(ui,|ui|{for name in &state.preset_names{if ui.selectable_label(false,name).clicked(){match presets::load(name){Ok(s)=>chosen=Some(s),Err(e)=>state.status=e}}}});
        if let Some(s)=chosen {s.apply(p,setter,shared);state.draft=s.lyrics.clone();state.preset_name=s.name.clone();executor.execute_background(Task::Compile);shared.rewind.store(true,Ordering::Release);}
        ui.menu_button("Save…",|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.preset_name).desired_width(140.).char_limit(64));
        if ui.button("Save voice").clicked(){state.status=presets::save(&presets::Snapshot::capture(p,&state.preset_name)).unwrap_or_else(|e|e);state.preset_names=presets::list();}
        });
        if ui.add_enabled(!state.undo.is_empty(),egui::Button::new("Undo")).clicked(){if let Some(s)=state.undo.pop(){state.redo.push(presets::Snapshot::capture(p,"Redo"));s.apply(p,setter,shared);state.draft=s.lyrics;state.before=None;executor.execute_background(Task::Compile);}}
        if ui.add_enabled(!state.redo.is_empty(),egui::Button::new("Redo")).clicked(){if let Some(s)=state.redo.pop(){state.undo.push(presets::Snapshot::capture(p,"Undo"));s.apply(p,setter,shared);state.draft=s.lyrics;state.before=None;executor.execute_background(Task::Compile);}}
        ui.menu_button("Explore",|ui|{
        if ui.button("Mutate").on_hover_text("Small bounded changes to the mouth and texture; undo returns to the previous voice.").clicked(){
            let mut rnd=||{state.random^=state.random<<13;state.random^=state.random>>7;state.random^=state.random<<17;(state.random as u32)as f32/u32::MAX as f32};
            for param in [&p.tongue_x,&p.tongue_y,&p.jaw,&p.lips,&p.root,&p.twang,&p.lab.wet,&p.lab.fry,&p.lab.rough] {let v=(param.unmodulated_normalized_value()+(rnd()-0.5)*0.24).clamp(0.,1.);set(setter,param,param.preview_plain(v));}
            if !state.lock_pitch {set(setter,&p.root_note,36+(rnd()*36.)as i32);}
            if !state.lock_lyrics {let mut session=p.lab.session.lock().unwrap();session.durations.iter_mut().for_each(|x|*x=[0.5,1.,1.5,2.][(rnd()*3.99)as usize]);shared.session_revision.fetch_add(1,Ordering::Release);executor.execute_background(Task::Compile);}
        }
        ui.checkbox(&mut state.lock_pitch,"Lock pitch");ui.checkbox(&mut state.lock_lyrics,"Lock lyric timing");
        });
        ui.label(egui::RichText::new(&state.preset_name).color(GREEN));
    });
    if !state.status.is_empty() {
        ui.small(&state.status);
    }
}

fn sliders(ui: &mut egui::Ui, setter: &ParamSetter, params: &[&FloatParam]) {
    egui::Grid::new("lab-sliders")
        .num_columns(2)
        .spacing([10., 8.])
        .show(ui, |ui| {
            for param in params {
                ui.label(param.name());
                ui.add(widgets::ParamSlider::for_param(*param, setter));
                ui.end_row();
            }
        });
}
fn toggle(ui: &mut egui::Ui, setter: &ParamSetter, p: &BoolParam) {
    let mut on = p.value();
    if ui.checkbox(&mut on, p.name()).changed() {
        set(setter, p, on);
    }
}
fn lab_panel(
    ui: &mut egui::Ui,
    setter: &ParamSetter,
    p: &Controls,
    shared: &Shared,
    state: &mut UiState,
    executor: &AsyncExecutor<ChimeraPlugin>,
) {
    let l = &p.lab;
    match state.tab {
        2 => {
            ui.label(egui::RichText::new("WET MOUTH / THROAT TEXTURES").color(PINK));
            sliders(
                ui,
                setter,
                &[&l.wet, &l.noise_level, &l.noise_decay, &l.noise_tone],
            );
            ui.collapsing("Throat character", |ui| {
                sliders(
                    ui,
                    setter,
                    &[
                        &l.fry,
                        &l.rough,
                        &l.ventricular,
                        &l.suction,
                        &l.curl,
                        &l.lateral,
                    ],
                );
            });
            ui.small("Tap a sound below, or play MIDI notes 24–33.");
            ui.horizontal_wrapped(|ui| {
                for (i, sound) in chimera::phonetics::beatbox::BEAT_SOUNDS.iter().enumerate() {
                    if ui.button(sound.name).on_hover_text(sound.hint).clicked() {
                        if l.noise_level.value() == 0. {
                            set(setter, &l.noise_level, 0.65);
                        }
                        shared.hit.store(i as u32 + 1, Ordering::Release);
                    }
                }
            });
            ui.small(
                "MIDI notes 24–33 trigger these ten sounds when percussion level is above zero.",
            );
            ui.collapsing("Rhythm sequencer", |ui| {
                ui.horizontal(|ui| {
                    toggle(ui, setter, &l.sequencer);
                    let preview = shared.preview.load(Ordering::Relaxed);
                    if ui
                        .button(if preview {
                            "Stop audition"
                        } else {
                            "Audition pattern"
                        })
                        .clicked()
                    {
                        shared.preview.store(!preview, Ordering::Release);
                        shared.rewind.store(true, Ordering::Release);
                    }
                });
                ui.small("16 steps: click to cycle sounds; right-click accents. A dot is a rest.");
                let mut session = l.session.lock().unwrap();
                let mut changed = false;
                egui::Grid::new("mouth-steps")
                    .num_columns(8)
                    .spacing([4., 5.])
                    .show(ui, |ui| {
                        for i in 0..16 {
                            let hit = session.steps[i];
                            let label = if hit == 0 {
                                "·"
                            } else {
                                chimera::phonetics::beatbox::BEAT_SOUNDS[(hit - 1) as usize].id
                            };
                            let r = ui.add_sized(
                                [54., 28.],
                                egui::Button::new(format!(
                                    "{} {}{}",
                                    i + 1,
                                    label,
                                    if session.accents[i] > 1. { "!" } else { "" }
                                )),
                            );
                            if r.clicked() {
                                session.steps[i] = (hit + 1) % 11;
                                changed = true;
                            }
                            if r.secondary_clicked() {
                                session.accents[i] = if session.accents[i] > 1. { 1. } else { 1.4 };
                                changed = true;
                            }
                            if i % 8 == 7 {
                                ui.end_row();
                            }
                        }
                    });
                if changed {
                    shared.session_revision.fetch_add(1, Ordering::Release);
                }
                drop(session);
                sliders(ui, setter, &[&l.swing]);
            });
        }
        3 => {
            ui.label(egui::RichText::new("RECORD THE MOUTH").color(GREEN));
            let recording = shared.motion_state.load(Ordering::Acquire);
            ui.horizontal(|ui| {
                if ui
                    .button(if recording > 0 {
                        "Cancel recording"
                    } else {
                        "Record one loop"
                    })
                    .clicked()
                {
                    shared
                        .motion_state
                        .store(if recording > 0 { 0 } else { 1 }, Ordering::Release);
                }
                ui.label(match recording {
                    1 => "Armed · starts on the next beat",
                    2 => "Recording your mouth movements",
                    3 | 4 => "Saving loop…",
                    _ => "Play Live, then move the tongue, lips or throat",
                });
            });
            sliders(
                ui,
                setter,
                &[&l.motion_length, &l.motion_depth, &l.morph_depth],
            );
            ui.small("Turn Motion playback depth up after recording. Recorded movement and shapes are saved with your voice and Live Set.");
            let mut session = l.session.lock().unwrap();
            ui.horizontal(|ui| {
                for i in 0..4 {
                    if ui
                        .button(format!(
                            "Store {}{}",
                            ['A', 'B', 'C', 'D'][i],
                            if session.shape_set[i] { " ✓" } else { "" }
                        ))
                        .clicked()
                    {
                        session.shapes[i] = lab::pose(p);
                        session.shape_set[i] = true;
                        shared.session_revision.fetch_add(1, Ordering::Release);
                    }
                }
            });
            let (rect, response) =
                ui.allocate_exact_size(EVec::new(490., 170.), egui::Sense::drag());
            let paint = ui.painter_at(rect);
            paint.rect_filled(rect, 4., Color32::from_rgb(3, 17, 21));
            for (pos, label) in [
                (rect.left_top(), "A"),
                (rect.right_top() - EVec::new(20., 0.), "B"),
                (rect.left_bottom() - EVec::new(0., 20.), "C"),
                (rect.right_bottom() - EVec::new(20., 20.), "D"),
            ] {
                paint.text(
                    pos + EVec::splat(4.),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::monospace(14.),
                    GREEN,
                );
            }
            if response.drag_started() {
                setter.begin_set_parameter(&l.morph_x);
                setter.begin_set_parameter(&l.morph_y);
            }
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    setter.set_parameter(
                        &l.morph_x,
                        ((pos.x - rect.left()) / rect.width()).clamp(0., 1.),
                    );
                    setter.set_parameter(
                        &l.morph_y,
                        ((pos.y - rect.top()) / rect.height()).clamp(0., 1.),
                    );
                }
            }
            if response.drag_stopped() {
                setter.end_set_parameter(&l.morph_x);
                setter.end_set_parameter(&l.morph_y);
            }
            let point = rect.left_top()
                + EVec::new(
                    l.morph_x.value() * rect.width(),
                    l.morph_y.value() * rect.height(),
                );
            paint.circle_filled(point, 7., GREEN);
            let frames = session.motion.len();
            ui.label(format!(
                "{frames} recorded motion frames · XY corners are saved mouth positions"
            ));
        }
        4 => {
            ui.label(egui::RichText::new("PHRASING / PRONUNCIATION").color(AMBER));
            ui.horizontal(|ui| {
                toggle(ui, setter, &l.hold);
                toggle(ui, setter, &l.legato);
            });
            toggle(ui, setter, &l.natural);
            ui.small("Hold keeps one vowel across new notes. Legato does this only while notes overlap. A standalone ~ in lyrics also extends a vowel.");
            sliders(
                ui,
                setter,
                &[&l.consonants, &l.swing, &l.vibrato_delay, &l.scoop],
            );
            let labels = shared
                .bank
                .try_lock()
                .ok()
                .map(|b| {
                    b.labels[..b.len]
                        .iter()
                        .map(|l| l.as_str().to_owned())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            ui.horizontal_wrapped(|ui| {
                for (i, label) in labels.iter().enumerate() {
                    if ui
                        .selectable_label(state.selected_syllable == i, label)
                        .clicked()
                    {
                        state.selected_syllable = i;
                    }
                }
            });
            let i = state.selected_syllable.min(31);
            let mut session = l.session.lock().unwrap();
            ui.label(format!(
                "Selected syllable: {}",
                labels.get(i).map_or("—", String::as_str)
            ));
            ui.add(
                egui::Slider::new(&mut session.durations[i], 0.25..=8.)
                    .step_by(0.25)
                    .text("Length × rhythm"),
            );
            ui.checkbox(&mut session.rests[i], "Rest / silence this syllable");
            ui.label("Pronunciation: type an alternative spelling (one syllable)");
            ui.add(
                egui::TextEdit::singleline(&mut session.pronunciation[i])
                    .char_limit(48)
                    .hint_text("Leave empty to use the dictionary"),
            );
            if ui.button("Apply phrasing").clicked() {
                shared.session_revision.fetch_add(1, Ordering::Release);
                executor.execute_background(Task::Compile);
            }
            ui.small("Enable Natural lyric timing for custom lengths and swing. Consonants retain their timing as vowels stretch. Pronunciation overrides apply to both word modes.");
        }
        5 => {
            ui.heading("Play a choir");
            toggle(ui, setter, &l.poly);
            if l.poly.value() {
                ui.horizontal(|ui| {
                    ui.label("Maximum voices");
                    voice_limit(ui, setter, p);
                });
                ui.small("Independent notes, sustain and releases. Start at 2 or 4 voices; 6 costs more CPU. Silent voices sleep.");
                sliders(ui, setter, &[&l.spread, &l.detune]);
                ui.small("A mix of male, female and small voices; one physical singer per note. The older harmony layers are bypassed here.");
            } else {
                ui.small("Classic layering follows the lead note. Enable Poly choir for independent chord notes.");
                sliders(
                    ui,
                    setter,
                    &[&l.voice, &l.choir, &l.harmony, &l.detune, &l.spread],
                );
            }
            ui.collapsing("Controller expression",|ui|{
                sliders(ui,setter,&[&l.expression,&l.bend_range]);
                ui.small("Pressure → effort · CC74 slide → tongue · mod wheel → vibrato. Match the pitch-bend range on your controller.");
            });
            ui.collapsing("Experimental microphone follower",|ui|{
            if l.poly.value(){ui.small("Microphone following is available with Poly choir off.");return;}
            toggle(ui, setter, &l.mic);
            sliders(ui, setter, &[&l.mic_gain, &l.mic_gate]);
            let level = f32::from_bits(shared.mic_level.load(Ordering::Relaxed));
            let hz = f32::from_bits(shared.mic_hz.load(Ordering::Relaxed));
            let confidence = f32::from_bits(shared.mic_confidence.load(Ordering::Relaxed));
            ui.add(egui::ProgressBar::new((level * 8.).min(1.)).text(format!(
                "MIC  {hz:.0} Hz · confidence {:.0}%",
                confidence * 100.
            )));
            ui.small("Route a monophonic mic/audio track to the VST3 sidechain. Vowels follow its pitch and breath; Phrase Loop follows its pitch while lyrics run. Pitch tracking adds analysis delay; headphones avoid feedback.");
            });
        }
        _ => {}
    }
}

fn voice_limit(ui: &mut egui::Ui, setter: &ParamSetter, p: &Controls) {
    for n in [2, 4, 6] {
        if ui
            .selectable_label(p.lab.voice_limit.value() == n, format!("{n}"))
            .on_hover_text(if n == 2 {
                "Lightest chord setting"
            } else if n == 4 {
                "Balanced voice limit"
            } else {
                "Six physical voices: higher CPU"
            })
            .clicked()
        {
            set(setter, &p.lab.voice_limit, n);
        }
    }
}
