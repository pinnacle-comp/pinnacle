use iced::keyboard::{key::Named, Key, Location};
use smithay_client_toolkit::seat::keyboard::Keysym;

// All this stuff from cosmic's iced-sctk

fn keysym_to_iced_key(keysym: Keysym) -> Key {
    let named = match keysym {
        // TTY function keys
        Keysym::BackSpace => Named::Backspace,
        Keysym::Tab => Named::Tab,
        // Keysym::Linefeed => Named::Linefeed,
        Keysym::Clear => Named::Clear,
        Keysym::Return => Named::Enter,
        Keysym::Pause => Named::Pause,
        Keysym::Scroll_Lock => Named::ScrollLock,
        Keysym::Sys_Req => Named::PrintScreen,
        Keysym::Escape => Named::Escape,
        Keysym::Delete => Named::Delete,

        // IME keys
        Keysym::Multi_key => Named::Compose,
        Keysym::Codeinput => Named::CodeInput,
        Keysym::SingleCandidate => Named::SingleCandidate,
        Keysym::MultipleCandidate => Named::AllCandidates,
        Keysym::PreviousCandidate => Named::PreviousCandidate,

        // Japanese key
        Keysym::Kanji => Named::KanjiMode,
        Keysym::Muhenkan => Named::NonConvert,
        Keysym::Henkan_Mode => Named::Convert,
        Keysym::Romaji => Named::Romaji,
        Keysym::Hiragana => Named::Hiragana,
        Keysym::Hiragana_Katakana => Named::HiraganaKatakana,
        Keysym::Zenkaku => Named::Zenkaku,
        Keysym::Hankaku => Named::Hankaku,
        Keysym::Zenkaku_Hankaku => Named::ZenkakuHankaku,
        // Keysym::Touroku => Named::Touroku,
        // Keysym::Massyo => Named::Massyo,
        Keysym::Kana_Lock => Named::KanaMode,
        Keysym::Kana_Shift => Named::KanaMode,
        Keysym::Eisu_Shift => Named::Alphanumeric,
        Keysym::Eisu_toggle => Named::Alphanumeric,
        // NOTE: The next three items are aliases for values we've already mapped.
        // Keysym::Kanji_Bangou => Named::CodeInput,
        // Keysym::Zen_Koho => Named::AllCandidates,
        // Keysym::Mae_Koho => Named::PreviousCandidate,

        // Cursor control & motion
        Keysym::Home => Named::Home,
        Keysym::Left => Named::ArrowLeft,
        Keysym::Up => Named::ArrowUp,
        Keysym::Right => Named::ArrowRight,
        Keysym::Down => Named::ArrowDown,
        // Keysym::Prior => Named::PageUp,
        Keysym::Page_Up => Named::PageUp,
        // Keysym::Next => Named::PageDown,
        Keysym::Page_Down => Named::PageDown,
        Keysym::End => Named::End,
        // Keysym::Begin => Named::Begin,

        // Misc. functions
        Keysym::Select => Named::Select,
        Keysym::Print => Named::PrintScreen,
        Keysym::Execute => Named::Execute,
        Keysym::Insert => Named::Insert,
        Keysym::Undo => Named::Undo,
        Keysym::Redo => Named::Redo,
        Keysym::Menu => Named::ContextMenu,
        Keysym::Find => Named::Find,
        Keysym::Cancel => Named::Cancel,
        Keysym::Help => Named::Help,
        Keysym::Break => Named::Pause,
        Keysym::Mode_switch => Named::ModeChange,
        // Keysym::script_switch => Named::ModeChange,
        Keysym::Num_Lock => Named::NumLock,

        // Keypad keys
        // Keysym::KP_Space => return Key::Character(" "),
        Keysym::KP_Tab => Named::Tab,
        Keysym::KP_Enter => Named::Enter,
        Keysym::KP_F1 => Named::F1,
        Keysym::KP_F2 => Named::F2,
        Keysym::KP_F3 => Named::F3,
        Keysym::KP_F4 => Named::F4,
        Keysym::KP_Home => Named::Home,
        Keysym::KP_Left => Named::ArrowLeft,
        Keysym::KP_Up => Named::ArrowUp,
        Keysym::KP_Right => Named::ArrowRight,
        Keysym::KP_Down => Named::ArrowDown,
        // Keysym::KP_Prior => Named::PageUp,
        Keysym::KP_Page_Up => Named::PageUp,
        // Keysym::KP_Next => Named::PageDown,
        Keysym::KP_Page_Down => Named::PageDown,
        Keysym::KP_End => Named::End,
        // This is the key labeled "5" on the numpad when NumLock is off.
        // Keysym::KP_Begin => Named::Begin,
        Keysym::KP_Insert => Named::Insert,
        Keysym::KP_Delete => Named::Delete,
        // Keysym::KP_Equal => Named::Equal,
        // Keysym::KP_Multiply => Named::Multiply,
        // Keysym::KP_Add => Named::Add,
        // Keysym::KP_Separator => Named::Separator,
        // Keysym::KP_Subtract => Named::Subtract,
        // Keysym::KP_Decimal => Named::Decimal,
        // Keysym::KP_Divide => Named::Divide,

        // Keysym::KP_0 => return Key::Character("0"),
        // Keysym::KP_1 => return Key::Character("1"),
        // Keysym::KP_2 => return Key::Character("2"),
        // Keysym::KP_3 => return Key::Character("3"),
        // Keysym::KP_4 => return Key::Character("4"),
        // Keysym::KP_5 => return Key::Character("5"),
        // Keysym::KP_6 => return Key::Character("6"),
        // Keysym::KP_7 => return Key::Character("7"),
        // Keysym::KP_8 => return Key::Character("8"),
        // Keysym::KP_9 => return Key::Character("9"),

        // Function keys
        Keysym::F1 => Named::F1,
        Keysym::F2 => Named::F2,
        Keysym::F3 => Named::F3,
        Keysym::F4 => Named::F4,
        Keysym::F5 => Named::F5,
        Keysym::F6 => Named::F6,
        Keysym::F7 => Named::F7,
        Keysym::F8 => Named::F8,
        Keysym::F9 => Named::F9,
        Keysym::F10 => Named::F10,
        Keysym::F11 => Named::F11,
        Keysym::F12 => Named::F12,
        Keysym::F13 => Named::F13,
        Keysym::F14 => Named::F14,
        Keysym::F15 => Named::F15,
        Keysym::F16 => Named::F16,
        Keysym::F17 => Named::F17,
        Keysym::F18 => Named::F18,
        Keysym::F19 => Named::F19,
        Keysym::F20 => Named::F20,
        Keysym::F21 => Named::F21,
        Keysym::F22 => Named::F22,
        Keysym::F23 => Named::F23,
        Keysym::F24 => Named::F24,
        Keysym::F25 => Named::F25,
        Keysym::F26 => Named::F26,
        Keysym::F27 => Named::F27,
        Keysym::F28 => Named::F28,
        Keysym::F29 => Named::F29,
        Keysym::F30 => Named::F30,
        Keysym::F31 => Named::F31,
        Keysym::F32 => Named::F32,
        Keysym::F33 => Named::F33,
        Keysym::F34 => Named::F34,
        Keysym::F35 => Named::F35,

        // Modifiers
        Keysym::Shift_L => Named::Shift,
        Keysym::Shift_R => Named::Shift,
        Keysym::Control_L => Named::Control,
        Keysym::Control_R => Named::Control,
        Keysym::Caps_Lock => Named::CapsLock,
        // Keysym::Shift_Lock => Named::ShiftLock,

        // Keysym::Meta_L => Named::Meta,
        // Keysym::Meta_R => Named::Meta,
        Keysym::Alt_L => Named::Alt,
        Keysym::Alt_R => Named::Alt,
        Keysym::Super_L => Named::Super,
        Keysym::Super_R => Named::Super,
        Keysym::Hyper_L => Named::Hyper,
        Keysym::Hyper_R => Named::Hyper,

        // XKB function and modifier keys
        // Keysym::ISO_Lock => Named::IsoLock,
        // Keysym::ISO_Level2_Latch => Named::IsoLevel2Latch,
        Keysym::ISO_Level3_Shift => Named::AltGraph,
        Keysym::ISO_Level3_Latch => Named::AltGraph,
        Keysym::ISO_Level3_Lock => Named::AltGraph,
        // Keysym::ISO_Level5_Shift => Named::IsoLevel5Shift,
        // Keysym::ISO_Level5_Latch => Named::IsoLevel5Latch,
        // Keysym::ISO_Level5_Lock => Named::IsoLevel5Lock,
        // Keysym::ISO_Group_Shift => Named::IsoGroupShift,
        // Keysym::ISO_Group_Latch => Named::IsoGroupLatch,
        // Keysym::ISO_Group_Lock => Named::IsoGroupLock,
        Keysym::ISO_Next_Group => Named::GroupNext,
        // Keysym::ISO_Next_Group_Lock => Named::GroupNextLock,
        Keysym::ISO_Prev_Group => Named::GroupPrevious,
        // Keysym::ISO_Prev_Group_Lock => Named::GroupPreviousLock,
        Keysym::ISO_First_Group => Named::GroupFirst,
        // Keysym::ISO_First_Group_Lock => Named::GroupFirstLock,
        Keysym::ISO_Last_Group => Named::GroupLast,
        // Keysym::ISO_Last_Group_Lock => Named::GroupLastLock,
        //
        Keysym::ISO_Left_Tab => Named::Tab,
        // Keysym::ISO_Move_Line_Up => Named::IsoMoveLineUp,
        // Keysym::ISO_Move_Line_Down => Named::IsoMoveLineDown,
        // Keysym::ISO_Partial_Line_Up => Named::IsoPartialLineUp,
        // Keysym::ISO_Partial_Line_Down => Named::IsoPartialLineDown,
        // Keysym::ISO_Partial_Space_Left => Named::IsoPartialSpaceLeft,
        // Keysym::ISO_Partial_Space_Right => Named::IsoPartialSpaceRight,
        // Keysym::ISO_Set_Margin_Left => Named::IsoSetMarginLeft,
        // Keysym::ISO_Set_Margin_Right => Named::IsoSetMarginRight,
        // Keysym::ISO_Release_Margin_Left => Named::IsoReleaseMarginLeft,
        // Keysym::ISO_Release_Margin_Right => Named::IsoReleaseMarginRight,
        // Keysym::ISO_Release_Both_Margins => Named::IsoReleaseBothMargins,
        // Keysym::ISO_Fast_Cursor_Left => Named::IsoFastCursorLeft,
        // Keysym::ISO_Fast_Cursor_Right => Named::IsoFastCursorRight,
        // Keysym::ISO_Fast_Cursor_Up => Named::IsoFastCursorUp,
        // Keysym::ISO_Fast_Cursor_Down => Named::IsoFastCursorDown,
        // Keysym::ISO_Continuous_Underline => Named::IsoContinuousUnderline,
        // Keysym::ISO_Discontinuous_Underline => Named::IsoDiscontinuousUnderline,
        // Keysym::ISO_Emphasize => Named::IsoEmphasize,
        // Keysym::ISO_Center_Object => Named::IsoCenterObject,
        Keysym::ISO_Enter => Named::Enter,

        // dead_grave..dead_currency

        // dead_lowline..dead_longsolidusoverlay

        // dead_a..dead_capital_schwa

        // dead_greek

        // First_Virtual_Screen..Terminate_Server

        // AccessX_Enable..AudibleBell_Enable

        // Pointer_Left..Pointer_Drag5

        // Pointer_EnableKeys..Pointer_DfltBtnPrev

        // ch..C_H

        // 3270 terminal keys
        // Keysym::3270_Duplicate => Named::Duplicate,
        // Keysym::3270_FieldMark => Named::FieldMark,
        // Keysym::3270_Right2 => Named::Right2,
        // Keysym::3270_Left2 => Named::Left2,
        // Keysym::3270_BackTab => Named::BackTab,
        Keysym::_3270_EraseEOF => Named::EraseEof,
        // Keysym::3270_EraseInput => Named::EraseInput,
        // Keysym::3270_Reset => Named::Reset,
        // Keysym::3270_Quit => Named::Quit,
        // Keysym::3270_PA1 => Named::Pa1,
        // Keysym::3270_PA2 => Named::Pa2,
        // Keysym::3270_PA3 => Named::Pa3,
        // Keysym::3270_Test => Named::Test,
        Keysym::_3270_Attn => Named::Attn,
        // Keysym::3270_CursorBlink => Named::CursorBlink,
        // Keysym::3270_AltCursor => Named::AltCursor,
        // Keysym::3270_KeyClick => Named::KeyClick,
        // Keysym::3270_Jump => Named::Jump,
        // Keysym::3270_Ident => Named::Ident,
        // Keysym::3270_Rule => Named::Rule,
        // Keysym::3270_Copy => Named::Copy,
        Keysym::_3270_Play => Named::Play,
        // Keysym::3270_Setup => Named::Setup,
        // Keysym::3270_Record => Named::Record,
        // Keysym::3270_ChangeScreen => Named::ChangeScreen,
        // Keysym::3270_DeleteWord => Named::DeleteWord,
        Keysym::_3270_ExSelect => Named::ExSel,
        Keysym::_3270_CursorSelect => Named::CrSel,
        Keysym::_3270_PrintScreen => Named::PrintScreen,
        Keysym::_3270_Enter => Named::Enter,

        Keysym::space => Named::Space,
        // exclam..Sinh_kunddaliya

        // XFree86
        // Keysym::XF86_ModeLock => Named::ModeLock,

        // XFree86 - Backlight controls
        Keysym::XF86_MonBrightnessUp => Named::BrightnessUp,
        Keysym::XF86_MonBrightnessDown => Named::BrightnessDown,
        // Keysym::XF86_KbdLightOnOff => Named::LightOnOff,
        // Keysym::XF86_KbdBrightnessUp => Named::KeyboardBrightnessUp,
        // Keysym::XF86_KbdBrightnessDown => Named::KeyboardBrightnessDown,

        // XFree86 - "Internet"
        Keysym::XF86_Standby => Named::Standby,
        Keysym::XF86_AudioLowerVolume => Named::AudioVolumeDown,
        Keysym::XF86_AudioRaiseVolume => Named::AudioVolumeUp,
        Keysym::XF86_AudioPlay => Named::MediaPlay,
        Keysym::XF86_AudioStop => Named::MediaStop,
        Keysym::XF86_AudioPrev => Named::MediaTrackPrevious,
        Keysym::XF86_AudioNext => Named::MediaTrackNext,
        Keysym::XF86_HomePage => Named::BrowserHome,
        Keysym::XF86_Mail => Named::LaunchMail,
        // Keysym::XF86_Start => Named::Start,
        Keysym::XF86_Search => Named::BrowserSearch,
        Keysym::XF86_AudioRecord => Named::MediaRecord,

        // XFree86 - PDA
        Keysym::XF86_Calculator => Named::LaunchApplication2,
        // Keysym::XF86_Memo => Named::Memo,
        // Keysym::XF86_ToDoList => Named::ToDoList,
        Keysym::XF86_Calendar => Named::LaunchCalendar,
        Keysym::XF86_PowerDown => Named::Power,
        // Keysym::XF86_ContrastAdjust => Named::AdjustContrast,
        // Keysym::XF86_RockerUp => Named::RockerUp,
        // Keysym::XF86_RockerDown => Named::RockerDown,
        // Keysym::XF86_RockerEnter => Named::RockerEnter,

        // XFree86 - More "Internet"
        Keysym::XF86_Back => Named::BrowserBack,
        Keysym::XF86_Forward => Named::BrowserForward,
        // Keysym::XF86_Stop => Named::Stop,
        Keysym::XF86_Refresh => Named::BrowserRefresh,
        Keysym::XF86_PowerOff => Named::Power,
        Keysym::XF86_WakeUp => Named::WakeUp,
        Keysym::XF86_Eject => Named::Eject,
        Keysym::XF86_ScreenSaver => Named::LaunchScreenSaver,
        Keysym::XF86_WWW => Named::LaunchWebBrowser,
        Keysym::XF86_Sleep => Named::Standby,
        Keysym::XF86_Favorites => Named::BrowserFavorites,
        Keysym::XF86_AudioPause => Named::MediaPause,
        // Keysym::XF86_AudioMedia => Named::AudioMedia,
        Keysym::XF86_MyComputer => Named::LaunchApplication1,
        // Keysym::XF86_VendorHome => Named::VendorHome,
        // Keysym::XF86_LightBulb => Named::LightBulb,
        // Keysym::XF86_Shop => Named::BrowserShop,
        // Keysym::XF86_History => Named::BrowserHistory,
        // Keysym::XF86_OpenURL => Named::OpenUrl,
        // Keysym::XF86_AddFavorite => Named::AddFavorite,
        // Keysym::XF86_HotLinks => Named::HotLinks,
        // Keysym::XF86_BrightnessAdjust => Named::BrightnessAdjust,
        // Keysym::XF86_Finance => Named::BrowserFinance,
        // Keysym::XF86_Community => Named::BrowserCommunity,
        Keysym::XF86_AudioRewind => Named::MediaRewind,
        // Keysym::XF86_BackForward => Key::???,
        // XF86_Launch0..XF86_LaunchF

        // XF86_ApplicationLeft..XF86_CD
        Keysym::XF86_Calculater => Named::LaunchApplication2, // Nice typo, libxkbcommon :)
        // XF86_Clear
        Keysym::XF86_Close => Named::Close,
        Keysym::XF86_Copy => Named::Copy,
        Keysym::XF86_Cut => Named::Cut,
        // XF86_Display..XF86_Documents
        Keysym::XF86_Excel => Named::LaunchSpreadsheet,
        // XF86_Explorer..XF86iTouch
        Keysym::XF86_LogOff => Named::LogOff,
        // XF86_Market..XF86_MenuPB
        Keysym::XF86_MySites => Named::BrowserFavorites,
        Keysym::XF86_New => Named::New,
        // XF86_News..XF86_OfficeHome
        Keysym::XF86_Open => Named::Open,
        // XF86_Option
        Keysym::XF86_Paste => Named::Paste,
        Keysym::XF86_Phone => Named::LaunchPhone,
        // XF86_Q
        Keysym::XF86_Reply => Named::MailReply,
        Keysym::XF86_Reload => Named::BrowserRefresh,
        // XF86_RotateWindows..XF86_RotationKB
        Keysym::XF86_Save => Named::Save,
        // XF86_ScrollUp..XF86_ScrollClick
        Keysym::XF86_Send => Named::MailSend,
        Keysym::XF86_Spell => Named::SpellCheck,
        Keysym::XF86_SplitScreen => Named::SplitScreenToggle,
        // XF86_Support..XF86_User2KB
        Keysym::XF86_Video => Named::LaunchMediaPlayer,
        // XF86_WheelButton
        Keysym::XF86_Word => Named::LaunchWordProcessor,
        // XF86_Xfer
        Keysym::XF86_ZoomIn => Named::ZoomIn,
        Keysym::XF86_ZoomOut => Named::ZoomOut,

        // XF86_Away..XF86_Messenger
        Keysym::XF86_WebCam => Named::LaunchWebCam,
        Keysym::XF86_MailForward => Named::MailForward,
        // XF86_Pictures
        Keysym::XF86_Music => Named::LaunchMusicPlayer,

        // XF86_Battery..XF86_UWB
        //
        Keysym::XF86_AudioForward => Named::MediaFastForward,
        // XF86_AudioRepeat
        Keysym::XF86_AudioRandomPlay => Named::RandomToggle,
        Keysym::XF86_Subtitle => Named::Subtitle,
        Keysym::XF86_AudioCycleTrack => Named::MediaAudioTrack,
        // XF86_CycleAngle..XF86_Blue
        //
        Keysym::XF86_Suspend => Named::Standby,
        Keysym::XF86_Hibernate => Named::Hibernate,
        // XF86_TouchpadToggle..XF86_TouchpadOff
        //
        Keysym::XF86_AudioMute => Named::AudioVolumeMute,

        // XF86_Switch_VT_1..XF86_Switch_VT_12

        // XF86_Ungrab..XF86_ClearGrab
        Keysym::XF86_Next_VMode => Named::VideoModeNext,
        // Keysym::XF86_Prev_VMode => Named::VideoModePrevious,
        // XF86_LogWindowTree..XF86_LogGrabInfo

        // SunFA_Grave..SunFA_Cedilla

        // Keysym::SunF36 => Named::F36 | Named::F11,
        // Keysym::SunF37 => Named::F37 | Named::F12,

        // Keysym::SunSys_Req => Named::PrintScreen,
        // The next couple of xkb (until SunStop) are already handled.
        // SunPrint_Screen..SunPageDown

        // SunUndo..SunFront
        Keysym::SUN_Copy => Named::Copy,
        Keysym::SUN_Open => Named::Open,
        Keysym::SUN_Paste => Named::Paste,
        Keysym::SUN_Cut => Named::Cut,

        // SunPowerSwitch
        Keysym::SUN_AudioLowerVolume => Named::AudioVolumeDown,
        Keysym::SUN_AudioMute => Named::AudioVolumeMute,
        Keysym::SUN_AudioRaiseVolume => Named::AudioVolumeUp,
        // SUN_VideoDegauss
        Keysym::SUN_VideoLowerBrightness => Named::BrightnessDown,
        Keysym::SUN_VideoRaiseBrightness => Named::BrightnessUp,
        // SunPowerSwitchShift
        //
        _ => return Key::Unidentified,
    };

    Key::Named(named)
}

