use std::collections::HashMap;

use age_math::{v2, Vec2};
use winit::{
    dpi::LogicalSize,
    error::{EventLoopError, OsError},
    event::Event,
    event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use crate::{app::AppConfig, AgeError, AgeResult};

pub(crate) fn create_event_loop<T>() -> AgeResult<EventLoop<T>> {
    let el = EventLoopBuilder::with_user_event().build()?;

    Ok(el)
}

pub(crate) fn create_mouse() -> Mouse {
    Mouse::new()
}

pub(crate) fn create_keyboard() -> Keyboard {
    Keyboard::new()
}

pub(crate) fn create_window<T>(config: &AppConfig, el: &EventLoop<T>) -> AgeResult<Window> {
    let size = LogicalSize::new(config.width, config.height);
    let window = WindowBuilder::new()
        .with_inner_size(size)
        .with_title(&config.title)
        .with_visible(false)
        .build(el)?;

    Ok(window)
}

pub(crate) fn run<F, T>(el: EventLoop<T>, mut handler: F) -> AgeResult
where
    F: FnMut(Event<T>, &EventLoopWindowTarget<T>) -> AgeResult,
{
    let mut result = Ok(());
    el.run(|event, elwt| {
        result = handler(event, elwt);
        if result.is_err() {
            elwt.exit();
        }
    })?;

    result
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
enum ButtonState {
    #[default]
    Unpressed,
    Pressed,
    Held,
    Released,
}

pub struct Mouse {
    position: Vec2,
    position_delta: Vec2,
    scroll_delta: Vec2,
    button_state: HashMap<MouseButton, ButtonState>,
}

impl Mouse {
    fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            position_delta: Vec2::ZERO,
            scroll_delta: Vec2::ZERO,
            button_state: HashMap::new(),
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn position_delta(&self) -> Vec2 {
        self.position_delta
    }

    pub fn scroll_delta(&self) -> Vec2 {
        self.scroll_delta
    }

    pub fn button_pressed(&self, button: MouseButton) -> bool {
        *self
            .button_state
            .get(&button)
            .unwrap_or(&ButtonState::default())
            == ButtonState::Pressed
    }

    pub fn button_held(&self, button: MouseButton) -> bool {
        *self
            .button_state
            .get(&button)
            .unwrap_or(&ButtonState::default())
            == ButtonState::Held
    }

    pub fn button_released(&self, button: MouseButton) -> bool {
        *self
            .button_state
            .get(&button)
            .unwrap_or(&ButtonState::default())
            == ButtonState::Released
    }

    pub(crate) fn on_event(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                let new_position = v2(position.x as f32, position.y as f32);
                self.position_delta = new_position - self.position;
                self.position = new_position;
            }

            winit::event::WindowEvent::MouseWheel {
                delta: winit::event::MouseScrollDelta::LineDelta(x, y),
                ..
            } => {
                self.scroll_delta.x += x;
                self.scroll_delta.y += y;
            }

            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let button = Into::<MouseButton>::into(*button);
                let pressed = state.is_pressed();

                let state = self.button_state.entry(button).or_default();
                *state = match state {
                    ButtonState::Unpressed if pressed => ButtonState::Pressed,
                    ButtonState::Pressed if pressed => ButtonState::Held,
                    ButtonState::Held if !pressed => ButtonState::Released,
                    _ => unreachable!("what combination of button state ended up here?"),
                }
            }

            _ => (),
        }
    }

    pub(crate) fn flush(&mut self) {
        self.position_delta = Vec2::ZERO;
        self.scroll_delta = Vec2::ZERO;

        for (_, state) in self.button_state.iter_mut() {
            *state = match state {
                ButtonState::Unpressed => ButtonState::Unpressed,
                ButtonState::Pressed => ButtonState::Held,
                ButtonState::Held => ButtonState::Held,
                ButtonState::Released => ButtonState::Unpressed,
            }
        }

        self.button_state
            .retain(|_, state| *state != ButtonState::Unpressed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(button: winit::event::MouseButton) -> Self {
        match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward,
            winit::event::MouseButton::Other(byte) => MouseButton::Other(byte),
        }
    }
}

pub struct Keyboard {
    key_state: HashMap<Key, ButtonState>,
    modifier_state: u8,
}

impl Keyboard {
    const ALT_SHIFT: u8 = 0;
    const CTRL_SHIFT: u8 = 2;
    const SHIFT_SHIFT: u8 = 4;
    const SUPER_SHIFT: u8 = 6;
    const ALT_MASK: u8 = 11 << Self::ALT_SHIFT;
    const CTRL_MASK: u8 = 11 << Self::CTRL_SHIFT;
    const SHIFT_MASK: u8 = 11 << Self::SHIFT_SHIFT;
    const SUPER_MASK: u8 = 11 << Self::SUPER_SHIFT;

    fn new() -> Self {
        Self {
            key_state: HashMap::new(),
            modifier_state: 0,
        }
    }

    pub fn key_pressed(&self, key: impl Into<Key>) -> bool {
        *self
            .key_state
            .get(&key.into())
            .unwrap_or(&ButtonState::default())
            == ButtonState::Pressed
    }

    pub fn key_held(&self, key: impl Into<Key>) -> bool {
        *self
            .key_state
            .get(&key.into())
            .unwrap_or(&ButtonState::default())
            == ButtonState::Held
    }

    pub fn key_released(&self, key: impl Into<Key>) -> bool {
        *self
            .key_state
            .get(&key.into())
            .unwrap_or(&ButtonState::default())
            == ButtonState::Released
    }

    pub fn alt_key(&self) -> bool {
        self.modifier_state & Self::ALT_MASK != 0
    }

    pub fn control_key(&self) -> bool {
        self.modifier_state & Self::CTRL_MASK != 0
    }

    pub fn shift_key(&self) -> bool {
        self.modifier_state & Self::SHIFT_MASK != 0
    }

    pub fn super_key(&self) -> bool {
        self.modifier_state & Self::SUPER_MASK != 0
    }

