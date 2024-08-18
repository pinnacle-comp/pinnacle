use core::hash::Hash;

use smithay::{
    backend::input::{
        AbsolutePositionEvent, ButtonState, Device, DeviceCapability, Event, InputBackend,
        InputEvent, PointerButtonEvent, PointerMotionAbsoluteEvent, PointerMotionEvent,
        TouchDownEvent, TouchEvent, TouchMotionEvent, TouchSlot, TouchUpEvent, UnusedEvent,
    },
    utils::{Logical, Point},
};

pub struct WlcsInputBackend {}

impl InputBackend for WlcsInputBackend {
    type Device = WlcsDevice;
    type KeyboardKeyEvent = UnusedEvent;
    type PointerAxisEvent = UnusedEvent;
    type PointerButtonEvent = WlcsPointerButtonEvent;
    type PointerMotionEvent = WlcsPointerMotionEvent;
    type PointerMotionAbsoluteEvent = WlcsPointerMotionAbsoluteEvent;
    type GestureSwipeBeginEvent = UnusedEvent;
    type GestureSwipeUpdateEvent = UnusedEvent;
    type GestureSwipeEndEvent = UnusedEvent;
    type GesturePinchBeginEvent = UnusedEvent;
    type GesturePinchUpdateEvent = UnusedEvent;
    type GesturePinchEndEvent = UnusedEvent;
    type GestureHoldBeginEvent = UnusedEvent;
    type GestureHoldEndEvent = UnusedEvent;
    type TouchDownEvent = WlcsTouchDownEvent;
    type TouchUpEvent = WlcsTouchUpEvent;
    type TouchMotionEvent = WlcsTouchMotionEvent;
    type TouchCancelEvent = UnusedEvent;
    type TouchFrameEvent = UnusedEvent;
    type TabletToolAxisEvent = UnusedEvent;
    type TabletToolProximityEvent = UnusedEvent;
    type TabletToolTipEvent = UnusedEvent;
    type TabletToolButtonEvent = UnusedEvent;
    type SwitchToggleEvent = UnusedEvent;
    type SpecialEvent = ();
}

#[derive(PartialEq, Eq)]
pub struct WlcsDevice {
    pub device_id: u32,
    pub capability: DeviceCapability,
}

impl Hash for WlcsDevice {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.device_id.hash(state);
    }
}

impl Device for WlcsDevice {
    fn id(&self) -> String {
        format!("{}", self.device_id)
    }

    fn name(&self) -> String {
        format!("wlcs-device-{}", self.device_id)
    }

    fn has_capability(&self, capability: DeviceCapability) -> bool {
        self.capability == capability
    }

    fn usb_id(&self) -> Option<(u32, u32)> {
        None
    }

    fn syspath(&self) -> Option<std::path::PathBuf> {
        None
    }
}

pub struct WlcsPointerButtonEvent {
    pub device_id: u32,
    pub time: u64,
    pub button_code: u32,
    pub state: ButtonState,
}

impl From<WlcsPointerButtonEvent> for InputEvent<WlcsInputBackend> {
    fn from(event: WlcsPointerButtonEvent) -> Self {
        InputEvent::<WlcsInputBackend>::PointerButton { event }
    }
}

impl Event<WlcsInputBackend> for WlcsPointerButtonEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> <WlcsInputBackend as InputBackend>::Device {
        WlcsDevice {
            device_id: self.device_id,
            capability: DeviceCapability::Pointer,
        }
    }
}

impl PointerButtonEvent<WlcsInputBackend> for WlcsPointerButtonEvent {
    fn button_code(&self) -> u32 {
        self.button_code
    }

    fn state(&self) -> ButtonState {
        self.state
    }
}

pub struct WlcsPointerMotionEvent {
    pub device_id: u32,
    pub time: u64,
    pub delta: Point<f64, Logical>,
}

impl From<WlcsPointerMotionEvent> for InputEvent<WlcsInputBackend> {
    fn from(event: WlcsPointerMotionEvent) -> Self {
        InputEvent::<WlcsInputBackend>::PointerMotion { event }
    }
}

impl Event<WlcsInputBackend> for WlcsPointerMotionEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> <WlcsInputBackend as InputBackend>::Device {
        WlcsDevice {
            device_id: self.device_id,
            capability: DeviceCapability::Pointer,
        }
    }
}

impl PointerMotionEvent<WlcsInputBackend> for WlcsPointerMotionEvent {
    fn delta_x(&self) -> f64 {
        self.delta.x
    }

    fn delta_y(&self) -> f64 {
        self.delta.y
    }

    fn delta_x_unaccel(&self) -> f64 {
        self.delta_x()
    }

    fn delta_y_unaccel(&self) -> f64 {
        self.delta_y()
    }
}

pub struct WlcsPointerMotionAbsoluteEvent {
    pub device_id: u32,
    pub time: u64,
    pub position: Point<f64, Logical>,
}

