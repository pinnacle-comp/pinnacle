use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::atomic::{AtomicU32, Ordering},
};

use indexmap::{map::Entry, IndexMap};
use smithay::input::keyboard::ModifiersState;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use xkbcommon::xkb::Keysym;

static BIND_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Default)]
pub struct BindState {
    pub layer_stack: Vec<String>,
    pub keybinds: Keybinds,
    pub mousebinds: Mousebinds,
}

impl BindState {
    pub fn clear(&mut self) {
        self.keybinds.id_map.clear();
        self.keybinds.keysym_map.clear();
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

    pub fn set_bind_group(&self, bind_id: u32, group: Option<String>) {
        if let Some(bind) = self.keybinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.group = group;
        } else if let Some(bind) = self.mousebinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.group = group;
        }
    }

    pub fn set_bind_desc(&self, bind_id: u32, desc: Option<String>) {
        if let Some(bind) = self.keybinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.desc = desc;
        } else if let Some(bind) = self.mousebinds.id_map.get(&bind_id) {
            bind.borrow_mut().bind_data.desc = desc;
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
    pub group: Option<String>,
    pub desc: Option<String>,
}

// Keybinds

#[derive(Debug)]
pub struct Keybind {
    pub bind_data: BindData,
    pub key: Keysym,
    pub sender: UnboundedSender<Edge>,
    pub recv: Option<UnboundedReceiver<Edge>>,
}

#[derive(Debug, Default)]
pub struct Keybinds {
    pub id_map: IndexMap<u32, Rc<RefCell<Keybind>>>,
    keysym_map: IndexMap<Keysym, Vec<Weak<RefCell<Keybind>>>>,

    pub last_pressed_triggered_binds: HashMap<Keysym, Vec<u32>>,
}

impl Keybinds {
    // Notifies configs that a key was pressed.
    //
    // Returns whether the key should be suppressed (not sent to the client).
    pub fn key(
        &mut self,
        key: Keysym,
        mods: ModifiersState,
        edge: Edge,
        current_layer: Option<String>,
    ) -> bool {
        let Some(keybinds) = self.keysym_map.get_mut(&key) else {
            return false;
        };

        if edge == Edge::Release {
            let last_triggered_binds_on_press = self.last_pressed_triggered_binds.remove(&key);
            let should_suppress = if let Some(bind_ids) = last_triggered_binds_on_press {
                for bind_id in bind_ids {
                    let keybind = self.id_map.entry(bind_id);
                    let Entry::Occupied(kb_entry) = keybind else {
                        continue;
                    };
                    let sent = kb_entry.get().borrow().sender.send(Edge::Release).is_ok();
                    if !sent {
                        kb_entry.shift_remove();
                    }
                }
                true
            } else {
                false
            };
            return should_suppress;
        }

        let mut should_suppress = false;

        keybinds.retain(|keybind| {
            let Some(keybind) = keybind.upgrade() else {
                return false;
            };

            let keybind = keybind.borrow();

            if current_layer != keybind.bind_data.layer {
                return true;
            }

            if keybind.bind_data.mods.matches(mods) {
                match edge {
                    Edge::Press => {
                        self.last_pressed_triggered_binds
                            .entry(key)
                            .or_default()
                            .push(keybind.bind_data.id);
                    }
                    Edge::Release => unreachable!(),
                }
                should_suppress = true;

                keybind.sender.send(edge).is_ok()
            } else {
                true
            }
        });

        should_suppress
    }

    pub fn add_keybind(
        &mut self,
        key: Keysym,
        mods: ModMask,
        layer: Option<String>,
        group: Option<String>,
        desc: Option<String>,
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
            },
            key,
            sender,
            recv: Some(recv),
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
}

// Mousebinds

#[derive(Debug)]
pub struct Mousebind {
    pub bind_data: BindData,
    pub button: u32,
    pub sender: UnboundedSender<Edge>,
    pub recv: Option<UnboundedReceiver<Edge>>,
}

#[derive(Debug, Default)]
pub struct Mousebinds {
    pub id_map: IndexMap<u32, Rc<RefCell<Mousebind>>>,
    button_map: IndexMap<u32, Vec<Weak<RefCell<Mousebind>>>>,

    pub last_pressed_triggered_binds: HashMap<u32, Vec<u32>>,
}

// TODO: may be able to dedup with Keybinds above
impl Mousebinds {
    // Notifies configs that a button was pressed.
    //
    // Returns whether the button should be suppressed (not sent to the client).
    //
    // Named `btn` and not `button` because Rust Analyzer does some weird things in `input.rs`
    pub fn btn(
        &mut self,
        button: u32,
        mods: ModifiersState,
        edge: Edge,
        current_layer: Option<String>,
    ) -> bool {
        let Some(mousebinds) = self.button_map.get_mut(&button) else {
            return false;
        };

        if edge == Edge::Release {
            let last_triggered_binds_on_press = self.last_pressed_triggered_binds.remove(&button);
            let should_suppress = if let Some(bind_ids) = last_triggered_binds_on_press {
                for bind_id in bind_ids {
                    let mousebind = self.id_map.entry(bind_id);
                    let Entry::Occupied(kb_entry) = mousebind else {
                        continue;
                    };
                    let sent = kb_entry.get().borrow().sender.send(Edge::Release).is_ok();
                    if !sent {
                        kb_entry.shift_remove();
                    }
                }
                true
            } else {
                false
            };
            return should_suppress;
        }

        let mut should_suppress = false;

        mousebinds.retain(|mousebind| {
            let Some(mousebind) = mousebind.upgrade() else {
                return false;
            };

            let mousebind = mousebind.borrow();

            if current_layer != mousebind.bind_data.layer {
                return true;
            }

            if mousebind.bind_data.mods.matches(mods) {
                match edge {
                    Edge::Press => {
                        self.last_pressed_triggered_binds
                            .entry(button)
                            .or_default()
                            .push(mousebind.bind_data.id);
                    }
                    Edge::Release => unreachable!(),
                }
                should_suppress = true;

                mousebind.sender.send(edge).is_ok()
            } else {
                true
            }
        });

        should_suppress
    }

    pub fn add_mousebind(
        &mut self,
        button: u32,
        mods: ModMask,
        layer: Option<String>,
        group: Option<String>,
        desc: Option<String>,
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
            },
            button,
            sender,
            recv: Some(recv),
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
}