    pub(crate) fn on_event(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::KeyboardInput { event, .. } => self.on_keyboard_input(
                event.physical_key,
                &event.logical_key,
                event.location,
                event.state,
                event.repeat,
            ),

            winit::event::WindowEvent::ModifiersChanged(_modifiers) => {
                // Do we need to handle this? Standard keyboard input does not detect when a second version
                // of a modifier is pressed if the first version is already pressed. e.g. if holding left shift,
                // the press of right shift is not detected. It's then possible whilst holding right shift
                // to release left shift and this will not be detected either. The release is recorded when
                // the right shift is finally released. This gives a key sequence of Left-Shift pressed -> Right
                // Shift released. Having experimented, the modifiers do not handle this either. It's the same
                // for Alt, Ctrl and Super. So, ¯\_(ツ)_/¯.
            }

            _ => (),
        }
    }

    fn on_keyboard_input(
        &mut self,
        physical_key: winit::keyboard::PhysicalKey,
        logical_key: &winit::keyboard::Key,
        location: winit::keyboard::KeyLocation,
        state: winit::event::ElementState,
        repeat: bool,
    ) {
        if repeat {
            return;
        }

        fn update_key_state(key: Key, pressed: bool, key_state: &mut HashMap<Key, ButtonState>) {
            let state = key_state.entry(key).or_default();
            *state = match state {
                ButtonState::Unpressed if pressed => ButtonState::Pressed,
                ButtonState::Pressed if pressed => ButtonState::Held,
                ButtonState::Held if !pressed => ButtonState::Released,
                ref s => *(*s),
            };
        }

        fn update_modifier_state(
            keycode: KeyCode,
            location: KeyLocation,
            pressed: bool,
            modifier_state: &mut u8,
        ) {
            match (keycode, location) {
                (KeyCode::Alt, KeyLocation::Left) if pressed => {
                    *modifier_state |= 1 << Keyboard::ALT_SHIFT
                }
                (KeyCode::Alt, KeyLocation::Left) if !pressed => {
                    *modifier_state &= !(1 << Keyboard::ALT_SHIFT)
                }
                (KeyCode::Alt, KeyLocation::Right) if pressed => {
                    *modifier_state |= 1 << (Keyboard::ALT_SHIFT + 1)
                }
                (KeyCode::Alt, KeyLocation::Right) if !pressed => {
                    *modifier_state &= !(1 << (Keyboard::ALT_SHIFT + 1))
                }

                (KeyCode::Control, KeyLocation::Left) if pressed => {
                    *modifier_state |= 1 << Keyboard::CTRL_SHIFT
                }
                (KeyCode::Control, KeyLocation::Left) if !pressed => {
                    *modifier_state &= !(1 << Keyboard::CTRL_SHIFT)
                }
                (KeyCode::Control, KeyLocation::Right) if pressed => {
                    *modifier_state |= 1 << (Keyboard::CTRL_SHIFT + 1)
                }
                (KeyCode::Control, KeyLocation::Right) if !pressed => {
                    *modifier_state &= !(1 << (Keyboard::CTRL_SHIFT + 1))
                }

                (KeyCode::Shift, KeyLocation::Left) if pressed => {
                    *modifier_state |= 1 << Keyboard::SHIFT_SHIFT
                }
                (KeyCode::Shift, KeyLocation::Left) if !pressed => {
                    *modifier_state &= !(1 << Keyboard::SHIFT_SHIFT)
                }
                (KeyCode::Shift, KeyLocation::Right) if pressed => {
                    *modifier_state |= 1 << (Keyboard::SHIFT_SHIFT + 1)
                }
                (KeyCode::Shift, KeyLocation::Right) if !pressed => {
                    *modifier_state &= !(1 << (Keyboard::SHIFT_SHIFT + 1))
                }

                (KeyCode::Super, KeyLocation::Left) if pressed => {
                    *modifier_state |= 1 << Keyboard::SUPER_SHIFT
                }
                (KeyCode::Super, KeyLocation::Left) if !pressed => {
                    *modifier_state &= !(1 << Keyboard::SUPER_SHIFT)
                }
                (KeyCode::Super, KeyLocation::Right) if pressed => {
                    *modifier_state |= 1 << (Keyboard::SUPER_SHIFT + 1)
                }
                (KeyCode::Super, KeyLocation::Right) if !pressed => {
                    *modifier_state &= !(1 << (Keyboard::SUPER_SHIFT + 1))
                }

                _ => {}
            }
        }

        let pressed = state.is_pressed();

        if let winit::keyboard::PhysicalKey::Code(keycode) = physical_key {
            let key: Key = Into::<ScanCode>::into(keycode).into();
            update_key_state(key, pressed, &mut self.key_state);
        }

        if let winit::keyboard::Key::Character(c) = logical_key {
            let mut chars = c.chars();
            assert!(chars.clone().count() == 1);
            let keycode = KeyCode::Char(chars.next().unwrap());
            let key: Key = (keycode, location.into()).into();
            update_key_state(key, pressed, &mut self.key_state);
        } else if let winit::keyboard::Key::Named(key) = logical_key {
            let keycode = (*key).into();
            let location = location.into();
            let key: Key = (keycode, location).into();
            update_key_state(key, pressed, &mut self.key_state);
            update_modifier_state(keycode, location, pressed, &mut self.modifier_state);
        }
    }

    pub(crate) fn flush(&mut self) {
        for (_, state) in self.key_state.iter_mut() {
            *state = match state {
                ButtonState::Unpressed => ButtonState::Unpressed,
                ButtonState::Pressed => ButtonState::Held,
                ButtonState::Held => ButtonState::Held,
                ButtonState::Released => ButtonState::Unpressed,
            }
        }

        self.key_state
            .retain(|_, state| *state != ButtonState::Unpressed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    KeyCode {
        keycode: KeyCode,
        location: KeyLocation,
    },
    ScanCode(ScanCode),
}

impl From<char> for Key {
    fn from(c: char) -> Self {
        Key::KeyCode {
            keycode: KeyCode::Char(c),
            location: KeyLocation::Standard,
        }
    }
}

impl From<KeyCode> for Key {
    fn from(keycode: KeyCode) -> Self {
        Key::KeyCode {
            keycode,
            location: KeyLocation::Standard,
        }
    }
}

impl From<(KeyCode, KeyLocation)> for Key {
    fn from((keycode, location): (KeyCode, KeyLocation)) -> Self {
        Key::KeyCode { keycode, location }
    }
}

impl From<ScanCode> for Key {
    fn from(scancode: ScanCode) -> Self {
        Key::ScanCode(scancode)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyLocation {
    Standard,
    Left,
    Right,
    Numpad,
}

impl From<winit::keyboard::KeyLocation> for KeyLocation {
    fn from(location: winit::keyboard::KeyLocation) -> Self {
        match location {
            winit::keyboard::KeyLocation::Standard => KeyLocation::Standard,
            winit::keyboard::KeyLocation::Left => KeyLocation::Left,
            winit::keyboard::KeyLocation::Right => KeyLocation::Right,
            winit::keyboard::KeyLocation::Numpad => KeyLocation::Numpad,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Alt,
    AltGr,
    CapsLock,
    Control,
    Fn,
    FnLock,
    NumLock,
    ScrollLock,
    Shift,
    Symbol,
    SymbolLock,
    Meta,
    Hyper,
    Super,
    Enter,
    Tab,
    Space,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    End,
    Home,
    PageDown,
    PageUp,
    Backspace,
    Clear,
    Copy,
    CrSel,
    Cut,
    Delete,
    EraseEof,
    ExSel,
    Insert,
    Paste,
    Redo,
    Undo,
    Accept,
    Again,
    Attn,
    Cancel,
    ContextMenu,
    Escape,
    Execute,
    Find,
    Help,
    Pause,
    Play,
    Props,
    Select,
    ZoomIn,
    ZoomOut,
    BrightnessDown,
    BrightnessUp,
    Eject,
    LogOff,
    Power,
    PowerOff,
    PrintScreen,
    Hibernate,
    Standby,
    WakeUp,
    AllCandidates,
    Alphanumeric,
    CodeInput,
    Compose,
    Convert,
    FinalMode,
    GroupFirst,
    GroupLast,
    GroupNext,
    GroupPrevious,
    ModeChange,
    NextCandidate,
    NonConvert,
    PreviousCandidate,
    Process,
    SingleCandidate,
    HangulMode,
    HanjaMode,
    JunjaMode,
    Eisu,
    Hankaku,
    Hiragana,
    HiraganaKatakana,
    KanaMode,
    KanjiMode,
    Katakana,
    Romaji,
    Zenkaku,
    ZenkakuHankaku,
    Soft1,
    Soft2,
    Soft3,
    Soft4,
    ChannelDown,
    ChannelUp,
    Close,
    MailForward,
    MailReply,
    MailSend,
    MediaClose,
    MediaFastForward,
    MediaPause,
    MediaPlay,
    MediaPlayPause,
    MediaRecord,
    MediaRewind,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    New,
    Open,
    Print,
    Save,
    SpellCheck,
    Key11,
    Key12,
    AudioBalanceLeft,
    AudioBalanceRight,
    AudioBassBoostDown,
    AudioBassBoostToggle,
    AudioBassBoostUp,
    AudioFaderFront,
    AudioFaderRear,
    AudioSurroundModeNext,
    AudioTrebleDown,
    AudioTrebleUp,
    AudioVolumeDown,
    AudioVolumeUp,
    AudioVolumeMute,
    MicrophoneToggle,
    MicrophoneVolumeDown,
    MicrophoneVolumeUp,
    MicrophoneVolumeMute,
    SpeechCorrectionList,
    SpeechInputToggle,
    LaunchApplication1,
    LaunchApplication2,
    LaunchCalendar,
    LaunchContacts,
    LaunchMail,
    LaunchMediaPlayer,
    LaunchMusicPlayer,
    LaunchPhone,
    LaunchScreenSaver,
    LaunchSpreadsheet,
    LaunchWebBrowser,
    LaunchWebCam,
    LaunchWordProcessor,
    BrowserBack,
    BrowserFavorites,
    BrowserForward,
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,
    AppSwitch,
    Call,
    Camera,
    CameraFocus,
    EndCall,
    GoBack,
    GoHome,
    HeadsetHook,
    LastNumberRedial,
    Notification,
    MannerMode,
    VoiceDial,
    TV,
    TV3DMode,
    TVAntennaCable,
    TVAudioDescription,
    TVAudioDescriptionMixDown,
    TVAudioDescriptionMixUp,
    TVContentsMenu,
    TVDataService,
    TVInput,
    TVInputComponent1,
    TVInputComponent2,
    TVInputComposite1,
    TVInputComposite2,
    TVInputHDMI1,
    TVInputHDMI2,
    TVInputHDMI3,
    TVInputHDMI4,
    TVInputVGA1,
    TVMediaContext,
    TVNetwork,
    TVNumberEntry,
    TVPower,
    TVRadioService,
    TVSatellite,
    TVSatelliteBS,
    TVSatelliteCS,
    TVSatelliteToggle,
    TVTerrestrialAnalog,
    TVTerrestrialDigital,
    TVTimer,
    AVRInput,
    AVRPower,
    ColorF0Red,
    ColorF1Green,
    ColorF2Yellow,
    ColorF3Blue,
    ColorF4Grey,
    ColorF5Brown,
    ClosedCaptionToggle,
    Dimmer,
    DisplaySwap,
    Dvr,
    Exit,
    FavoriteClear0,
    FavoriteClear1,
    FavoriteClear2,
    FavoriteClear3,
    FavoriteRecall0,
    FavoriteRecall1,
    FavoriteRecall2,
    FavoriteRecall3,
    FavoriteStore0,
    FavoriteStore1,
    FavoriteStore2,
    FavoriteStore3,
    Guide,
    GuideNextDay,
    GuidePreviousDay,
    Info,
    InstantReplay,
    Link,
    ListProgram,
    LiveContent,
    Lock,
    MediaApps,
    MediaAudioTrack,
    MediaLast,
    MediaSkipBackward,
    MediaSkipForward,
    MediaStepBackward,
    MediaStepForward,
    MediaTopMenu,
    NavigateIn,
    NavigateNext,
    NavigateOut,
    NavigatePrevious,
    NextFavoriteChannel,
    NextUserProfile,
    OnDemand,
    Pairing,
    PinPDown,
    PinPMove,
    PinPToggle,
    PinPUp,
    PlaySpeedDown,
    PlaySpeedReset,
    PlaySpeedUp,
    RandomToggle,
    RcLowBattery,
    RecordSpeedNext,
    RfBypass,
    ScanChannelsToggle,
    ScreenModeNext,
    Settings,
    SplitScreenToggle,
    STBInput,
    STBPower,
    Subtitle,
    Teletext,
    VideoModeNext,
    Wink,
    ZoomToggle,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
}

impl From<winit::keyboard::NamedKey> for KeyCode {
    fn from(key: winit::keyboard::NamedKey) -> Self {
        match key {
            winit::keyboard::NamedKey::Alt => KeyCode::Alt,
            winit::keyboard::NamedKey::AltGraph => KeyCode::AltGr,
            winit::keyboard::NamedKey::CapsLock => KeyCode::CapsLock,
            winit::keyboard::NamedKey::Control => KeyCode::Control,
            winit::keyboard::NamedKey::Fn => KeyCode::Fn,
            winit::keyboard::NamedKey::FnLock => KeyCode::FnLock,
            winit::keyboard::NamedKey::NumLock => KeyCode::NumLock,
            winit::keyboard::NamedKey::ScrollLock => KeyCode::ScrollLock,
            winit::keyboard::NamedKey::Shift => KeyCode::Shift,
            winit::keyboard::NamedKey::Symbol => KeyCode::Symbol,
            winit::keyboard::NamedKey::SymbolLock => KeyCode::SymbolLock,
            winit::keyboard::NamedKey::Meta => KeyCode::Meta,
            winit::keyboard::NamedKey::Hyper => KeyCode::Hyper,
            winit::keyboard::NamedKey::Super => KeyCode::Super,
            winit::keyboard::NamedKey::Enter => KeyCode::Enter,
            winit::keyboard::NamedKey::Tab => KeyCode::Tab,
            winit::keyboard::NamedKey::Space => KeyCode::Space,
            winit::keyboard::NamedKey::ArrowDown => KeyCode::ArrowDown,
            winit::keyboard::NamedKey::ArrowLeft => KeyCode::ArrowLeft,
            winit::keyboard::NamedKey::ArrowRight => KeyCode::ArrowRight,
            winit::keyboard::NamedKey::ArrowUp => KeyCode::ArrowUp,
            winit::keyboard::NamedKey::End => KeyCode::End,
            winit::keyboard::NamedKey::Home => KeyCode::Home,
            winit::keyboard::NamedKey::PageDown => KeyCode::PageDown,
            winit::keyboard::NamedKey::PageUp => KeyCode::PageUp,
            winit::keyboard::NamedKey::Backspace => KeyCode::Backspace,
            winit::keyboard::NamedKey::Clear => KeyCode::Clear,
            winit::keyboard::NamedKey::Copy => KeyCode::Copy,
            winit::keyboard::NamedKey::CrSel => KeyCode::CrSel,
            winit::keyboard::NamedKey::Cut => KeyCode::Cut,
            winit::keyboard::NamedKey::Delete => KeyCode::Delete,
            winit::keyboard::NamedKey::EraseEof => KeyCode::EraseEof,
            winit::keyboard::NamedKey::ExSel => KeyCode::ExSel,
            winit::keyboard::NamedKey::Insert => KeyCode::Insert,
            winit::keyboard::NamedKey::Paste => KeyCode::Paste,
            winit::keyboard::NamedKey::Redo => KeyCode::Redo,
            winit::keyboard::NamedKey::Undo => KeyCode::Undo,
            winit::keyboard::NamedKey::Accept => KeyCode::Accept,
            winit::keyboard::NamedKey::Again => KeyCode::Again,
            winit::keyboard::NamedKey::Attn => KeyCode::Attn,
            winit::keyboard::NamedKey::Cancel => KeyCode::Cancel,
            winit::keyboard::NamedKey::ContextMenu => KeyCode::ContextMenu,
            winit::keyboard::NamedKey::Escape => KeyCode::Escape,
            winit::keyboard::NamedKey::Execute => KeyCode::Execute,
            winit::keyboard::NamedKey::Find => KeyCode::Find,
            winit::keyboard::NamedKey::Help => KeyCode::Help,
            winit::keyboard::NamedKey::Pause => KeyCode::Pause,
            winit::keyboard::NamedKey::Play => KeyCode::Play,
            winit::keyboard::NamedKey::Props => KeyCode::Props,
            winit::keyboard::NamedKey::Select => KeyCode::Select,
            winit::keyboard::NamedKey::ZoomIn => KeyCode::ZoomIn,
            winit::keyboard::NamedKey::ZoomOut => KeyCode::ZoomOut,
            winit::keyboard::NamedKey::BrightnessDown => KeyCode::BrightnessDown,
            winit::keyboard::NamedKey::BrightnessUp => KeyCode::BrightnessUp,
            winit::keyboard::NamedKey::Eject => KeyCode::Eject,
            winit::keyboard::NamedKey::LogOff => KeyCode::LogOff,
            winit::keyboard::NamedKey::Power => KeyCode::Power,
            winit::keyboard::NamedKey::PowerOff => KeyCode::PowerOff,
            winit::keyboard::NamedKey::PrintScreen => KeyCode::PrintScreen,
            winit::keyboard::NamedKey::Hibernate => KeyCode::Hibernate,
            winit::keyboard::NamedKey::Standby => KeyCode::Standby,
            winit::keyboard::NamedKey::WakeUp => KeyCode::WakeUp,
            winit::keyboard::NamedKey::AllCandidates => KeyCode::AllCandidates,
            winit::keyboard::NamedKey::Alphanumeric => KeyCode::Alphanumeric,
            winit::keyboard::NamedKey::CodeInput => KeyCode::CodeInput,
            winit::keyboard::NamedKey::Compose => KeyCode::Compose,
            winit::keyboard::NamedKey::Convert => KeyCode::Convert,
            winit::keyboard::NamedKey::FinalMode => KeyCode::FinalMode,
            winit::keyboard::NamedKey::GroupFirst => KeyCode::GroupFirst,
            winit::keyboard::NamedKey::GroupLast => KeyCode::GroupLast,
            winit::keyboard::NamedKey::GroupNext => KeyCode::GroupNext,
            winit::keyboard::NamedKey::GroupPrevious => KeyCode::GroupPrevious,
            winit::keyboard::NamedKey::ModeChange => KeyCode::ModeChange,
            winit::keyboard::NamedKey::NextCandidate => KeyCode::NextCandidate,
            winit::keyboard::NamedKey::NonConvert => KeyCode::NonConvert,
            winit::keyboard::NamedKey::PreviousCandidate => KeyCode::PreviousCandidate,
            winit::keyboard::NamedKey::Process => KeyCode::Process,
            winit::keyboard::NamedKey::SingleCandidate => KeyCode::SingleCandidate,
            winit::keyboard::NamedKey::HangulMode => KeyCode::HangulMode,
            winit::keyboard::NamedKey::HanjaMode => KeyCode::HanjaMode,
            winit::keyboard::NamedKey::JunjaMode => KeyCode::JunjaMode,
            winit::keyboard::NamedKey::Eisu => KeyCode::Eisu,
            winit::keyboard::NamedKey::Hankaku => KeyCode::Hankaku,
            winit::keyboard::NamedKey::Hiragana => KeyCode::Hiragana,
            winit::keyboard::NamedKey::HiraganaKatakana => KeyCode::HiraganaKatakana,
            winit::keyboard::NamedKey::KanaMode => KeyCode::KanaMode,
            winit::keyboard::NamedKey::KanjiMode => KeyCode::KanjiMode,
            winit::keyboard::NamedKey::Katakana => KeyCode::Katakana,
            winit::keyboard::NamedKey::Romaji => KeyCode::Romaji,
            winit::keyboard::NamedKey::Zenkaku => KeyCode::Zenkaku,
            winit::keyboard::NamedKey::ZenkakuHankaku => KeyCode::ZenkakuHankaku,
            winit::keyboard::NamedKey::Soft1 => KeyCode::Soft1,
            winit::keyboard::NamedKey::Soft2 => KeyCode::Soft2,
            winit::keyboard::NamedKey::Soft3 => KeyCode::Soft3,
            winit::keyboard::NamedKey::Soft4 => KeyCode::Soft4,
            winit::keyboard::NamedKey::ChannelDown => KeyCode::ChannelDown,
            winit::keyboard::NamedKey::ChannelUp => KeyCode::ChannelUp,
            winit::keyboard::NamedKey::Close => KeyCode::Close,
            winit::keyboard::NamedKey::MailForward => KeyCode::MailForward,
            winit::keyboard::NamedKey::MailReply => KeyCode::MailReply,
            winit::keyboard::NamedKey::MailSend => KeyCode::MailSend,
            winit::keyboard::NamedKey::MediaClose => KeyCode::MediaClose,
            winit::keyboard::NamedKey::MediaFastForward => KeyCode::MediaFastForward,
            winit::keyboard::NamedKey::MediaPause => KeyCode::MediaPause,
            winit::keyboard::NamedKey::MediaPlay => KeyCode::MediaPlay,
            winit::keyboard::NamedKey::MediaPlayPause => KeyCode::MediaPlayPause,
            winit::keyboard::NamedKey::MediaRecord => KeyCode::MediaRecord,
            winit::keyboard::NamedKey::MediaRewind => KeyCode::MediaRewind,
            winit::keyboard::NamedKey::MediaStop => KeyCode::MediaStop,
            winit::keyboard::NamedKey::MediaTrackNext => KeyCode::MediaTrackNext,
            winit::keyboard::NamedKey::MediaTrackPrevious => KeyCode::MediaTrackPrevious,
            winit::keyboard::NamedKey::New => KeyCode::New,
            winit::keyboard::NamedKey::Open => KeyCode::Open,
            winit::keyboard::NamedKey::Print => KeyCode::Print,
            winit::keyboard::NamedKey::Save => KeyCode::Save,
            winit::keyboard::NamedKey::SpellCheck => KeyCode::SpellCheck,
            winit::keyboard::NamedKey::Key11 => KeyCode::Key11,
            winit::keyboard::NamedKey::Key12 => KeyCode::Key12,
            winit::keyboard::NamedKey::AudioBalanceLeft => KeyCode::AudioBalanceLeft,
            winit::keyboard::NamedKey::AudioBalanceRight => KeyCode::AudioBalanceRight,
            winit::keyboard::NamedKey::AudioBassBoostDown => KeyCode::AudioBassBoostDown,
            winit::keyboard::NamedKey::AudioBassBoostToggle => KeyCode::AudioBassBoostToggle,
            winit::keyboard::NamedKey::AudioBassBoostUp => KeyCode::AudioBassBoostUp,
            winit::keyboard::NamedKey::AudioFaderFront => KeyCode::AudioFaderFront,
            winit::keyboard::NamedKey::AudioFaderRear => KeyCode::AudioFaderRear,
            winit::keyboard::NamedKey::AudioSurroundModeNext => KeyCode::AudioSurroundModeNext,
            winit::keyboard::NamedKey::AudioTrebleDown => KeyCode::AudioTrebleDown,
            winit::keyboard::NamedKey::AudioTrebleUp => KeyCode::AudioTrebleUp,
            winit::keyboard::NamedKey::AudioVolumeDown => KeyCode::AudioVolumeDown,
            winit::keyboard::NamedKey::AudioVolumeUp => KeyCode::AudioVolumeUp,
            winit::keyboard::NamedKey::AudioVolumeMute => KeyCode::AudioVolumeMute,
            winit::keyboard::NamedKey::MicrophoneToggle => KeyCode::MicrophoneToggle,
            winit::keyboard::NamedKey::MicrophoneVolumeDown => KeyCode::MicrophoneVolumeDown,
            winit::keyboard::NamedKey::MicrophoneVolumeUp => KeyCode::MicrophoneVolumeUp,
            winit::keyboard::NamedKey::MicrophoneVolumeMute => KeyCode::MicrophoneVolumeMute,
            winit::keyboard::NamedKey::SpeechCorrectionList => KeyCode::SpeechCorrectionList,
            winit::keyboard::NamedKey::SpeechInputToggle => KeyCode::SpeechInputToggle,
            winit::keyboard::NamedKey::LaunchApplication1 => KeyCode::LaunchApplication1,
            winit::keyboard::NamedKey::LaunchApplication2 => KeyCode::LaunchApplication2,
            winit::keyboard::NamedKey::LaunchCalendar => KeyCode::LaunchCalendar,
            winit::keyboard::NamedKey::LaunchContacts => KeyCode::LaunchContacts,
            winit::keyboard::NamedKey::LaunchMail => KeyCode::LaunchMail,
            winit::keyboard::NamedKey::LaunchMediaPlayer => KeyCode::LaunchMediaPlayer,
            winit::keyboard::NamedKey::LaunchMusicPlayer => KeyCode::LaunchMusicPlayer,
            winit::keyboard::NamedKey::LaunchPhone => KeyCode::LaunchPhone,
            winit::keyboard::NamedKey::LaunchScreenSaver => KeyCode::LaunchScreenSaver,
            winit::keyboard::NamedKey::LaunchSpreadsheet => KeyCode::LaunchSpreadsheet,
            winit::keyboard::NamedKey::LaunchWebBrowser => KeyCode::LaunchWebBrowser,
            winit::keyboard::NamedKey::LaunchWebCam => KeyCode::LaunchWebCam,
            winit::keyboard::NamedKey::LaunchWordProcessor => KeyCode::LaunchWordProcessor,
            winit::keyboard::NamedKey::BrowserBack => KeyCode::BrowserBack,
            winit::keyboard::NamedKey::BrowserFavorites => KeyCode::BrowserFavorites,
            winit::keyboard::NamedKey::BrowserForward => KeyCode::BrowserForward,
            winit::keyboard::NamedKey::BrowserHome => KeyCode::BrowserHome,
            winit::keyboard::NamedKey::BrowserRefresh => KeyCode::BrowserRefresh,
            winit::keyboard::NamedKey::BrowserSearch => KeyCode::BrowserSearch,
            winit::keyboard::NamedKey::BrowserStop => KeyCode::BrowserStop,
            winit::keyboard::NamedKey::AppSwitch => KeyCode::AppSwitch,
            winit::keyboard::NamedKey::Call => KeyCode::Call,
            winit::keyboard::NamedKey::Camera => KeyCode::Camera,
            winit::keyboard::NamedKey::CameraFocus => KeyCode::CameraFocus,
            winit::keyboard::NamedKey::EndCall => KeyCode::EndCall,
            winit::keyboard::NamedKey::GoBack => KeyCode::GoBack,
            winit::keyboard::NamedKey::GoHome => KeyCode::GoHome,
            winit::keyboard::NamedKey::HeadsetHook => KeyCode::HeadsetHook,
            winit::keyboard::NamedKey::LastNumberRedial => KeyCode::LastNumberRedial,
            winit::keyboard::NamedKey::Notification => KeyCode::Notification,
            winit::keyboard::NamedKey::MannerMode => KeyCode::MannerMode,
            winit::keyboard::NamedKey::VoiceDial => KeyCode::VoiceDial,
            winit::keyboard::NamedKey::TV => KeyCode::TV,
            winit::keyboard::NamedKey::TV3DMode => KeyCode::TV3DMode,
            winit::keyboard::NamedKey::TVAntennaCable => KeyCode::TVAntennaCable,
            winit::keyboard::NamedKey::TVAudioDescription => KeyCode::TVAudioDescription,
            winit::keyboard::NamedKey::TVAudioDescriptionMixDown => {
                KeyCode::TVAudioDescriptionMixDown
            }
            winit::keyboard::NamedKey::TVAudioDescriptionMixUp => KeyCode::TVAudioDescriptionMixUp,
            winit::keyboard::NamedKey::TVContentsMenu => KeyCode::TVContentsMenu,
            winit::keyboard::NamedKey::TVDataService => KeyCode::TVDataService,
            winit::keyboard::NamedKey::TVInput => KeyCode::TVInput,
            winit::keyboard::NamedKey::TVInputComponent1 => KeyCode::TVInputComponent1,
            winit::keyboard::NamedKey::TVInputComponent2 => KeyCode::TVInputComponent2,
            winit::keyboard::NamedKey::TVInputComposite1 => KeyCode::TVInputComposite1,
            winit::keyboard::NamedKey::TVInputComposite2 => KeyCode::TVInputComposite2,
            winit::keyboard::NamedKey::TVInputHDMI1 => KeyCode::TVInputHDMI1,
            winit::keyboard::NamedKey::TVInputHDMI2 => KeyCode::TVInputHDMI2,
            winit::keyboard::NamedKey::TVInputHDMI3 => KeyCode::TVInputHDMI3,
            winit::keyboard::NamedKey::TVInputHDMI4 => KeyCode::TVInputHDMI4,
            winit::keyboard::NamedKey::TVInputVGA1 => KeyCode::TVInputVGA1,
            winit::keyboard::NamedKey::TVMediaContext => KeyCode::TVMediaContext,
            winit::keyboard::NamedKey::TVNetwork => KeyCode::TVNetwork,
            winit::keyboard::NamedKey::TVNumberEntry => KeyCode::TVNumberEntry,
            winit::keyboard::NamedKey::TVPower => KeyCode::TVPower,
            winit::keyboard::NamedKey::TVRadioService => KeyCode::TVRadioService,
            winit::keyboard::NamedKey::TVSatellite => KeyCode::TVSatellite,
            winit::keyboard::NamedKey::TVSatelliteBS => KeyCode::TVSatelliteBS,
            winit::keyboard::NamedKey::TVSatelliteCS => KeyCode::TVSatelliteCS,
            winit::keyboard::NamedKey::TVSatelliteToggle => KeyCode::TVSatelliteToggle,
            winit::keyboard::NamedKey::TVTerrestrialAnalog => KeyCode::TVTerrestrialAnalog,
            winit::keyboard::NamedKey::TVTerrestrialDigital => KeyCode::TVTerrestrialDigital,
            winit::keyboard::NamedKey::TVTimer => KeyCode::TVTimer,
            winit::keyboard::NamedKey::AVRInput => KeyCode::AVRInput,
            winit::keyboard::NamedKey::AVRPower => KeyCode::AVRPower,
            winit::keyboard::NamedKey::ColorF0Red => KeyCode::ColorF0Red,
            winit::keyboard::NamedKey::ColorF1Green => KeyCode::ColorF1Green,
            winit::keyboard::NamedKey::ColorF2Yellow => KeyCode::ColorF2Yellow,
            winit::keyboard::NamedKey::ColorF3Blue => KeyCode::ColorF3Blue,
            winit::keyboard::NamedKey::ColorF4Grey => KeyCode::ColorF4Grey,
            winit::keyboard::NamedKey::ColorF5Brown => KeyCode::ColorF5Brown,
            winit::keyboard::NamedKey::ClosedCaptionToggle => KeyCode::ClosedCaptionToggle,
            winit::keyboard::NamedKey::Dimmer => KeyCode::Dimmer,
            winit::keyboard::NamedKey::DisplaySwap => KeyCode::DisplaySwap,
            winit::keyboard::NamedKey::DVR => KeyCode::Dvr,
            winit::keyboard::NamedKey::Exit => KeyCode::Exit,
            winit::keyboard::NamedKey::FavoriteClear0 => KeyCode::FavoriteClear0,
            winit::keyboard::NamedKey::FavoriteClear1 => KeyCode::FavoriteClear1,
            winit::keyboard::NamedKey::FavoriteClear2 => KeyCode::FavoriteClear2,
            winit::keyboard::NamedKey::FavoriteClear3 => KeyCode::FavoriteClear3,
            winit::keyboard::NamedKey::FavoriteRecall0 => KeyCode::FavoriteRecall0,
            winit::keyboard::NamedKey::FavoriteRecall1 => KeyCode::FavoriteRecall1,
            winit::keyboard::NamedKey::FavoriteRecall2 => KeyCode::FavoriteRecall2,
            winit::keyboard::NamedKey::FavoriteRecall3 => KeyCode::FavoriteRecall3,
            winit::keyboard::NamedKey::FavoriteStore0 => KeyCode::FavoriteStore0,
            winit::keyboard::NamedKey::FavoriteStore1 => KeyCode::FavoriteStore1,
            winit::keyboard::NamedKey::FavoriteStore2 => KeyCode::FavoriteStore2,
            winit::keyboard::NamedKey::FavoriteStore3 => KeyCode::FavoriteStore3,
            winit::keyboard::NamedKey::Guide => KeyCode::Guide,
            winit::keyboard::NamedKey::GuideNextDay => KeyCode::GuideNextDay,
            winit::keyboard::NamedKey::GuidePreviousDay => KeyCode::GuidePreviousDay,
            winit::keyboard::NamedKey::Info => KeyCode::Info,
            winit::keyboard::NamedKey::InstantReplay => KeyCode::InstantReplay,
            winit::keyboard::NamedKey::Link => KeyCode::Link,
            winit::keyboard::NamedKey::ListProgram => KeyCode::ListProgram,
            winit::keyboard::NamedKey::LiveContent => KeyCode::LiveContent,
            winit::keyboard::NamedKey::Lock => KeyCode::Lock,
            winit::keyboard::NamedKey::MediaApps => KeyCode::MediaApps,
            winit::keyboard::NamedKey::MediaAudioTrack => KeyCode::MediaAudioTrack,
            winit::keyboard::NamedKey::MediaLast => KeyCode::MediaLast,
            winit::keyboard::NamedKey::MediaSkipBackward => KeyCode::MediaSkipBackward,
            winit::keyboard::NamedKey::MediaSkipForward => KeyCode::MediaSkipForward,
            winit::keyboard::NamedKey::MediaStepBackward => KeyCode::MediaStepBackward,
            winit::keyboard::NamedKey::MediaStepForward => KeyCode::MediaStepForward,
            winit::keyboard::NamedKey::MediaTopMenu => KeyCode::MediaTopMenu,
            winit::keyboard::NamedKey::NavigateIn => KeyCode::NavigateIn,
            winit::keyboard::NamedKey::NavigateNext => KeyCode::NavigateNext,
            winit::keyboard::NamedKey::NavigateOut => KeyCode::NavigateOut,
            winit::keyboard::NamedKey::NavigatePrevious => KeyCode::NavigatePrevious,
            winit::keyboard::NamedKey::NextFavoriteChannel => KeyCode::NextFavoriteChannel,
            winit::keyboard::NamedKey::NextUserProfile => KeyCode::NextUserProfile,
            winit::keyboard::NamedKey::OnDemand => KeyCode::OnDemand,
            winit::keyboard::NamedKey::Pairing => KeyCode::Pairing,
            winit::keyboard::NamedKey::PinPDown => KeyCode::PinPDown,
            winit::keyboard::NamedKey::PinPMove => KeyCode::PinPMove,
            winit::keyboard::NamedKey::PinPToggle => KeyCode::PinPToggle,
            winit::keyboard::NamedKey::PinPUp => KeyCode::PinPUp,
            winit::keyboard::NamedKey::PlaySpeedDown => KeyCode::PlaySpeedDown,
            winit::keyboard::NamedKey::PlaySpeedReset => KeyCode::PlaySpeedReset,
            winit::keyboard::NamedKey::PlaySpeedUp => KeyCode::PlaySpeedUp,
            winit::keyboard::NamedKey::RandomToggle => KeyCode::RandomToggle,
            winit::keyboard::NamedKey::RcLowBattery => KeyCode::RcLowBattery,
            winit::keyboard::NamedKey::RecordSpeedNext => KeyCode::RecordSpeedNext,
            winit::keyboard::NamedKey::RfBypass => KeyCode::RfBypass,
            winit::keyboard::NamedKey::ScanChannelsToggle => KeyCode::ScanChannelsToggle,
            winit::keyboard::NamedKey::ScreenModeNext => KeyCode::ScreenModeNext,
            winit::keyboard::NamedKey::Settings => KeyCode::Settings,
            winit::keyboard::NamedKey::SplitScreenToggle => KeyCode::SplitScreenToggle,
            winit::keyboard::NamedKey::STBInput => KeyCode::STBInput,
            winit::keyboard::NamedKey::STBPower => KeyCode::STBPower,
            winit::keyboard::NamedKey::Subtitle => KeyCode::Subtitle,
            winit::keyboard::NamedKey::Teletext => KeyCode::Teletext,
            winit::keyboard::NamedKey::VideoModeNext => KeyCode::VideoModeNext,
            winit::keyboard::NamedKey::Wink => KeyCode::Wink,
            winit::keyboard::NamedKey::ZoomToggle => KeyCode::ZoomToggle,
            winit::keyboard::NamedKey::F1 => KeyCode::F1,
            winit::keyboard::NamedKey::F2 => KeyCode::F2,
            winit::keyboard::NamedKey::F3 => KeyCode::F3,
            winit::keyboard::NamedKey::F4 => KeyCode::F4,
            winit::keyboard::NamedKey::F5 => KeyCode::F5,
            winit::keyboard::NamedKey::F6 => KeyCode::F6,
            winit::keyboard::NamedKey::F7 => KeyCode::F7,
            winit::keyboard::NamedKey::F8 => KeyCode::F8,
            winit::keyboard::NamedKey::F9 => KeyCode::F9,
            winit::keyboard::NamedKey::F10 => KeyCode::F10,
            winit::keyboard::NamedKey::F11 => KeyCode::F11,
            winit::keyboard::NamedKey::F12 => KeyCode::F12,
            winit::keyboard::NamedKey::F13 => KeyCode::F13,
            winit::keyboard::NamedKey::F14 => KeyCode::F14,
            winit::keyboard::NamedKey::F15 => KeyCode::F15,
            winit::keyboard::NamedKey::F16 => KeyCode::F16,
            winit::keyboard::NamedKey::F17 => KeyCode::F17,
            winit::keyboard::NamedKey::F18 => KeyCode::F18,
            winit::keyboard::NamedKey::F19 => KeyCode::F19,
            winit::keyboard::NamedKey::F20 => KeyCode::F20,
            winit::keyboard::NamedKey::F21 => KeyCode::F21,
            winit::keyboard::NamedKey::F22 => KeyCode::F22,
            winit::keyboard::NamedKey::F23 => KeyCode::F23,
            winit::keyboard::NamedKey::F24 => KeyCode::F24,
            winit::keyboard::NamedKey::F25 => KeyCode::F25,
            winit::keyboard::NamedKey::F26 => KeyCode::F26,
            winit::keyboard::NamedKey::F27 => KeyCode::F27,
            winit::keyboard::NamedKey::F28 => KeyCode::F28,
            winit::keyboard::NamedKey::F29 => KeyCode::F29,
            winit::keyboard::NamedKey::F30 => KeyCode::F30,
            winit::keyboard::NamedKey::F31 => KeyCode::F31,
            winit::keyboard::NamedKey::F32 => KeyCode::F32,
            winit::keyboard::NamedKey::F33 => KeyCode::F33,
            winit::keyboard::NamedKey::F34 => KeyCode::F34,
            winit::keyboard::NamedKey::F35 => KeyCode::F35,
            _ => todo!(),
        }
    }
}

impl TryFrom<&winit::keyboard::Key> for KeyCode {
    type Error = AgeError;

    fn try_from(key: &winit::keyboard::Key) -> Result<Self, Self::Error> {
        match key {
            winit::keyboard::Key::Named(named_key) => Ok((*named_key).into()),
            winit::keyboard::Key::Character(c) => {
                let mut chars = c.chars();
                assert!(chars.clone().count() == 1);
                Ok(KeyCode::Char(chars.next().unwrap()))
            }
            winit::keyboard::Key::Unidentified(_) => {
                Err("unidentified keys are not supported".into())
            }
            winit::keyboard::Key::Dead(_) => Err("dead keys are not supported".into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanCode {
    Backquote,
    Backslash,
    BracketLeft,
    BracketRight,
    Comma,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Equal,
    IntlBackslash,
    IntlRo,
    IntlYen,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Minus,
    Period,
    Quote,
    Semicolon,
    Slash,
    AltLeft,
    AltRight,
    Backspace,
    CapsLock,
    ContextMenu,
    ControlLeft,
    ControlRight,
    Enter,
    SuperLeft,
    SuperRight,
    ShiftLeft,
    ShiftRight,
    Space,
    Tab,
    Convert,
    KanaMode,
    Lang1,
    Lang2,
    Lang3,
    Lang4,
    Lang5,
    NonConvert,
    Delete,
    End,
    Help,
    Home,
    Insert,
    PageDown,
    PageUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    NumLock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadBackspace,
    NumpadClear,
    NumpadClearEntry,
    NumpadComma,
    NumpadDecimal,
    NumpadDivide,
    NumpadEnter,
    NumpadEqual,
    NumpadHash,
    NumpadMemoryAdd,
    NumpadMemoryClear,
    NumpadMemoryRecall,
    NumpadMemoryStore,
    NumpadMemorySubtract,
    NumpadMultiply,
    NumpadParenLeft,
    NumpadParenRight,
    NumpadStar,
    NumpadSubtract,
    Escape,
    Fn,
    FnLock,
    PrintScreen,
    ScrollLock,
    Pause,
    BrowserBack,
    BrowserFavorites,
    BrowserForward,
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,
    Eject,
    LaunchApp1,
    LaunchApp2,
    LaunchMail,
    MediaPlayPause,
    MediaSelect,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    Power,
    Sleep,
    AudioVolumeDown,
    AudioVolumeMute,
    AudioVolumeUp,
    WakeUp,
    Meta,
    Hyper,
    Turbo,
    Abort,
    Resume,
    Suspend,
    Again,
    Copy,
    Cut,
    Find,
    Open,
    Paste,
    Props,
    Select,
    Undo,
    Hiragana,
    Katakana,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
}

impl From<winit::keyboard::KeyCode> for ScanCode {
    fn from(keycode: winit::keyboard::KeyCode) -> Self {
        match keycode {
            winit::keyboard::KeyCode::Backquote => ScanCode::Backquote,
            winit::keyboard::KeyCode::Backslash => ScanCode::Backslash,
            winit::keyboard::KeyCode::BracketLeft => ScanCode::BracketLeft,
            winit::keyboard::KeyCode::BracketRight => ScanCode::BracketRight,
            winit::keyboard::KeyCode::Comma => ScanCode::Comma,
            winit::keyboard::KeyCode::Digit0 => ScanCode::Num0,
            winit::keyboard::KeyCode::Digit1 => ScanCode::Num1,
            winit::keyboard::KeyCode::Digit2 => ScanCode::Num2,
            winit::keyboard::KeyCode::Digit3 => ScanCode::Num3,
            winit::keyboard::KeyCode::Digit4 => ScanCode::Num4,
            winit::keyboard::KeyCode::Digit5 => ScanCode::Num5,
            winit::keyboard::KeyCode::Digit6 => ScanCode::Num6,
            winit::keyboard::KeyCode::Digit7 => ScanCode::Num7,
            winit::keyboard::KeyCode::Digit8 => ScanCode::Num8,
            winit::keyboard::KeyCode::Digit9 => ScanCode::Num9,
            winit::keyboard::KeyCode::Equal => ScanCode::Equal,
            winit::keyboard::KeyCode::IntlBackslash => ScanCode::IntlBackslash,
            winit::keyboard::KeyCode::IntlRo => ScanCode::IntlRo,
            winit::keyboard::KeyCode::IntlYen => ScanCode::IntlYen,
            winit::keyboard::KeyCode::KeyA => ScanCode::A,
            winit::keyboard::KeyCode::KeyB => ScanCode::B,
            winit::keyboard::KeyCode::KeyC => ScanCode::C,
            winit::keyboard::KeyCode::KeyD => ScanCode::D,
            winit::keyboard::KeyCode::KeyE => ScanCode::E,
            winit::keyboard::KeyCode::KeyF => ScanCode::F,
            winit::keyboard::KeyCode::KeyG => ScanCode::G,
            winit::keyboard::KeyCode::KeyH => ScanCode::H,
            winit::keyboard::KeyCode::KeyI => ScanCode::I,
            winit::keyboard::KeyCode::KeyJ => ScanCode::J,
            winit::keyboard::KeyCode::KeyK => ScanCode::K,
            winit::keyboard::KeyCode::KeyL => ScanCode::L,
            winit::keyboard::KeyCode::KeyM => ScanCode::M,
            winit::keyboard::KeyCode::KeyN => ScanCode::N,
            winit::keyboard::KeyCode::KeyO => ScanCode::O,
            winit::keyboard::KeyCode::KeyP => ScanCode::P,
            winit::keyboard::KeyCode::KeyQ => ScanCode::Q,
            winit::keyboard::KeyCode::KeyR => ScanCode::R,
            winit::keyboard::KeyCode::KeyS => ScanCode::S,
            winit::keyboard::KeyCode::KeyT => ScanCode::T,
            winit::keyboard::KeyCode::KeyU => ScanCode::U,
            winit::keyboard::KeyCode::KeyV => ScanCode::V,
            winit::keyboard::KeyCode::KeyW => ScanCode::W,
            winit::keyboard::KeyCode::KeyX => ScanCode::X,
            winit::keyboard::KeyCode::KeyY => ScanCode::Y,
            winit::keyboard::KeyCode::KeyZ => ScanCode::Z,
            winit::keyboard::KeyCode::Minus => ScanCode::Minus,
            winit::keyboard::KeyCode::Period => ScanCode::Period,
            winit::keyboard::KeyCode::Quote => ScanCode::Quote,
            winit::keyboard::KeyCode::Semicolon => ScanCode::Semicolon,
            winit::keyboard::KeyCode::Slash => ScanCode::Slash,
            winit::keyboard::KeyCode::AltLeft => ScanCode::AltLeft,
            winit::keyboard::KeyCode::AltRight => ScanCode::AltRight,
            winit::keyboard::KeyCode::Backspace => ScanCode::Backspace,
            winit::keyboard::KeyCode::CapsLock => ScanCode::CapsLock,
            winit::keyboard::KeyCode::ContextMenu => ScanCode::ContextMenu,
            winit::keyboard::KeyCode::ControlLeft => ScanCode::ControlLeft,
            winit::keyboard::KeyCode::ControlRight => ScanCode::ControlRight,
            winit::keyboard::KeyCode::Enter => ScanCode::Enter,
            winit::keyboard::KeyCode::SuperLeft => ScanCode::SuperLeft,
            winit::keyboard::KeyCode::SuperRight => ScanCode::SuperRight,
            winit::keyboard::KeyCode::ShiftLeft => ScanCode::ShiftLeft,
            winit::keyboard::KeyCode::ShiftRight => ScanCode::ShiftRight,
            winit::keyboard::KeyCode::Space => ScanCode::Space,
            winit::keyboard::KeyCode::Tab => ScanCode::Tab,
            winit::keyboard::KeyCode::Convert => ScanCode::Convert,
            winit::keyboard::KeyCode::KanaMode => ScanCode::KanaMode,
            winit::keyboard::KeyCode::Lang1 => ScanCode::Lang1,
            winit::keyboard::KeyCode::Lang2 => ScanCode::Lang2,
            winit::keyboard::KeyCode::Lang3 => ScanCode::Lang3,
            winit::keyboard::KeyCode::Lang4 => ScanCode::Lang4,
            winit::keyboard::KeyCode::Lang5 => ScanCode::Lang5,
            winit::keyboard::KeyCode::NonConvert => ScanCode::NonConvert,
            winit::keyboard::KeyCode::Delete => ScanCode::Delete,
            winit::keyboard::KeyCode::End => ScanCode::End,
            winit::keyboard::KeyCode::Help => ScanCode::Help,
            winit::keyboard::KeyCode::Home => ScanCode::Home,
            winit::keyboard::KeyCode::Insert => ScanCode::Insert,
            winit::keyboard::KeyCode::PageDown => ScanCode::PageDown,
            winit::keyboard::KeyCode::PageUp => ScanCode::PageUp,
            winit::keyboard::KeyCode::ArrowDown => ScanCode::ArrowDown,
            winit::keyboard::KeyCode::ArrowLeft => ScanCode::ArrowLeft,
            winit::keyboard::KeyCode::ArrowRight => ScanCode::ArrowRight,
            winit::keyboard::KeyCode::ArrowUp => ScanCode::ArrowUp,
            winit::keyboard::KeyCode::NumLock => ScanCode::NumLock,
            winit::keyboard::KeyCode::Numpad0 => ScanCode::Numpad0,
            winit::keyboard::KeyCode::Numpad1 => ScanCode::Numpad1,
            winit::keyboard::KeyCode::Numpad2 => ScanCode::Numpad2,
            winit::keyboard::KeyCode::Numpad3 => ScanCode::Numpad3,
            winit::keyboard::KeyCode::Numpad4 => ScanCode::Numpad4,
            winit::keyboard::KeyCode::Numpad5 => ScanCode::Numpad5,
            winit::keyboard::KeyCode::Numpad6 => ScanCode::Numpad6,
            winit::keyboard::KeyCode::Numpad7 => ScanCode::Numpad7,
            winit::keyboard::KeyCode::Numpad8 => ScanCode::Numpad8,
            winit::keyboard::KeyCode::Numpad9 => ScanCode::Numpad9,
            winit::keyboard::KeyCode::NumpadAdd => ScanCode::NumpadAdd,
            winit::keyboard::KeyCode::NumpadBackspace => ScanCode::NumpadBackspace,
            winit::keyboard::KeyCode::NumpadClear => ScanCode::NumpadClear,
            winit::keyboard::KeyCode::NumpadClearEntry => ScanCode::NumpadClearEntry,
            winit::keyboard::KeyCode::NumpadComma => ScanCode::NumpadComma,
            winit::keyboard::KeyCode::NumpadDecimal => ScanCode::NumpadDecimal,
            winit::keyboard::KeyCode::NumpadDivide => ScanCode::NumpadDivide,
            winit::keyboard::KeyCode::NumpadEnter => ScanCode::NumpadEnter,
            winit::keyboard::KeyCode::NumpadEqual => ScanCode::NumpadEqual,
            winit::keyboard::KeyCode::NumpadHash => ScanCode::NumpadHash,
            winit::keyboard::KeyCode::NumpadMemoryAdd => ScanCode::NumpadMemoryAdd,
            winit::keyboard::KeyCode::NumpadMemoryClear => ScanCode::NumpadMemoryClear,
            winit::keyboard::KeyCode::NumpadMemoryRecall => ScanCode::NumpadMemoryRecall,
            winit::keyboard::KeyCode::NumpadMemoryStore => ScanCode::NumpadMemoryStore,
            winit::keyboard::KeyCode::NumpadMemorySubtract => ScanCode::NumpadMemorySubtract,
            winit::keyboard::KeyCode::NumpadMultiply => ScanCode::NumpadMultiply,
            winit::keyboard::KeyCode::NumpadParenLeft => ScanCode::NumpadParenLeft,
            winit::keyboard::KeyCode::NumpadParenRight => ScanCode::NumpadParenRight,
            winit::keyboard::KeyCode::NumpadStar => ScanCode::NumpadStar,
            winit::keyboard::KeyCode::NumpadSubtract => ScanCode::NumpadSubtract,
            winit::keyboard::KeyCode::Escape => ScanCode::Escape,
            winit::keyboard::KeyCode::Fn => ScanCode::Fn,
            winit::keyboard::KeyCode::FnLock => ScanCode::FnLock,
            winit::keyboard::KeyCode::PrintScreen => ScanCode::PrintScreen,
            winit::keyboard::KeyCode::ScrollLock => ScanCode::ScrollLock,
            winit::keyboard::KeyCode::Pause => ScanCode::Pause,
            winit::keyboard::KeyCode::BrowserBack => ScanCode::BrowserBack,
            winit::keyboard::KeyCode::BrowserFavorites => ScanCode::BrowserFavorites,
            winit::keyboard::KeyCode::BrowserForward => ScanCode::BrowserForward,
            winit::keyboard::KeyCode::BrowserHome => ScanCode::BrowserHome,
            winit::keyboard::KeyCode::BrowserRefresh => ScanCode::BrowserRefresh,
            winit::keyboard::KeyCode::BrowserSearch => ScanCode::BrowserSearch,
            winit::keyboard::KeyCode::BrowserStop => ScanCode::BrowserStop,
            winit::keyboard::KeyCode::Eject => ScanCode::Eject,
            winit::keyboard::KeyCode::LaunchApp1 => ScanCode::LaunchApp1,
            winit::keyboard::KeyCode::LaunchApp2 => ScanCode::LaunchApp2,
            winit::keyboard::KeyCode::LaunchMail => ScanCode::LaunchMail,
            winit::keyboard::KeyCode::MediaPlayPause => ScanCode::MediaPlayPause,
            winit::keyboard::KeyCode::MediaSelect => ScanCode::MediaSelect,
            winit::keyboard::KeyCode::MediaStop => ScanCode::MediaStop,
            winit::keyboard::KeyCode::MediaTrackNext => ScanCode::MediaTrackNext,
            winit::keyboard::KeyCode::MediaTrackPrevious => ScanCode::MediaTrackPrevious,
            winit::keyboard::KeyCode::Power => ScanCode::Power,
            winit::keyboard::KeyCode::Sleep => ScanCode::Sleep,
            winit::keyboard::KeyCode::AudioVolumeDown => ScanCode::AudioVolumeDown,
            winit::keyboard::KeyCode::AudioVolumeMute => ScanCode::AudioVolumeMute,
            winit::keyboard::KeyCode::AudioVolumeUp => ScanCode::AudioVolumeUp,
            winit::keyboard::KeyCode::WakeUp => ScanCode::WakeUp,
            winit::keyboard::KeyCode::Meta => ScanCode::Meta,
            winit::keyboard::KeyCode::Hyper => ScanCode::Hyper,
            winit::keyboard::KeyCode::Turbo => ScanCode::Turbo,
            winit::keyboard::KeyCode::Abort => ScanCode::Abort,
            winit::keyboard::KeyCode::Resume => ScanCode::Resume,
            winit::keyboard::KeyCode::Suspend => ScanCode::Suspend,
            winit::keyboard::KeyCode::Again => ScanCode::Again,
            winit::keyboard::KeyCode::Copy => ScanCode::Copy,
            winit::keyboard::KeyCode::Cut => ScanCode::Cut,
            winit::keyboard::KeyCode::Find => ScanCode::Find,
            winit::keyboard::KeyCode::Open => ScanCode::Open,
            winit::keyboard::KeyCode::Paste => ScanCode::Paste,
            winit::keyboard::KeyCode::Props => ScanCode::Props,
            winit::keyboard::KeyCode::Select => ScanCode::Select,
            winit::keyboard::KeyCode::Undo => ScanCode::Undo,
            winit::keyboard::KeyCode::Hiragana => ScanCode::Hiragana,
            winit::keyboard::KeyCode::Katakana => ScanCode::Katakana,
            winit::keyboard::KeyCode::F1 => ScanCode::F1,
            winit::keyboard::KeyCode::F2 => ScanCode::F2,
            winit::keyboard::KeyCode::F3 => ScanCode::F3,
            winit::keyboard::KeyCode::F4 => ScanCode::F4,
            winit::keyboard::KeyCode::F5 => ScanCode::F5,
            winit::keyboard::KeyCode::F6 => ScanCode::F6,
            winit::keyboard::KeyCode::F7 => ScanCode::F7,
            winit::keyboard::KeyCode::F8 => ScanCode::F8,
            winit::keyboard::KeyCode::F9 => ScanCode::F9,
            winit::keyboard::KeyCode::F10 => ScanCode::F10,
            winit::keyboard::KeyCode::F11 => ScanCode::F11,
            winit::keyboard::KeyCode::F12 => ScanCode::F12,
            winit::keyboard::KeyCode::F13 => ScanCode::F13,
            winit::keyboard::KeyCode::F14 => ScanCode::F14,
            winit::keyboard::KeyCode::F15 => ScanCode::F15,
            winit::keyboard::KeyCode::F16 => ScanCode::F16,
            winit::keyboard::KeyCode::F17 => ScanCode::F17,
            winit::keyboard::KeyCode::F18 => ScanCode::F18,
            winit::keyboard::KeyCode::F19 => ScanCode::F19,
            winit::keyboard::KeyCode::F20 => ScanCode::F20,
            winit::keyboard::KeyCode::F21 => ScanCode::F21,
            winit::keyboard::KeyCode::F22 => ScanCode::F22,
            winit::keyboard::KeyCode::F23 => ScanCode::F23,
            winit::keyboard::KeyCode::F24 => ScanCode::F24,
            winit::keyboard::KeyCode::F25 => ScanCode::F25,
            winit::keyboard::KeyCode::F26 => ScanCode::F26,
            winit::keyboard::KeyCode::F27 => ScanCode::F27,
            winit::keyboard::KeyCode::F28 => ScanCode::F28,
            winit::keyboard::KeyCode::F29 => ScanCode::F29,
            winit::keyboard::KeyCode::F30 => ScanCode::F30,
            winit::keyboard::KeyCode::F31 => ScanCode::F31,
            winit::keyboard::KeyCode::F32 => ScanCode::F32,
            winit::keyboard::KeyCode::F33 => ScanCode::F33,
            winit::keyboard::KeyCode::F34 => ScanCode::F34,
            winit::keyboard::KeyCode::F35 => ScanCode::F35,
            _ => unreachable!("if we match here, there is a new key code"),
        }
    }
}

impl TryFrom<winit::keyboard::PhysicalKey> for ScanCode {
    type Error = AgeError;

    fn try_from(key: winit::keyboard::PhysicalKey) -> Result<Self, Self::Error> {
        match key {
            winit::keyboard::PhysicalKey::Code(keycode) => Ok(keycode.into()),
            winit::keyboard::PhysicalKey::Unidentified(_) => {
                Err("unidentified scancodes are not supported".into())
            }
        }
    }
}

impl From<EventLoopError> for AgeError {
    fn from(err: EventLoopError) -> Self {
        AgeError::new("failed to create event loop").with_source(err)
    }
}

impl From<OsError> for AgeError {
    fn from(err: OsError) -> Self {
        AgeError::new("failed to perform os action").with_source(err)
    }
}

#[cfg(test)]
mod test {
    use winit::dpi::PhysicalPosition;

    use super::*;

    #[test]
    fn mouse_defaults() {
        let m = Mouse::new();

        assert_eq!(Vec2::ZERO, m.position());
        assert_eq!(Vec2::ZERO, m.position_delta());
        assert_eq!(Vec2::ZERO, m.scroll_delta());

        assert!(!m.button_pressed(MouseButton::Left));
        assert!(!m.button_held(MouseButton::Left));
        assert!(!m.button_released(MouseButton::Left));

        assert!(!m.button_pressed(MouseButton::Right));
        assert!(!m.button_held(MouseButton::Right));
        assert!(!m.button_released(MouseButton::Right));

        assert!(!m.button_pressed(MouseButton::Middle));
        assert!(!m.button_held(MouseButton::Middle));
        assert!(!m.button_released(MouseButton::Middle));

        assert!(!m.button_pressed(MouseButton::Back));
        assert!(!m.button_held(MouseButton::Back));
        assert!(!m.button_released(MouseButton::Back));

        assert!(!m.button_pressed(MouseButton::Forward));
        assert!(!m.button_held(MouseButton::Forward));
        assert!(!m.button_released(MouseButton::Forward));
    }

    #[test]
    fn mouse_position_changed() {
        let mut m = Mouse::new();

        m.on_event(&cursor_moved(10.0, 10.0));

        assert_eq!(v2(10.0, 10.0), m.position());
        assert_eq!(v2(10.0, 10.0), m.position_delta());

        m.flush();
        m.on_event(&cursor_moved(15.0, 20.0));

        assert_eq!(v2(15.0, 20.0), m.position());
        assert_eq!(v2(5.0, 10.0), m.position_delta());
    }

    #[test]
    fn mouse_left_button() {
        let mut m = Mouse::new();

        assert!(!m.button_pressed(MouseButton::Left));
        assert!(!m.button_held(MouseButton::Left));
        assert!(!m.button_released(MouseButton::Left));

        m.flush();
        m.on_event(&mouse_input(
            winit::event::ElementState::Pressed,
            winit::event::MouseButton::Left,
        ));

        assert!(m.button_pressed(MouseButton::Left));
        assert!(!m.button_held(MouseButton::Left));
        assert!(!m.button_released(MouseButton::Left));

        m.flush();

        assert!(!m.button_pressed(MouseButton::Left));
        assert!(m.button_held(MouseButton::Left));
        assert!(!m.button_released(MouseButton::Left));

        m.flush();
        m.on_event(&mouse_input(
            winit::event::ElementState::Released,
            winit::event::MouseButton::Left,
        ));

        assert!(!m.button_pressed(MouseButton::Left));
        assert!(!m.button_held(MouseButton::Left));
        assert!(m.button_released(MouseButton::Left));

        m.flush();

        assert!(!m.button_pressed(MouseButton::Left));
        assert!(!m.button_held(MouseButton::Left));
        assert!(!m.button_released(MouseButton::Left));
    }

    #[test]
    fn mouse_scroll_delta() {
        let mut m = Mouse::new();

        m.on_event(&mouse_wheel(1.0, 2.0));

        assert_eq!(v2(1.0, 2.0), m.scroll_delta());

        m.flush();
        m.on_event(&mouse_wheel(-1.0, -2.0));

        assert_eq!(v2(-1.0, -2.0), m.scroll_delta());

        m.flush();
        m.on_event(&mouse_wheel(3.0, 4.0));
        m.on_event(&mouse_wheel(5.0, 6.0));

        assert_eq!(v2(8.0, 10.0), m.scroll_delta());

        m.flush();

        assert_eq!(v2(0.0, 0.0), m.scroll_delta());
    }

    fn cursor_moved(x: f32, y: f32) -> winit::event::WindowEvent {
        winit::event::WindowEvent::CursorMoved {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            position: PhysicalPosition::new(x as f64, y as f64),
        }
    }

    fn mouse_input(
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) -> winit::event::WindowEvent {
        winit::event::WindowEvent::MouseInput {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            state,
            button,
        }
    }

    fn mouse_wheel(x: f32, y: f32) -> winit::event::WindowEvent {
        winit::event::WindowEvent::MouseWheel {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            delta: winit::event::MouseScrollDelta::LineDelta(x, y),
            phase: winit::event::TouchPhase::Moved, // Doesn't really matter whilst touch is unsupported.
        }
    }

    #[test]
    fn keyboard_defaults() {
        let k = Keyboard::new();

        assert!(!k.key_pressed('q'));
        assert!(!k.key_held('q'));
        assert!(!k.key_released('q'));

        assert!(!k.key_pressed(KeyCode::Space));
        assert!(!k.key_held(KeyCode::Space));
        assert!(!k.key_released(KeyCode::Space));

        assert!(!k.key_pressed((KeyCode::Alt, KeyLocation::Left)));
        assert!(!k.key_held((KeyCode::Alt, KeyLocation::Left)));
        assert!(!k.key_released((KeyCode::Alt, KeyLocation::Left)));

        assert!(!k.key_pressed(ScanCode::Q));
        assert!(!k.key_held(ScanCode::Q));
        assert!(!k.key_released(ScanCode::Q));

        assert!(!k.alt_key());
        assert!(!k.control_key());
        assert!(!k.shift_key());
        assert!(!k.super_key());
    }

    #[test]
    fn keyboard_q_key() {
        let mut k = Keyboard::new();

        assert!(!k.key_pressed('q'));
        assert!(!k.key_held('q'));
        assert!(!k.key_released('q'));

        assert!(!k.key_pressed(ScanCode::Q));
        assert!(!k.key_held(ScanCode::Q));
        assert!(!k.key_released(ScanCode::Q));

        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyQ),
            &winit::keyboard::Key::Character("q".into()),
            winit::keyboard::KeyLocation::Standard,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.key_pressed('q'));
        assert!(!k.key_held('q'));
        assert!(!k.key_released('q'));

        assert!(k.key_pressed(ScanCode::Q));
        assert!(!k.key_held(ScanCode::Q));
        assert!(!k.key_released(ScanCode::Q));

        k.flush();

        assert!(!k.key_pressed('q'));
        assert!(k.key_held('q'));
        assert!(!k.key_released('q'));

        assert!(!k.key_pressed(ScanCode::Q));
        assert!(k.key_held(ScanCode::Q));
        assert!(!k.key_released(ScanCode::Q));

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyQ),
            &winit::keyboard::Key::Character("q".into()),
            winit::keyboard::KeyLocation::Standard,
            winit::event::ElementState::Pressed,
            true,
        );

        assert!(!k.key_pressed('q'));
        assert!(k.key_held('q'));
        assert!(!k.key_pressed('q'));

        assert!(!k.key_pressed(ScanCode::Q));
        assert!(k.key_held(ScanCode::Q));
        assert!(!k.key_released(ScanCode::Q));

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyQ),
            &winit::keyboard::Key::Character("q".into()),
            winit::keyboard::KeyLocation::Standard,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.key_pressed('q'));
        assert!(!k.key_held('q'));
        assert!(k.key_released('q'));

        assert!(!k.key_pressed(ScanCode::Q));
        assert!(!k.key_held(ScanCode::Q));
        assert!(k.key_released(ScanCode::Q));

        k.flush();

        assert!(!k.key_pressed('q'));
        assert!(!k.key_held('q'));
        assert!(!k.key_released('q'));

        assert!(!k.key_pressed(ScanCode::Q));
        assert!(!k.key_held(ScanCode::Q));
        assert!(!k.key_released(ScanCode::Q));
    }

    #[test]
    fn keyboard_esc_key() {
        let mut k = Keyboard::new();

        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            winit::keyboard::KeyLocation::Standard,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.key_pressed(KeyCode::Escape));
        assert!(!k.key_held(KeyCode::Escape));
        assert!(!k.key_released(KeyCode::Escape));

        assert!(k.key_pressed(ScanCode::Escape));
        assert!(!k.key_held(ScanCode::Escape));
        assert!(!k.key_released(ScanCode::Escape));

        k.flush();

        assert!(!k.key_pressed(KeyCode::Escape));
        assert!(k.key_held(KeyCode::Escape));
        assert!(!k.key_released(KeyCode::Escape));

        assert!(!k.key_pressed(ScanCode::Escape));
        assert!(k.key_held(ScanCode::Escape));
        assert!(!k.key_released(ScanCode::Escape));

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            winit::keyboard::KeyLocation::Standard,
            winit::event::ElementState::Pressed,
            true,
        );

        assert!(!k.key_pressed(KeyCode::Escape));
        assert!(k.key_held(KeyCode::Escape));
        assert!(!k.key_released(KeyCode::Escape));

        assert!(!k.key_pressed(ScanCode::Escape));
        assert!(k.key_held(ScanCode::Escape));
        assert!(!k.key_released(ScanCode::Escape));

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
            winit::keyboard::KeyLocation::Standard,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.key_pressed(KeyCode::Escape));
        assert!(!k.key_held(KeyCode::Escape));
        assert!(k.key_released(KeyCode::Escape));

        assert!(!k.key_pressed(ScanCode::Escape));
        assert!(!k.key_held(ScanCode::Escape));
        assert!(k.key_released(ScanCode::Escape));

        k.flush();

        assert!(!k.key_pressed(KeyCode::Escape));
        assert!(!k.key_held(KeyCode::Escape));
        assert!(!k.key_released(KeyCode::Escape));

        assert!(!k.key_pressed(ScanCode::Escape));
        assert!(!k.key_held(ScanCode::Escape));
        assert!(!k.key_released(ScanCode::Escape));
    }

    #[test]
    fn keyboard_alt_key() {
        let mut k = Keyboard::new();

        assert!(!k.alt_key());

        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::AltLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Alt),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.alt_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::AltLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Alt),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.alt_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::AltRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Alt),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.alt_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::AltRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Alt),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.alt_key());
    }

    #[test]
    fn keyboard_control_key() {
        let mut k = Keyboard::new();

        assert!(!k.control_key());

        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ControlLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.control_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ControlLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.control_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ControlRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.control_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ControlRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Control),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.control_key());
    }

    #[test]
    fn keyboard_shift_key() {
        let mut k = Keyboard::new();

        assert!(!k.shift_key());

        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ShiftLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Shift),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.shift_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ShiftLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Shift),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.shift_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ShiftRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Shift),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.shift_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ShiftRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Shift),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.shift_key());
    }

    #[test]
    fn keyboard_super_key() {
        let mut k = Keyboard::new();

        assert!(!k.super_key());

        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::SuperLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Super),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.super_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::SuperLeft),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Super),
            winit::keyboard::KeyLocation::Left,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.super_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::SuperRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Super),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Pressed,
            false,
        );

        assert!(k.super_key());

        k.flush();
        k.on_keyboard_input(
            winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::SuperRight),
            &winit::keyboard::Key::Named(winit::keyboard::NamedKey::Super),
            winit::keyboard::KeyLocation::Right,
            winit::event::ElementState::Released,
            false,
        );

        assert!(!k.super_key());
    }
}
