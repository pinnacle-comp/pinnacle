use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::atomic::{AtomicU32, Ordering},
};

use indexmap::{IndexMap, map::Entry};
use pinnacle_api_defs::pinnacle::input::v1::{GestureDirection, GestureFingers};
use smithay::input::keyboard::ModifiersState;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xkbcommon::xkb::Keysym;

static BIND_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Default)]
pub struct BindState {
    pub layer_stack: Vec<String>,
    pub keybinds: Keybinds,
    pub mousebinds: Mousebinds,
    pub gesturebinds: Gesturebinds,
}

impl BindState {
    pub fn clear(&mut self) {
        self.keybinds.id_map.clear();
        self.keybinds.keysym_map.clear();
        self.mousebinds.id_map.clear();
        self.mousebinds.button_map.clear();
    }

    pub fn enter_layer(&mut self, layer: Option<String>) {
        match layer {
            Some(layer) => {
                self.layer_stack.retain(|l| *l != layer);
                self.layer_stack.push(layer);
            }
            None => self.layer_stack.clear(),
        }
    }

    pub fn current_layer(&self) -> Option<String> {
        self.layer_stack.last().cloned()
    }

    pub fn enter_previous_layer(&mut self) {
        self.layer_stack.pop();
    }

    pub fn set_bind_group(&self, bind_id: u32, group: String) {
        if let Some(bind) = self.keybinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.group = group;
        } else if let Some(bind) = self.mousebinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.group = group;
        }
    }

    pub fn set_bind_desc(&self, bind_id: u32, desc: String) {
        if let Some(bind) = self.keybinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.desc = desc;
        } else if let Some(bind) = self.mousebinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.desc = desc;
        }
    }

    pub fn set_quit(&self, bind_id: u32, quit: bool) {
        if let Some(bind) = self.keybinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.is_quit_bind = quit;
        } else if let Some(bind) = self.mousebinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.is_quit_bind = quit;
        }
    }

    pub fn set_reload_config(&self, bind_id: u32, reload_config: bool) {
        if let Some(bind) = self.keybinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.is_reload_config_bind = reload_config;
        } else if let Some(bind) = self.mousebinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.is_reload_config_bind = reload_config;
        }
    }

    pub fn set_allow_when_locked(&self, bind_id: u32, allow_when_locked: bool) {
        if let Some(bind) = self.keybinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.allow_when_locked = allow_when_locked;
        } else if let Some(bind) = self.mousebinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.allow_when_locked = allow_when_locked;
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Edge {
    Press,
    Release,
}

#[derive(Debug, Copy, Clone)]
pub struct ModMask {
    pub shift: Option<bool>,
    pub ctrl: Option<bool>,
    pub alt: Option<bool>,
    pub super_: Option<bool>,
    pub iso_level3_shift: Option<bool>,
    pub iso_level5_shift: Option<bool>,
}

impl ModMask {
    pub fn new() -> Self {
        Self {
            shift: Some(false),
            ctrl: Some(false),
            alt: Some(false),
            super_: Some(false),
            iso_level3_shift: Some(false),
            iso_level5_shift: Some(false),
        }
    }

    pub fn matches(&self, mod_state: ModifiersState) -> bool {
        let shift = self.shift.is_none_or(|shift| shift == mod_state.shift);
        let ctrl = self.ctrl.is_none_or(|ctrl| ctrl == mod_state.ctrl);
        let alt = self.alt.is_none_or(|alt| alt == mod_state.alt);
        let super_ = self.super_.is_none_or(|super_| super_ == mod_state.logo);
        let iso_level3_shift = self
            .iso_level3_shift
            .is_none_or(|iso_level3_shift| iso_level3_shift == mod_state.iso_level3_shift);
        let iso_level5_shift = self
            .iso_level5_shift
            .is_none_or(|iso_level5_shift| iso_level5_shift == mod_state.iso_level5_shift);

        shift && ctrl && alt && super_ && iso_level3_shift && iso_level5_shift
    }
}

#[derive(Debug)]
pub struct BindData {
    pub id: u32,
    pub mods: ModMask,
    pub layer: Option<String>,
    pub group: String,
    pub desc: String,
    pub is_quit_bind: bool,
    pub is_reload_config_bind: bool,
    pub allow_when_locked: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BindAction {
    Forward,
    Suppress,
    Quit,
    ReloadConfig,
}

// Keybinds

#[derive(Debug)]
pub struct Keybind {
    pub bind_data: BindData,
    pub key: Keysym,
    sender: UnboundedSender<Edge>,
    pub recv: Option<UnboundedReceiver<Edge>>,
    pub has_on_press: bool,
}

#[derive(Debug, Default)]
pub struct Keybinds {
    pub id_map: IndexMap<u32, Rc<RefCell<Keybind>>>,
    keysym_map: IndexMap<Keysym, Vec<Weak<RefCell<Keybind>>>>,