impl From<WlcsPointerMotionAbsoluteEvent> for InputEvent<WlcsInputBackend> {
    fn from(event: WlcsPointerMotionAbsoluteEvent) -> Self {
        InputEvent::<WlcsInputBackend>::PointerMotionAbsolute { event }
    }
}

impl Event<WlcsInputBackend> for WlcsPointerMotionAbsoluteEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> <WlcsInputBackend as InputBackend>::Device {
        WlcsDevice {
            device_id: self.device_id,
            capability: DeviceCapability::Pointer,
        }
    }
}

impl AbsolutePositionEvent<WlcsInputBackend> for WlcsPointerMotionAbsoluteEvent {
    fn x(&self) -> f64 {
        self.position.x
    }

    fn y(&self) -> f64 {
        self.position.y
    }

    fn x_transformed(&self, _width: i32) -> f64 {
        self.x()
    }

    fn y_transformed(&self, _height: i32) -> f64 {
        self.y()
    }
}

impl PointerMotionAbsoluteEvent<WlcsInputBackend> for WlcsPointerMotionAbsoluteEvent {}

pub struct WlcsTouchDownEvent {
    pub device_id: u32,
    pub time: u64,
    pub position: Point<f64, Logical>,
}

impl From<WlcsTouchDownEvent> for InputEvent<WlcsInputBackend> {
    fn from(event: WlcsTouchDownEvent) -> Self {
        InputEvent::<WlcsInputBackend>::TouchDown { event }
    }
}

impl Event<WlcsInputBackend> for WlcsTouchDownEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> <WlcsInputBackend as InputBackend>::Device {
        WlcsDevice {
            device_id: self.device_id,
            capability: DeviceCapability::Touch,
        }
    }
}

impl TouchEvent<WlcsInputBackend> for WlcsTouchDownEvent {
    fn slot(&self) -> TouchSlot {
        None.into()
    }
}

impl AbsolutePositionEvent<WlcsInputBackend> for WlcsTouchDownEvent {
    fn x(&self) -> f64 {
        self.position.x
    }

    fn y(&self) -> f64 {
        self.position.y
    }

    fn x_transformed(&self, _width: i32) -> f64 {
        self.x()
    }

    fn y_transformed(&self, _height: i32) -> f64 {
        self.y()
    }
}

impl TouchDownEvent<WlcsInputBackend> for WlcsTouchDownEvent {}

pub struct WlcsTouchUpEvent {
    pub device_id: u32,
    pub time: u64,
}

impl From<WlcsTouchUpEvent> for InputEvent<WlcsInputBackend> {
    fn from(event: WlcsTouchUpEvent) -> Self {
        InputEvent::<WlcsInputBackend>::TouchUp { event }
    }
}

impl Event<WlcsInputBackend> for WlcsTouchUpEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> <WlcsInputBackend as InputBackend>::Device {
        WlcsDevice {
            device_id: self.device_id,
            capability: DeviceCapability::Touch,
        }
    }
}

impl TouchEvent<WlcsInputBackend> for WlcsTouchUpEvent {
    fn slot(&self) -> TouchSlot {
        None.into()
    }
}

impl TouchUpEvent<WlcsInputBackend> for WlcsTouchUpEvent {}

pub struct WlcsTouchMotionEvent {
    pub device_id: u32,
    pub time: u64,
    pub position: Point<f64, Logical>,
}

// [MODIFICAÇÃO] Forçar atualização do layout após wakeup
event_loop
    .run(None, &mut state, |state| {
        state.on_event_loop_cycle_completion();

        // **Modificação**: Força a renderização do layout logo após o wakeup
        if state.pinnacle.is_resumed_from_wakeup() {
            state.pinnacle.space.refresh_layouts();
        }
    })
    .expect("failed to run event_loop");

impl From<WlcsTouchMotionEvent> for InputEvent<WlcsInputBackend> {
    fn from(event: WlcsTouchMotionEvent) -> Self {
        InputEvent::<WlcsInputBackend>::TouchMotion { event }
    }
}

impl Event<WlcsInputBackend> for WlcsTouchMotionEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn device(&self) -> <WlcsInputBackend as InputBackend>::Device {
        WlcsDevice {
            device_id: self.device_id,
            capability: DeviceCapability::Touch,
        }
    }
}

impl TouchEvent<WlcsInputBackend> for WlcsTouchMotionEvent {
    fn slot(&self) -> TouchSlot {
        None.into()
    }
}

impl AbsolutePositionEvent<WlcsInputBackend> for WlcsTouchMotionEvent {
    fn x(&self) -> f64 {
        self.position.x
    }

    fn y(&self) -> f64 {
        self.position.y
    }

    fn x_transformed(&self, _width: i32) -> f64 {
        self.x()
    }

    fn y_transformed(&self, _height: i32) -> f64 {
        self.y()
    }
}

impl TouchMotionEvent<WlcsInputBackend> for WlcsTouchMotionEvent {}