fn keysym_location(keysym: Keysym) -> Location {
    match keysym {
        Keysym::Shift_L
        | Keysym::Control_L
        | Keysym::Meta_L
        | Keysym::Alt_L
        | Keysym::Super_L
        | Keysym::Hyper_L => Location::Left,
        Keysym::Shift_R
        | Keysym::Control_R
        | Keysym::Meta_R
        | Keysym::Alt_R
        | Keysym::Super_R
        | Keysym::Hyper_R => Location::Right,
        Keysym::KP_0
        | Keysym::KP_1
        | Keysym::KP_2
        | Keysym::KP_3
        | Keysym::KP_4
        | Keysym::KP_5
        | Keysym::KP_6
        | Keysym::KP_7
        | Keysym::KP_8
        | Keysym::KP_9
        | Keysym::KP_Space
        | Keysym::KP_Tab
        | Keysym::KP_Enter
        | Keysym::KP_F1
        | Keysym::KP_F2
        | Keysym::KP_F3
        | Keysym::KP_F4
        | Keysym::KP_Home
        | Keysym::KP_Left
        | Keysym::KP_Up
        | Keysym::KP_Right
        | Keysym::KP_Down
        | Keysym::KP_Page_Up
        | Keysym::KP_Page_Down
        | Keysym::KP_End
        | Keysym::KP_Begin
        | Keysym::KP_Insert
        | Keysym::KP_Delete
        | Keysym::KP_Equal
        | Keysym::KP_Multiply
        | Keysym::KP_Add
        | Keysym::KP_Separator
        | Keysym::KP_Subtract
        | Keysym::KP_Decimal
        | Keysym::KP_Divide => Location::Numpad,
        _ => Location::Standard,
    }
}

pub fn keysym_to_iced_key_and_loc(keysym: Keysym) -> (Key, Location) {
    let raw = keysym;
    let mut key = keysym_to_iced_key(keysym);
    if matches!(key, Key::Unidentified) {
        let mut utf8 = xkbcommon::xkb::keysym_to_utf8(keysym);
        utf8.pop();
        if !utf8.is_empty() {
            key = Key::Character(utf8.into());
        }
    }

    let location = keysym_location(raw);
    (key, location)
}