    pub last_pressed_triggered_binds: HashMap<Keysym, Vec<u32>>,
}

impl Keybinds {
    /// Notifies configs that a key was pressed.
    ///
    /// Returns whether the key should be suppressed (not sent to the client).
    pub fn key(
        &mut self,
        key: Keysym,
        mods: ModifiersState,
        edge: Edge,
        current_layer: Option<String>,
        shortcuts_inhibited: bool,
        is_locked: bool,
    ) -> BindAction {
        let Some(keybinds) = self.keysym_map.get_mut(&key) else {
            return BindAction::Forward;
        };

        if edge == Edge::Release {
            let last_triggered_binds_on_press = self.last_pressed_triggered_binds.remove(&key);
            let bind_action = if let Some(bind_ids) = last_triggered_binds_on_press {
                let mut bind_action = BindAction::Forward;
                for bind_id in bind_ids {
                    let keybind = self.id_map.entry(bind_id);
                    let Entry::Occupied(kb_entry) = keybind else {
                        continue;
                    };
                    if kb_entry.get().borrow().bind_data.is_quit_bind {
                        return BindAction::Quit;
                    }
                    if kb_entry.get().borrow().bind_data.is_reload_config_bind {
                        return BindAction::ReloadConfig;
                    }
                    if shortcuts_inhibited
                        || (is_locked && !kb_entry.get().borrow().bind_data.allow_when_locked)
                    {
                        return BindAction::Forward;
                    }
                    if kb_entry.get().borrow().has_on_press {
                        bind_action = BindAction::Suppress;
                    }
                    let sent = kb_entry.get().borrow().sender.send(Edge::Release).is_ok();
                    if !sent {
                        kb_entry.shift_remove();
                    }
                }
                bind_action
            } else {
                BindAction::Forward
            };
            return bind_action;
        }

        let mut bind_action = BindAction::Forward;

        let mut should_clear_releases = false;

        keybinds.retain(|keybind| {
            let Some(keybind) = keybind.upgrade() else {
                return false;
            };

            let keybind = keybind.borrow();

            let same_layer = current_layer == keybind.bind_data.layer;

            if let BindAction::Quit | BindAction::ReloadConfig = bind_action {
                return true;
            }

            if keybind.bind_data.mods.matches(mods) {
                if keybind.has_on_press {
                    should_clear_releases = true;
                }

                match edge {
                    Edge::Press => {
                        self.last_pressed_triggered_binds
                            .entry(key)
                            .or_default()
                            .push(keybind.bind_data.id);
                    }
                    Edge::Release => unreachable!(),
                }

                let mut retain = true;

                if keybind.bind_data.is_quit_bind {
                    bind_action = BindAction::Quit;
                } else if keybind.bind_data.is_reload_config_bind {
                    bind_action = BindAction::ReloadConfig;
                } else if keybind.has_on_press
                    && same_layer
                    && (!shortcuts_inhibited && (!is_locked || keybind.bind_data.allow_when_locked))
                {
                    retain = keybind.sender.send(edge).is_ok();
                    bind_action = BindAction::Suppress;
                };

                retain
            } else {
                true
            }
        });

        if should_clear_releases {
            self.last_pressed_triggered_binds
                .retain(|keysym, _| *keysym == key);
        }

        bind_action
    }

    pub fn add_keybind(
        &mut self,
        key: Keysym,
        mods: ModMask,
        layer: Option<String>,
        group: String,
        desc: String,
        is_quit_bind: bool,
        is_reload_config_bind: bool,
        allow_when_locked: bool,
    ) -> u32 {
        let id = BIND_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        let (sender, recv) = tokio::sync::mpsc::unbounded_channel::<Edge>();

        let keybind = Rc::new(RefCell::new(Keybind {
            bind_data: BindData {
                id,
                mods,
                layer,
                group,
                desc,
                is_quit_bind,
                is_reload_config_bind,
                allow_when_locked,
            },
            key,
            sender,
            recv: Some(recv),
            has_on_press: false,
        }));

        assert!(
            self.id_map.insert(id, keybind.clone()).is_none(),
            "new keybind should have unique id"
        );

        self.keysym_map
            .entry(key)
            .or_default()
            .push(Rc::downgrade(&keybind));

        id
    }

    pub fn remove_keybind(&mut self, keybind_id: u32) {
        self.id_map.shift_remove(&keybind_id);
    }

    pub fn set_keybind_has_on_press(&self, keybind_id: u32) {
        let Some(keybind) = self.id_map.get(&keybind_id) else {
            return;
        };
        keybind.borrow_mut().has_on_press = true;
    }
}

// Mousebinds

#[derive(Debug)]
pub struct Mousebind {
    pub bind_data: BindData,
    pub button: u32,
    sender: UnboundedSender<Edge>,
    pub recv: Option<UnboundedReceiver<Edge>>,
    pub has_on_press: bool,
}

#[derive(Debug, Default)]
pub struct Mousebinds {
    pub id_map: IndexMap<u32, Rc<RefCell<Mousebind>>>,
    button_map: IndexMap<u32, Vec<Weak<RefCell<Mousebind>>>>,

    pub last_pressed_triggered_binds: HashMap<u32, Vec<u32>>,
}

// TODO: may be able to dedup with Keybinds above
impl Mousebinds {
    /// Notifies configs that a button was pressed.
    ///
    /// Returns whether the button should be suppressed (not sent to the client).
    ///
    /// Named `btn` and not `button` because Rust Analyzer does some weird things in `input.rs`
    pub fn btn(
        &mut self,
        button: u32,
        mods: ModifiersState,
        edge: Edge,
        current_layer: Option<String>,
        is_locked: bool,
    ) -> BindAction {
        let Some(mousebinds) = self.button_map.get_mut(&button) else {
            return BindAction::Forward;
        };

        if edge == Edge::Release {
            let last_triggered_binds_on_press = self.last_pressed_triggered_binds.remove(&button);
            let bind_action = if let Some(bind_ids) = last_triggered_binds_on_press {
                let mut bind_action = BindAction::Forward;
                for bind_id in bind_ids {
                    let mousebind = self.id_map.entry(bind_id);
                    let Entry::Occupied(mb_entry) = mousebind else {
                        continue;
                    };
                    if mb_entry.get().borrow().bind_data.is_quit_bind {
                        return BindAction::Quit;
                    }
                    if mb_entry.get().borrow().bind_data.is_reload_config_bind {
                        return BindAction::ReloadConfig;
                    }
                    if is_locked && !mb_entry.get().borrow().bind_data.allow_when_locked {
                        return BindAction::Forward;
                    }
                    if mb_entry.get().borrow().has_on_press {
                        bind_action = BindAction::Suppress;
                    }
                    let sent = mb_entry.get().borrow().sender.send(Edge::Release).is_ok();
                    if !sent {
                        mb_entry.shift_remove();
                    }
                }
                bind_action
            } else {
                BindAction::Forward
            };
            return bind_action;
        }

        let mut bind_action = BindAction::Forward;

        let mut should_clear_releases = false;

        mousebinds.retain(|mousebind| {
            let Some(mousebind) = mousebind.upgrade() else {
                return false;
            };

            let mousebind = mousebind.borrow();

            let same_layer = current_layer == mousebind.bind_data.layer;

            if let BindAction::Quit | BindAction::ReloadConfig = bind_action {
                return true;
            }

            if mousebind.bind_data.mods.matches(mods) {
                if mousebind.has_on_press {
                    should_clear_releases = true;
                }

                match edge {
                    Edge::Press => {
                        self.last_pressed_triggered_binds
                            .entry(button)
                            .or_default()
                            .push(mousebind.bind_data.id);
                    }
                    Edge::Release => unreachable!(),
                }

                let mut retain = true;

                if mousebind.bind_data.is_quit_bind {
                    bind_action = BindAction::Quit;
                } else if mousebind.bind_data.is_reload_config_bind {
                    bind_action = BindAction::ReloadConfig;
                } else if mousebind.has_on_press
                    && same_layer
                    && (!is_locked || mousebind.bind_data.allow_when_locked)
                {
                    retain = mousebind.sender.send(edge).is_ok();
                    bind_action = BindAction::Suppress;
                };

                retain
            } else {
                true
            }
        });

        if should_clear_releases {
            self.last_pressed_triggered_binds
                .retain(|btn, _| *btn == button);
        }

        bind_action
    }

    pub fn add_mousebind(
        &mut self,
        button: u32,
        mods: ModMask,
        layer: Option<String>,
        group: String,
        desc: String,
        is_quit_bind: bool,
        is_reload_config_bind: bool,
        allow_when_locked: bool,
    ) -> u32 {
        let id = BIND_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        let (sender, recv) = tokio::sync::mpsc::unbounded_channel::<Edge>();

        let mousebind = Rc::new(RefCell::new(Mousebind {
            bind_data: BindData {
                id,
                mods,
                layer,
                group,
                desc,
                is_quit_bind,
                is_reload_config_bind,
                allow_when_locked,
            },
            button,
            sender,
            recv: Some(recv),
            has_on_press: false,
        }));

        assert!(
            self.id_map.insert(id, mousebind.clone()).is_none(),
            "new keybind should have unique id"
        );

        self.button_map
            .entry(button)
            .or_default()
            .push(Rc::downgrade(&mousebind));

        id
    }

    pub fn remove_mousebind(&mut self, mousebind_id: u32) {
        self.id_map.shift_remove(&mousebind_id);
    }

    pub fn set_mousebind_has_on_press(&self, mousebind_id: u32) {
        let Some(mousebind) = self.id_map.get(&mousebind_id) else {
            return;
        };
        mousebind.borrow_mut().has_on_press = true;
    }
}

// Gesturebinds

#[derive(Debug)]
pub struct Gesturebind {
    pub bind_data: BindData,
    pub direction: GestureDirection,
    pub fingers: GestureFingers,
    sender: UnboundedSender<Edge>,
    pub recv: Option<UnboundedReceiver<Edge>>,
    pub has_on_begin: bool,
}

#[derive(Debug, Default)]
pub struct Gesturebinds {
    pub id_map: IndexMap<u32, Rc<RefCell<Gesturebind>>>,
    gesture_map: IndexMap<(GestureDirection, GestureFingers), Vec<Weak<RefCell<Gesturebind>>>>,

    pub last_pressed_triggered_binds: HashMap<(GestureDirection, GestureFingers), Vec<u32>>,
}

// TODO: may be able to dedup with Keybinds above
impl Gesturebinds {
    /// Notifies configs that a gesture was executed.
    ///
    /// Returns whether the gesture should be suppressed (not sent to the client).
    pub fn gesture(
        &mut self,
        direction: GestureDirection,
        fingers: GestureFingers,
        mods: ModifiersState,
        edge: Edge,
        _current_layer: Option<String>,
        is_locked: bool,
    ) -> BindAction {
        let Some(gesturebinds) = self.gesture_map.get_mut(&(direction, fingers)) else {
            return BindAction::Forward;
        };

        if edge == Edge::Release {
            let mut bind_action = BindAction::Forward;

            for gesturebind in gesturebinds {
                let Some(gesturebind) = gesturebind.upgrade() else {
                    continue;
                };

                if !gesturebind.borrow().bind_data.mods.matches(mods) {
                    continue;
                }

                if gesturebind.borrow().bind_data.is_quit_bind {
                    return BindAction::Quit;
                }
                if gesturebind.borrow().bind_data.is_reload_config_bind {
                    return BindAction::ReloadConfig;
                }
                if is_locked && !gesturebind.borrow().bind_data.allow_when_locked {
                    return BindAction::Forward;
                }
                if gesturebind.borrow().has_on_begin {
                    bind_action = BindAction::Suppress;
                }
                let _sent = gesturebind.borrow().sender.send(Edge::Release).is_ok();
            }

            return bind_action;
        }

        BindAction::Forward
    }

    pub fn add_gesturebind(
        &mut self,
        direction: GestureDirection,
        fingers: GestureFingers,
        mods: ModMask,
        layer: Option<String>,
        group: String,
        desc: String,
        is_quit_bind: bool,
        is_reload_config_bind: bool,
        allow_when_locked: bool,
    ) -> u32 {
        let id = BIND_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

        let (sender, recv) = tokio::sync::mpsc::unbounded_channel::<Edge>();

        let gesturebind = Rc::new(RefCell::new(Gesturebind {
            bind_data: BindData {
                id,
                mods,
                layer,
                group,
                desc,
                is_quit_bind,
                is_reload_config_bind,
                allow_when_locked,
            },
            direction,
            fingers,
            sender,
            recv: Some(recv),
            has_on_begin: false,
        }));

        assert!(
            self.id_map.insert(id, gesturebind.clone()).is_none(),
            "new keybind should have unique id"
        );

        self.gesture_map
            .entry((direction, fingers))
            .or_default()
            .push(Rc::downgrade(&gesturebind));

        id
    }

    pub fn remove_gesturebind(&mut self, gesturebind_id: u32) {
        self.id_map.shift_remove(&gesturebind_id);
    }

    pub fn set_gesturebind_has_on_begin(&self, gesturebind_id: u32) {
        let Some(gesturebind) = self.id_map.get(&gesturebind_id) else {
            return;
        };
        gesturebind.borrow_mut().has_on_begin = true;
    }

    pub fn set_gesturebind_has_on_finish(&self, gesturebind_id: u32) {
        let Some(gesturebind) = self.id_map.get(&gesturebind_id) else {
            return;
        };
        gesturebind.borrow_mut().has_on_begin = false;
    }
}
