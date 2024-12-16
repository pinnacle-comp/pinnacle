-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---@enum snowcap.Key
local keys = {
    NoSymbol = 0x00000000,

    VoidSymbol = 0x00ffffff,

    BackSpace = 0xff08,
    Tab = 0xff09,

    Linefeed = 0xff0a,
    Clear = 0xff0b,

    Return = 0xff0d,

    Pause = 0xff13,
    Scroll_Lock = 0xff14,
    Sys_Req = 0xff15,
    Escape = 0xff1b,

    Delete = 0xffff,

    Multi_key = 0xff20,
    Codeinput = 0xff37,
    SingleCandidate = 0xff3c,
    MultipleCandidate = 0xff3d,
    PreviousCandidate = 0xff3e,

    Kanji = 0xff21,

    Muhenkan = 0xff22,

    Henkan_Mode = 0xff23,

    Henkan = 0xff23,

    Romaji = 0xff24,

    Hiragana = 0xff25,

    Katakana = 0xff26,

    Hiragana_Katakana = 0xff27,

    Zenkaku = 0xff28,

    Hankaku = 0xff29,

    Zenkaku_Hankaku = 0xff2a,

    Touroku = 0xff2b,

    Massyo = 0xff2c,

    Kana_Lock = 0xff2d,

    Kana_Shift = 0xff2e,

    Eisu_Shift = 0xff2f,

    Eisu_toggle = 0xff30,

    Kanji_Bangou = 0xff37,

    Zen_Koho = 0xff3d,

    Mae_Koho = 0xff3e,

    Home = 0xff50,

    Left = 0xff51,

    Up = 0xff52,

    Right = 0xff53,

    Down = 0xff54,

    Prior = 0xff55,
    Page_Up = 0xff55,

    Next = 0xff56,
    Page_Down = 0xff56,

    End = 0xff57,

    Begin = 0xff58,

    Select = 0xff60,
    Print = 0xff61,

    Execute = 0xff62,

    Insert = 0xff63,
    Undo = 0xff65,

    Redo = 0xff66,
    Menu = 0xff67,

    Find = 0xff68,

    Cancel = 0xff69,

    Help = 0xff6a,
    Break = 0xff6b,

    Mode_switch = 0xff7e,

    script_switch = 0xff7e,
    Num_Lock = 0xff7f,

    KP_Space = 0xff80,
    KP_Tab = 0xff89,

    KP_Enter = 0xff8d,
    KP_F1 = 0xff91,
    KP_F2 = 0xff92,
    KP_F3 = 0xff93,
    KP_F4 = 0xff94,
    KP_Home = 0xff95,
    KP_Left = 0xff96,
    KP_Up = 0xff97,
    KP_Right = 0xff98,
    KP_Down = 0xff99,
    KP_Prior = 0xff9a,
    KP_Page_Up = 0xff9a,
    KP_Next = 0xff9b,
    KP_Page_Down = 0xff9b,
    KP_End = 0xff9c,
    KP_Begin = 0xff9d,
    KP_Insert = 0xff9e,
    KP_Delete = 0xff9f,

    KP_Equal = 0xffbd,
    KP_Multiply = 0xffaa,
    KP_Add = 0xffab,

    KP_Separator = 0xffac,
    KP_Subtract = 0xffad,
    KP_Decimal = 0xffae,
    KP_Divide = 0xffaf,

    KP_0 = 0xffb0,
    KP_1 = 0xffb1,
    KP_2 = 0xffb2,
    KP_3 = 0xffb3,
    KP_4 = 0xffb4,
    KP_5 = 0xffb5,
    KP_6 = 0xffb6,
    KP_7 = 0xffb7,
    KP_8 = 0xffb8,
    KP_9 = 0xffb9,

    F1 = 0xffbe,
    F2 = 0xffbf,
    F3 = 0xffc0,
    F4 = 0xffc1,
    F5 = 0xffc2,
    F6 = 0xffc3,
    F7 = 0xffc4,
    F8 = 0xffc5,
    F9 = 0xffc6,
    F10 = 0xffc7,
    F11 = 0xffc8,
    L1 = 0xffc8,
    F12 = 0xffc9,
    L2 = 0xffc9,
    F13 = 0xffca,
    L3 = 0xffca,
    F14 = 0xffcb,
    L4 = 0xffcb,
    F15 = 0xffcc,
    L5 = 0xffcc,
    F16 = 0xffcd,
    L6 = 0xffcd,
    F17 = 0xffce,
    L7 = 0xffce,
    F18 = 0xffcf,
    L8 = 0xffcf,
    F19 = 0xffd0,
    L9 = 0xffd0,
    F20 = 0xffd1,
    L10 = 0xffd1,
    F21 = 0xffd2,
    R1 = 0xffd2,
    F22 = 0xffd3,
    R2 = 0xffd3,
    F23 = 0xffd4,
    R3 = 0xffd4,
    F24 = 0xffd5,
    R4 = 0xffd5,
    F25 = 0xffd6,
    R5 = 0xffd6,
    F26 = 0xffd7,
    R6 = 0xffd7,
    F27 = 0xffd8,
    R7 = 0xffd8,
    F28 = 0xffd9,
    R8 = 0xffd9,
    F29 = 0xffda,
    R9 = 0xffda,
    F30 = 0xffdb,
    R10 = 0xffdb,
    F31 = 0xffdc,
    R11 = 0xffdc,
    F32 = 0xffdd,
    R12 = 0xffdd,
    F33 = 0xffde,
    R13 = 0xffde,
    F34 = 0xffdf,
    R14 = 0xffdf,
    F35 = 0xffe0,
    R15 = 0xffe0,

    Shift_L = 0xffe1,

    Shift_R = 0xffe2,

    Control_L = 0xffe3,

    Control_R = 0xffe4,

    Caps_Lock = 0xffe5,

    Shift_Lock = 0xffe6,

    Meta_L = 0xffe7,

    Meta_R = 0xffe8,

    Alt_L = 0xffe9,

    Alt_R = 0xffea,

    Super_L = 0xffeb,

    Super_R = 0xffec,

    Hyper_L = 0xffed,

    Hyper_R = 0xffee,

    ISO_Lock = 0xfe01,
    ISO_Level2_Latch = 0xfe02,
    ISO_Level3_Shift = 0xfe03,
    ISO_Level3_Latch = 0xfe04,
    ISO_Level3_Lock = 0xfe05,
    ISO_Level5_Shift = 0xfe11,
    ISO_Level5_Latch = 0xfe12,
    ISO_Level5_Lock = 0xfe13,

    ISO_Group_Shift = 0xff7e,
    ISO_Group_Latch = 0xfe06,
    ISO_Group_Lock = 0xfe07,
    ISO_Next_Group = 0xfe08,
    ISO_Next_Group_Lock = 0xfe09,
    ISO_Prev_Group = 0xfe0a,
    ISO_Prev_Group_Lock = 0xfe0b,
    ISO_First_Group = 0xfe0c,
    ISO_First_Group_Lock = 0xfe0d,
    ISO_Last_Group = 0xfe0e,
    ISO_Last_Group_Lock = 0xfe0f,

    ISO_Left_Tab = 0xfe20,
    ISO_Move_Line_Up = 0xfe21,
    ISO_Move_Line_Down = 0xfe22,
    ISO_Partial_Line_Up = 0xfe23,
    ISO_Partial_Line_Down = 0xfe24,
    ISO_Partial_Space_Left = 0xfe25,
    ISO_Partial_Space_Right = 0xfe26,
    ISO_Set_Margin_Left = 0xfe27,
    ISO_Set_Margin_Right = 0xfe28,
    ISO_Release_Margin_Left = 0xfe29,
    ISO_Release_Margin_Right = 0xfe2a,
    ISO_Release_Both_Margins = 0xfe2b,
    ISO_Fast_Cursor_Left = 0xfe2c,
    ISO_Fast_Cursor_Right = 0xfe2d,
    ISO_Fast_Cursor_Up = 0xfe2e,
    ISO_Fast_Cursor_Down = 0xfe2f,
    ISO_Continuous_Underline = 0xfe30,
    ISO_Discontinuous_Underline = 0xfe31,
    ISO_Emphasize = 0xfe32,
    ISO_Center_Object = 0xfe33,
    ISO_Enter = 0xfe34,

    dead_grave = 0xfe50,
    dead_acute = 0xfe51,
    dead_circumflex = 0xfe52,
    dead_tilde = 0xfe53,

    dead_perispomeni = 0xfe53,
    dead_macron = 0xfe54,
    dead_breve = 0xfe55,
    dead_abovedot = 0xfe56,
    dead_diaeresis = 0xfe57,
    dead_abovering = 0xfe58,
    dead_doubleacute = 0xfe59,
    dead_caron = 0xfe5a,
    dead_cedilla = 0xfe5b,
    dead_ogonek = 0xfe5c,
    dead_iota = 0xfe5d,
    dead_voiced_sound = 0xfe5e,
    dead_semivoiced_sound = 0xfe5f,
    dead_belowdot = 0xfe60,
    dead_hook = 0xfe61,
    dead_horn = 0xfe62,
    dead_stroke = 0xfe63,
    dead_abovecomma = 0xfe64,

    dead_psili = 0xfe64,
    dead_abovereversedcomma = 0xfe65,

    dead_dasia = 0xfe65,
    dead_doublegrave = 0xfe66,
    dead_belowring = 0xfe67,
    dead_belowmacron = 0xfe68,
    dead_belowcircumflex = 0xfe69,
    dead_belowtilde = 0xfe6a,
    dead_belowbreve = 0xfe6b,
    dead_belowdiaeresis = 0xfe6c,
    dead_invertedbreve = 0xfe6d,
    dead_belowcomma = 0xfe6e,
    dead_currency = 0xfe6f,

    dead_lowline = 0xfe90,
    dead_aboveverticalline = 0xfe91,
    dead_belowverticalline = 0xfe92,
    dead_longsolidusoverlay = 0xfe93,

    dead_a = 0xfe80,
    dead_A = 0xfe81,
    dead_e = 0xfe82,
    dead_E = 0xfe83,
    dead_i = 0xfe84,
    dead_I = 0xfe85,
    dead_o = 0xfe86,
    dead_O = 0xfe87,
    dead_u = 0xfe88,
    dead_U = 0xfe89,
    dead_small_schwa = 0xfe8a,
    dead_capital_schwa = 0xfe8b,

    dead_greek = 0xfe8c,

    First_Virtual_Screen = 0xfed0,
    Prev_Virtual_Screen = 0xfed1,
    Next_Virtual_Screen = 0xfed2,
    Last_Virtual_Screen = 0xfed4,
    Terminate_Server = 0xfed5,

    AccessX_Enable = 0xfe70,
    AccessX_Feedback_Enable = 0xfe71,
    RepeatKeys_Enable = 0xfe72,
    SlowKeys_Enable = 0xfe73,
    BounceKeys_Enable = 0xfe74,
    StickyKeys_Enable = 0xfe75,
    MouseKeys_Enable = 0xfe76,
    MouseKeys_Accel_Enable = 0xfe77,
    Overlay1_Enable = 0xfe78,
    Overlay2_Enable = 0xfe79,
    AudibleBell_Enable = 0xfe7a,

    Pointer_Left = 0xfee0,
    Pointer_Right = 0xfee1,
    Pointer_Up = 0xfee2,
    Pointer_Down = 0xfee3,
    Pointer_UpLeft = 0xfee4,
    Pointer_UpRight = 0xfee5,
    Pointer_DownLeft = 0xfee6,
    Pointer_DownRight = 0xfee7,
    Pointer_Button_Dflt = 0xfee8,
    Pointer_Button1 = 0xfee9,
    Pointer_Button2 = 0xfeea,
    Pointer_Button3 = 0xfeeb,
    Pointer_Button4 = 0xfeec,
    Pointer_Button5 = 0xfeed,
    Pointer_DblClick_Dflt = 0xfeee,
    Pointer_DblClick1 = 0xfeef,
    Pointer_DblClick2 = 0xfef0,
    Pointer_DblClick3 = 0xfef1,
    Pointer_DblClick4 = 0xfef2,
    Pointer_DblClick5 = 0xfef3,
    Pointer_Drag_Dflt = 0xfef4,
    Pointer_Drag1 = 0xfef5,
    Pointer_Drag2 = 0xfef6,
    Pointer_Drag3 = 0xfef7,
    Pointer_Drag4 = 0xfef8,
    Pointer_Drag5 = 0xfefd,

    Pointer_EnableKeys = 0xfef9,
    Pointer_Accelerate = 0xfefa,
    Pointer_DfltBtnNext = 0xfefb,
    Pointer_DfltBtnPrev = 0xfefc,

    ch = 0xfea0,
    Ch = 0xfea1,
    CH = 0xfea2,
    c_h = 0xfea3,
    C_h = 0xfea4,
    C_H = 0xfea5,

    KEY_3270_Duplicate = 0xfd01,
    KEY_3270_FieldMark = 0xfd02,
    KEY_3270_Right2 = 0xfd03,
    KEY_3270_Left2 = 0xfd04,
    KEY_3270_BackTab = 0xfd05,
    KEY_3270_EraseEOF = 0xfd06,
    KEY_3270_EraseInput = 0xfd07,
    KEY_3270_Reset = 0xfd08,
    KEY_3270_Quit = 0xfd09,
    KEY_3270_PA1 = 0xfd0a,
    KEY_3270_PA2 = 0xfd0b,
    KEY_3270_PA3 = 0xfd0c,
    KEY_3270_Test = 0xfd0d,
    KEY_3270_Attn = 0xfd0e,
    KEY_3270_CursorBlink = 0xfd0f,
    KEY_3270_AltCursor = 0xfd10,
    KEY_3270_KeyClick = 0xfd11,
    KEY_3270_Jump = 0xfd12,
    KEY_3270_Ident = 0xfd13,
    KEY_3270_Rule = 0xfd14,
    KEY_3270_Copy = 0xfd15,
    KEY_3270_Play = 0xfd16,
    KEY_3270_Setup = 0xfd17,
    KEY_3270_Record = 0xfd18,
    KEY_3270_ChangeScreen = 0xfd19,
    KEY_3270_DeleteWord = 0xfd1a,
    KEY_3270_ExSelect = 0xfd1b,
    KEY_3270_CursorSelect = 0xfd1c,
    KEY_3270_PrintScreen = 0xfd1d,
    KEY_3270_Enter = 0xfd1e,

    space = 0x0020,

    exclam = 0x0021,

    quotedbl = 0x0022,

    numbersign = 0x0023,

    dollar = 0x0024,

    percent = 0x0025,

    ampersand = 0x0026,

    apostrophe = 0x0027,

    quoteright = 0x0027,

    parenleft = 0x0028,

    parenright = 0x0029,

    asterisk = 0x002a,

    plus = 0x002b,

    comma = 0x002c,

    minus = 0x002d,

    period = 0x002e,

    slash = 0x002f,

    KEY_0 = 0x0030,

    KEY_1 = 0x0031,

    KEY_2 = 0x0032,

    KEY_3 = 0x0033,

    KEY_4 = 0x0034,

    KEY_5 = 0x0035,

    KEY_6 = 0x0036,

    KEY_7 = 0x0037,

    KEY_8 = 0x0038,

    KEY_9 = 0x0039,

    colon = 0x003a,

    semicolon = 0x003b,

    less = 0x003c,

    equal = 0x003d,

    greater = 0x003e,

    question = 0x003f,

    at = 0x0040,

    A = 0x0041,

    B = 0x0042,

    C = 0x0043,

    D = 0x0044,

    E = 0x0045,

    F = 0x0046,

    G = 0x0047,

    H = 0x0048,

    I = 0x0049,

    J = 0x004a,

    K = 0x004b,

    L = 0x004c,

    M = 0x004d,

    N = 0x004e,

    O = 0x004f,

    P = 0x0050,

    Q = 0x0051,

    R = 0x0052,

    S = 0x0053,

    T = 0x0054,

    U = 0x0055,

    V = 0x0056,

    W = 0x0057,

    X = 0x0058,

    Y = 0x0059,

    Z = 0x005a,

    bracketleft = 0x005b,

    backslash = 0x005c,

    bracketright = 0x005d,

    asciicircum = 0x005e,

    underscore = 0x005f,

    grave = 0x0060,

    quoteleft = 0x0060,

    a = 0x0061,

    b = 0x0062,

    c = 0x0063,

    d = 0x0064,

    e = 0x0065,

    f = 0x0066,

    g = 0x0067,

    h = 0x0068,

    i = 0x0069,

    j = 0x006a,

    k = 0x006b,

    l = 0x006c,

    m = 0x006d,

    n = 0x006e,

    o = 0x006f,

    p = 0x0070,

    q = 0x0071,

    r = 0x0072,

    s = 0x0073,

    t = 0x0074,

    u = 0x0075,

    v = 0x0076,

    w = 0x0077,

    x = 0x0078,

    y = 0x0079,

    z = 0x007a,

    braceleft = 0x007b,

    bar = 0x007c,

    braceright = 0x007d,

    asciitilde = 0x007e,

    nobreakspace = 0x00a0,

    exclamdown = 0x00a1,

    cent = 0x00a2,

    sterling = 0x00a3,

    currency = 0x00a4,

    yen = 0x00a5,

    brokenbar = 0x00a6,

    section = 0x00a7,

    diaeresis = 0x00a8,

    copyright = 0x00a9,

    ordfeminine = 0x00aa,

    guillemotleft = 0x00ab,

    notsign = 0x00ac,

    hyphen = 0x00ad,

    registered = 0x00ae,

    macron = 0x00af,

    degree = 0x00b0,

    plusminus = 0x00b1,

    twosuperior = 0x00b2,

    threesuperior = 0x00b3,

    acute = 0x00b4,

    mu = 0x00b5,

    paragraph = 0x00b6,

    periodcentered = 0x00b7,

    cedilla = 0x00b8,

    onesuperior = 0x00b9,

    masculine = 0x00ba,

    guillemotright = 0x00bb,

    onequarter = 0x00bc,

    onehalf = 0x00bd,

    threequarters = 0x00be,

    questiondown = 0x00bf,

    Agrave = 0x00c0,

    Aacute = 0x00c1,

    Acircumflex = 0x00c2,

    Atilde = 0x00c3,

    Adiaeresis = 0x00c4,

    Aring = 0x00c5,

    AE = 0x00c6,

    Ccedilla = 0x00c7,

    Egrave = 0x00c8,

    Eacute = 0x00c9,

    Ecircumflex = 0x00ca,

    Ediaeresis = 0x00cb,

    Igrave = 0x00cc,

    Iacute = 0x00cd,

    Icircumflex = 0x00ce,

    Idiaeresis = 0x00cf,

    ETH = 0x00d0,

    Eth = 0x00d0,

    Ntilde = 0x00d1,

    Ograve = 0x00d2,

    Oacute = 0x00d3,

    Ocircumflex = 0x00d4,

    Otilde = 0x00d5,

    Odiaeresis = 0x00d6,

    multiply = 0x00d7,

    Oslash = 0x00d8,

    Ooblique = 0x00d8,

    Ugrave = 0x00d9,

    Uacute = 0x00da,

    Ucircumflex = 0x00db,

    Udiaeresis = 0x00dc,

    Yacute = 0x00dd,

    THORN = 0x00de,

    Thorn = 0x00de,

    ssharp = 0x00df,

    agrave = 0x00e0,

    aacute = 0x00e1,

    acircumflex = 0x00e2,

    atilde = 0x00e3,

    adiaeresis = 0x00e4,

    aring = 0x00e5,

    ae = 0x00e6,

    ccedilla = 0x00e7,

    egrave = 0x00e8,

    eacute = 0x00e9,

    ecircumflex = 0x00ea,

    ediaeresis = 0x00eb,

    igrave = 0x00ec,

    iacute = 0x00ed,

    icircumflex = 0x00ee,

    idiaeresis = 0x00ef,

    eth = 0x00f0,

    ntilde = 0x00f1,

    ograve = 0x00f2,

    oacute = 0x00f3,

    ocircumflex = 0x00f4,

    otilde = 0x00f5,

    odiaeresis = 0x00f6,

    division = 0x00f7,

    oslash = 0x00f8,

    ooblique = 0x00f8,

    ugrave = 0x00f9,

    uacute = 0x00fa,

    ucircumflex = 0x00fb,

    udiaeresis = 0x00fc,

    yacute = 0x00fd,

    thorn = 0x00fe,

    ydiaeresis = 0x00ff,

    Aogonek = 0x01a1,

    breve = 0x01a2,

    Lstroke = 0x01a3,

    Lcaron = 0x01a5,

    Sacute = 0x01a6,

    Scaron = 0x01a9,

    Scedilla = 0x01aa,

    Tcaron = 0x01ab,

    Zacute = 0x01ac,

    Zcaron = 0x01ae,

    Zabovedot = 0x01af,

    aogonek = 0x01b1,

    ogonek = 0x01b2,

    lstroke = 0x01b3,

    lcaron = 0x01b5,

    sacute = 0x01b6,

    caron = 0x01b7,

    scaron = 0x01b9,

    scedilla = 0x01ba,

    tcaron = 0x01bb,

    zacute = 0x01bc,

    doubleacute = 0x01bd,

    zcaron = 0x01be,

    zabovedot = 0x01bf,

    Racute = 0x01c0,

    Abreve = 0x01c3,

    Lacute = 0x01c5,

    Cacute = 0x01c6,

    Ccaron = 0x01c8,

    Eogonek = 0x01ca,

    Ecaron = 0x01cc,

    Dcaron = 0x01cf,

    Dstroke = 0x01d0,

    Nacute = 0x01d1,

    Ncaron = 0x01d2,

    Odoubleacute = 0x01d5,

    Rcaron = 0x01d8,

    Uring = 0x01d9,

    Udoubleacute = 0x01db,

    Tcedilla = 0x01de,

    racute = 0x01e0,

    abreve = 0x01e3,

    lacute = 0x01e5,

    cacute = 0x01e6,

    ccaron = 0x01e8,

    eogonek = 0x01ea,

    ecaron = 0x01ec,

    dcaron = 0x01ef,

    dstroke = 0x01f0,

    nacute = 0x01f1,

    ncaron = 0x01f2,

    odoubleacute = 0x01f5,

    rcaron = 0x01f8,

    uring = 0x01f9,

    udoubleacute = 0x01fb,

    tcedilla = 0x01fe,

    abovedot = 0x01ff,

    Hstroke = 0x02a1,

    Hcircumflex = 0x02a6,

    Iabovedot = 0x02a9,

    Gbreve = 0x02ab,

    Jcircumflex = 0x02ac,

    hstroke = 0x02b1,

    hcircumflex = 0x02b6,

    idotless = 0x02b9,

    gbreve = 0x02bb,

    jcircumflex = 0x02bc,

    Cabovedot = 0x02c5,

    Ccircumflex = 0x02c6,

    Gabovedot = 0x02d5,

    Gcircumflex = 0x02d8,

    Ubreve = 0x02dd,

    Scircumflex = 0x02de,

    cabovedot = 0x02e5,

    ccircumflex = 0x02e6,

    gabovedot = 0x02f5,

    gcircumflex = 0x02f8,

    ubreve = 0x02fd,

    scircumflex = 0x02fe,

    kra = 0x03a2,

    kappa = 0x03a2,

    Rcedilla = 0x03a3,

    Itilde = 0x03a5,

    Lcedilla = 0x03a6,

    Emacron = 0x03aa,

    Gcedilla = 0x03ab,

    Tslash = 0x03ac,

    rcedilla = 0x03b3,

    itilde = 0x03b5,

    lcedilla = 0x03b6,

    emacron = 0x03ba,

    gcedilla = 0x03bb,

    tslash = 0x03bc,

    ENG = 0x03bd,

    eng = 0x03bf,

    Amacron = 0x03c0,

    Iogonek = 0x03c7,

    Eabovedot = 0x03cc,

    Imacron = 0x03cf,

    Ncedilla = 0x03d1,

    Omacron = 0x03d2,

    Kcedilla = 0x03d3,

    Uogonek = 0x03d9,

    Utilde = 0x03dd,

    Umacron = 0x03de,

    amacron = 0x03e0,

    iogonek = 0x03e7,

    eabovedot = 0x03ec,

    imacron = 0x03ef,

    ncedilla = 0x03f1,

    omacron = 0x03f2,

    kcedilla = 0x03f3,

    uogonek = 0x03f9,

    utilde = 0x03fd,

    umacron = 0x03fe,

    Wcircumflex = 0x01000174,

    wcircumflex = 0x01000175,

    Ycircumflex = 0x01000176,

    ycircumflex = 0x01000177,

    Babovedot = 0x01001e02,

    babovedot = 0x01001e03,

    Dabovedot = 0x01001e0a,

    dabovedot = 0x01001e0b,

    Fabovedot = 0x01001e1e,

    fabovedot = 0x01001e1f,

    Mabovedot = 0x01001e40,

    mabovedot = 0x01001e41,

    Pabovedot = 0x01001e56,

    pabovedot = 0x01001e57,

    Sabovedot = 0x01001e60,

    sabovedot = 0x01001e61,

    Tabovedot = 0x01001e6a,

    tabovedot = 0x01001e6b,

    Wgrave = 0x01001e80,

    wgrave = 0x01001e81,

    Wacute = 0x01001e82,

    wacute = 0x01001e83,

    Wdiaeresis = 0x01001e84,

    wdiaeresis = 0x01001e85,

    Ygrave = 0x01001ef2,

    ygrave = 0x01001ef3,

    OE = 0x13bc,

    oe = 0x13bd,

    Ydiaeresis = 0x13be,

    overline = 0x047e,

    kana_fullstop = 0x04a1,

    kana_openingbracket = 0x04a2,

    kana_closingbracket = 0x04a3,

    kana_comma = 0x04a4,

    kana_conjunctive = 0x04a5,

    kana_middledot = 0x04a5,

    kana_WO = 0x04a6,

    kana_a = 0x04a7,

    kana_i = 0x04a8,

    kana_u = 0x04a9,

    kana_e = 0x04aa,

    kana_o = 0x04ab,

    kana_ya = 0x04ac,

    kana_yu = 0x04ad,

    kana_yo = 0x04ae,

    kana_tsu = 0x04af,

    kana_tu = 0x04af,

    prolongedsound = 0x04b0,

    kana_A = 0x04b1,

    kana_I = 0x04b2,

    kana_U = 0x04b3,

    kana_E = 0x04b4,

    kana_O = 0x04b5,

    kana_KA = 0x04b6,

    kana_KI = 0x04b7,

    kana_KU = 0x04b8,

    kana_KE = 0x04b9,

    kana_KO = 0x04ba,

    kana_SA = 0x04bb,

    kana_SHI = 0x04bc,

    kana_SU = 0x04bd,

    kana_SE = 0x04be,

    kana_SO = 0x04bf,

    kana_TA = 0x04c0,

    kana_CHI = 0x04c1,

    kana_TI = 0x04c1,

    kana_TSU = 0x04c2,

    kana_TU = 0x04c2,

    kana_TE = 0x04c3,

    kana_TO = 0x04c4,

    kana_NA = 0x04c5,

    kana_NI = 0x04c6,

    kana_NU = 0x04c7,

    kana_NE = 0x04c8,

    kana_NO = 0x04c9,

    kana_HA = 0x04ca,

    kana_HI = 0x04cb,

    kana_FU = 0x04cc,

    kana_HU = 0x04cc,

    kana_HE = 0x04cd,

    kana_HO = 0x04ce,

    kana_MA = 0x04cf,

    kana_MI = 0x04d0,

    kana_MU = 0x04d1,

    kana_ME = 0x04d2,

    kana_MO = 0x04d3,

    kana_YA = 0x04d4,

    kana_YU = 0x04d5,

    kana_YO = 0x04d6,

    kana_RA = 0x04d7,

    kana_RI = 0x04d8,

    kana_RU = 0x04d9,

    kana_RE = 0x04da,

    kana_RO = 0x04db,

    kana_WA = 0x04dc,

    kana_N = 0x04dd,

    voicedsound = 0x04de,

    semivoicedsound = 0x04df,

    kana_switch = 0xff7e,

    Farsi_0 = 0x010006f0,

    Farsi_1 = 0x010006f1,

    Farsi_2 = 0x010006f2,

    Farsi_3 = 0x010006f3,

    Farsi_4 = 0x010006f4,

    Farsi_5 = 0x010006f5,

    Farsi_6 = 0x010006f6,

    Farsi_7 = 0x010006f7,

    Farsi_8 = 0x010006f8,

    Farsi_9 = 0x010006f9,

    Arabic_percent = 0x0100066a,

    Arabic_superscript_alef = 0x01000670,

    Arabic_tteh = 0x01000679,

    Arabic_peh = 0x0100067e,

    Arabic_tcheh = 0x01000686,

    Arabic_ddal = 0x01000688,

    Arabic_rreh = 0x01000691,

    Arabic_comma = 0x05ac,

    Arabic_fullstop = 0x010006d4,

    Arabic_0 = 0x01000660,

    Arabic_1 = 0x01000661,

    Arabic_2 = 0x01000662,

    Arabic_3 = 0x01000663,

    Arabic_4 = 0x01000664,

    Arabic_5 = 0x01000665,

    Arabic_6 = 0x01000666,

    Arabic_7 = 0x01000667,

    Arabic_8 = 0x01000668,

    Arabic_9 = 0x01000669,

    Arabic_semicolon = 0x05bb,

    Arabic_question_mark = 0x05bf,

    Arabic_hamza = 0x05c1,

    Arabic_maddaonalef = 0x05c2,

    Arabic_hamzaonalef = 0x05c3,

    Arabic_hamzaonwaw = 0x05c4,

    Arabic_hamzaunderalef = 0x05c5,

    Arabic_hamzaonyeh = 0x05c6,

    Arabic_alef = 0x05c7,

    Arabic_beh = 0x05c8,

    Arabic_tehmarbuta = 0x05c9,

    Arabic_teh = 0x05ca,

    Arabic_theh = 0x05cb,

    Arabic_jeem = 0x05cc,

    Arabic_hah = 0x05cd,

    Arabic_khah = 0x05ce,

    Arabic_dal = 0x05cf,

    Arabic_thal = 0x05d0,

    Arabic_ra = 0x05d1,

    Arabic_zain = 0x05d2,

    Arabic_seen = 0x05d3,

    Arabic_sheen = 0x05d4,

    Arabic_sad = 0x05d5,

    Arabic_dad = 0x05d6,

    Arabic_tah = 0x05d7,

    Arabic_zah = 0x05d8,

    Arabic_ain = 0x05d9,

    Arabic_ghain = 0x05da,

    Arabic_tatweel = 0x05e0,

    Arabic_feh = 0x05e1,

    Arabic_qaf = 0x05e2,

    Arabic_kaf = 0x05e3,

    Arabic_lam = 0x05e4,

    Arabic_meem = 0x05e5,

    Arabic_noon = 0x05e6,

    Arabic_ha = 0x05e7,

    Arabic_heh = 0x05e7,

    Arabic_waw = 0x05e8,

    Arabic_alefmaksura = 0x05e9,

    Arabic_yeh = 0x05ea,

    Arabic_fathatan = 0x05eb,

    Arabic_dammatan = 0x05ec,

    Arabic_kasratan = 0x05ed,

    Arabic_fatha = 0x05ee,

    Arabic_damma = 0x05ef,

    Arabic_kasra = 0x05f0,

    Arabic_shadda = 0x05f1,

    Arabic_sukun = 0x05f2,

    Arabic_madda_above = 0x01000653,

    Arabic_hamza_above = 0x01000654,

    Arabic_hamza_below = 0x01000655,

    Arabic_jeh = 0x01000698,

    Arabic_veh = 0x010006a4,

    Arabic_keheh = 0x010006a9,

    Arabic_gaf = 0x010006af,

    Arabic_noon_ghunna = 0x010006ba,

    Arabic_heh_doachashmee = 0x010006be,

    Farsi_yeh = 0x010006cc,

    Arabic_farsi_yeh = 0x010006cc,

    Arabic_yeh_baree = 0x010006d2,

    Arabic_heh_goal = 0x010006c1,

    Arabic_switch = 0xff7e,

    Cyrillic_GHE_bar = 0x01000492,

    Cyrillic_ghe_bar = 0x01000493,

    Cyrillic_ZHE_descender = 0x01000496,

    Cyrillic_zhe_descender = 0x01000497,

    Cyrillic_KA_descender = 0x0100049a,

    Cyrillic_ka_descender = 0x0100049b,

    Cyrillic_KA_vertstroke = 0x0100049c,

    Cyrillic_ka_vertstroke = 0x0100049d,

    Cyrillic_EN_descender = 0x010004a2,

    Cyrillic_en_descender = 0x010004a3,

    Cyrillic_U_straight = 0x010004ae,

    Cyrillic_u_straight = 0x010004af,

    Cyrillic_U_straight_bar = 0x010004b0,

    Cyrillic_u_straight_bar = 0x010004b1,

    Cyrillic_HA_descender = 0x010004b2,

    Cyrillic_ha_descender = 0x010004b3,

    Cyrillic_CHE_descender = 0x010004b6,

    Cyrillic_che_descender = 0x010004b7,

    Cyrillic_CHE_vertstroke = 0x010004b8,

    Cyrillic_che_vertstroke = 0x010004b9,

    Cyrillic_SHHA = 0x010004ba,

    Cyrillic_shha = 0x010004bb,

    Cyrillic_SCHWA = 0x010004d8,

    Cyrillic_schwa = 0x010004d9,

    Cyrillic_I_macron = 0x010004e2,

    Cyrillic_i_macron = 0x010004e3,

    Cyrillic_O_bar = 0x010004e8,

    Cyrillic_o_bar = 0x010004e9,

    Cyrillic_U_macron = 0x010004ee,

    Cyrillic_u_macron = 0x010004ef,

    Serbian_dje = 0x06a1,

    Macedonia_gje = 0x06a2,

    Cyrillic_io = 0x06a3,

    Ukrainian_ie = 0x06a4,

    Ukranian_je = 0x06a4,

    Macedonia_dse = 0x06a5,

    Ukrainian_i = 0x06a6,

    Ukranian_i = 0x06a6,

    Ukrainian_yi = 0x06a7,

    Ukranian_yi = 0x06a7,

    Cyrillic_je = 0x06a8,

    Serbian_je = 0x06a8,

    Cyrillic_lje = 0x06a9,

    Serbian_lje = 0x06a9,

    Cyrillic_nje = 0x06aa,

    Serbian_nje = 0x06aa,

    Serbian_tshe = 0x06ab,

    Macedonia_kje = 0x06ac,

    Ukrainian_ghe_with_upturn = 0x06ad,

    Byelorussian_shortu = 0x06ae,

    Cyrillic_dzhe = 0x06af,

    Serbian_dze = 0x06af,

    numerosign = 0x06b0,

    Serbian_DJE = 0x06b1,

    Macedonia_GJE = 0x06b2,

    Cyrillic_IO = 0x06b3,

    Ukrainian_IE = 0x06b4,

    Ukranian_JE = 0x06b4,

    Macedonia_DSE = 0x06b5,

    Ukrainian_I = 0x06b6,

    Ukranian_I = 0x06b6,

    Ukrainian_YI = 0x06b7,

    Ukranian_YI = 0x06b7,

    Cyrillic_JE = 0x06b8,

    Serbian_JE = 0x06b8,

    Cyrillic_LJE = 0x06b9,

    Serbian_LJE = 0x06b9,

    Cyrillic_NJE = 0x06ba,

    Serbian_NJE = 0x06ba,

    Serbian_TSHE = 0x06bb,

    Macedonia_KJE = 0x06bc,

    Ukrainian_GHE_WITH_UPTURN = 0x06bd,

    Byelorussian_SHORTU = 0x06be,

    Cyrillic_DZHE = 0x06bf,

    Serbian_DZE = 0x06bf,

    Cyrillic_yu = 0x06c0,

    Cyrillic_a = 0x06c1,

    Cyrillic_be = 0x06c2,

    Cyrillic_tse = 0x06c3,

    Cyrillic_de = 0x06c4,

    Cyrillic_ie = 0x06c5,

    Cyrillic_ef = 0x06c6,

    Cyrillic_ghe = 0x06c7,

    Cyrillic_ha = 0x06c8,

    Cyrillic_i = 0x06c9,

    Cyrillic_shorti = 0x06ca,

    Cyrillic_ka = 0x06cb,

    Cyrillic_el = 0x06cc,

    Cyrillic_em = 0x06cd,

    Cyrillic_en = 0x06ce,

    Cyrillic_o = 0x06cf,

    Cyrillic_pe = 0x06d0,

    Cyrillic_ya = 0x06d1,

    Cyrillic_er = 0x06d2,

    Cyrillic_es = 0x06d3,

    Cyrillic_te = 0x06d4,

    Cyrillic_u = 0x06d5,

    Cyrillic_zhe = 0x06d6,

    Cyrillic_ve = 0x06d7,

    Cyrillic_softsign = 0x06d8,

    Cyrillic_yeru = 0x06d9,

    Cyrillic_ze = 0x06da,

    Cyrillic_sha = 0x06db,

    Cyrillic_e = 0x06dc,

    Cyrillic_shcha = 0x06dd,

    Cyrillic_che = 0x06de,

    Cyrillic_hardsign = 0x06df,

    Cyrillic_YU = 0x06e0,

    Cyrillic_A = 0x06e1,

    Cyrillic_BE = 0x06e2,

    Cyrillic_TSE = 0x06e3,

    Cyrillic_DE = 0x06e4,

    Cyrillic_IE = 0x06e5,

    Cyrillic_EF = 0x06e6,

    Cyrillic_GHE = 0x06e7,

    Cyrillic_HA = 0x06e8,

    Cyrillic_I = 0x06e9,

    Cyrillic_SHORTI = 0x06ea,

    Cyrillic_KA = 0x06eb,

    Cyrillic_EL = 0x06ec,

    Cyrillic_EM = 0x06ed,

    Cyrillic_EN = 0x06ee,

    Cyrillic_O = 0x06ef,

    Cyrillic_PE = 0x06f0,

    Cyrillic_YA = 0x06f1,

    Cyrillic_ER = 0x06f2,

    Cyrillic_ES = 0x06f3,

    Cyrillic_TE = 0x06f4,

    Cyrillic_U = 0x06f5,

    Cyrillic_ZHE = 0x06f6,

    Cyrillic_VE = 0x06f7,

    Cyrillic_SOFTSIGN = 0x06f8,

    Cyrillic_YERU = 0x06f9,

    Cyrillic_ZE = 0x06fa,

    Cyrillic_SHA = 0x06fb,

    Cyrillic_E = 0x06fc,

    Cyrillic_SHCHA = 0x06fd,

    Cyrillic_CHE = 0x06fe,

    Cyrillic_HARDSIGN = 0x06ff,

    Greek_ALPHAaccent = 0x07a1,

    Greek_EPSILONaccent = 0x07a2,

    Greek_ETAaccent = 0x07a3,

    Greek_IOTAaccent = 0x07a4,

    Greek_IOTAdieresis = 0x07a5,

    Greek_IOTAdiaeresis = 0x07a5,

    Greek_OMICRONaccent = 0x07a7,

    Greek_UPSILONaccent = 0x07a8,

    Greek_UPSILONdieresis = 0x07a9,

    Greek_OMEGAaccent = 0x07ab,

    Greek_accentdieresis = 0x07ae,

    Greek_horizbar = 0x07af,

    Greek_alphaaccent = 0x07b1,

    Greek_epsilonaccent = 0x07b2,

    Greek_etaaccent = 0x07b3,

    Greek_iotaaccent = 0x07b4,

    Greek_iotadieresis = 0x07b5,

    Greek_iotaaccentdieresis = 0x07b6,

    Greek_omicronaccent = 0x07b7,

    Greek_upsilonaccent = 0x07b8,

    Greek_upsilondieresis = 0x07b9,

    Greek_upsilonaccentdieresis = 0x07ba,

    Greek_omegaaccent = 0x07bb,

    Greek_ALPHA = 0x07c1,

    Greek_BETA = 0x07c2,

    Greek_GAMMA = 0x07c3,

    Greek_DELTA = 0x07c4,

    Greek_EPSILON = 0x07c5,

    Greek_ZETA = 0x07c6,

    Greek_ETA = 0x07c7,

    Greek_THETA = 0x07c8,

    Greek_IOTA = 0x07c9,

    Greek_KAPPA = 0x07ca,

    Greek_LAMDA = 0x07cb,

    Greek_LAMBDA = 0x07cb,

    Greek_MU = 0x07cc,

    Greek_NU = 0x07cd,

    Greek_XI = 0x07ce,

    Greek_OMICRON = 0x07cf,

    Greek_PI = 0x07d0,

    Greek_RHO = 0x07d1,

    Greek_SIGMA = 0x07d2,

    Greek_TAU = 0x07d4,

    Greek_UPSILON = 0x07d5,

    Greek_PHI = 0x07d6,

    Greek_CHI = 0x07d7,

    Greek_PSI = 0x07d8,

    Greek_OMEGA = 0x07d9,

    Greek_alpha = 0x07e1,

    Greek_beta = 0x07e2,

    Greek_gamma = 0x07e3,

    Greek_delta = 0x07e4,

    Greek_epsilon = 0x07e5,

    Greek_zeta = 0x07e6,

    Greek_eta = 0x07e7,

    Greek_theta = 0x07e8,

    Greek_iota = 0x07e9,

    Greek_kappa = 0x07ea,

    Greek_lamda = 0x07eb,

    Greek_lambda = 0x07eb,

    Greek_mu = 0x07ec,

    Greek_nu = 0x07ed,

    Greek_xi = 0x07ee,

    Greek_omicron = 0x07ef,

    Greek_pi = 0x07f0,

    Greek_rho = 0x07f1,

    Greek_sigma = 0x07f2,

    Greek_finalsmallsigma = 0x07f3,

    Greek_tau = 0x07f4,

    Greek_upsilon = 0x07f5,

    Greek_phi = 0x07f6,

    Greek_chi = 0x07f7,

    Greek_psi = 0x07f8,

    Greek_omega = 0x07f9,

    Greek_switch = 0xff7e,

    leftradical = 0x08a1,

    topleftradical = 0x08a2,

    horizconnector = 0x08a3,

    topintegral = 0x08a4,

    botintegral = 0x08a5,

    vertconnector = 0x08a6,

    topleftsqbracket = 0x08a7,

    botleftsqbracket = 0x08a8,

    toprightsqbracket = 0x08a9,

    botrightsqbracket = 0x08aa,

    topleftparens = 0x08ab,

    botleftparens = 0x08ac,

    toprightparens = 0x08ad,

    botrightparens = 0x08ae,

    leftmiddlecurlybrace = 0x08af,

    rightmiddlecurlybrace = 0x08b0,
    topleftsummation = 0x08b1,
    botleftsummation = 0x08b2,
    topvertsummationconnector = 0x08b3,
    botvertsummationconnector = 0x08b4,
    toprightsummation = 0x08b5,
    botrightsummation = 0x08b6,
    rightmiddlesummation = 0x08b7,

    lessthanequal = 0x08bc,

    notequal = 0x08bd,

    greaterthanequal = 0x08be,

    integral = 0x08bf,

    therefore = 0x08c0,

    variation = 0x08c1,

    infinity = 0x08c2,

    nabla = 0x08c5,

    approximate = 0x08c8,

    similarequal = 0x08c9,

    ifonlyif = 0x08cd,

    implies = 0x08ce,

    identical = 0x08cf,

    radical = 0x08d6,

    includedin = 0x08da,

    includes = 0x08db,

    intersection = 0x08dc,

    union = 0x08dd,

    logicaland = 0x08de,

    logicalor = 0x08df,

    partialderivative = 0x08ef,

    KEY_function = 0x08f6,

    leftarrow = 0x08fb,

    uparrow = 0x08fc,

    rightarrow = 0x08fd,

    downarrow = 0x08fe,

    blank = 0x09df,

    soliddiamond = 0x09e0,

    checkerboard = 0x09e1,

    ht = 0x09e2,

    ff = 0x09e3,

    cr = 0x09e4,

    lf = 0x09e5,

    nl = 0x09e8,

    vt = 0x09e9,

    lowrightcorner = 0x09ea,

    uprightcorner = 0x09eb,

    upleftcorner = 0x09ec,

    lowleftcorner = 0x09ed,

    crossinglines = 0x09ee,

    horizlinescan1 = 0x09ef,

    horizlinescan3 = 0x09f0,

    horizlinescan5 = 0x09f1,

    horizlinescan7 = 0x09f2,

    horizlinescan9 = 0x09f3,

    leftt = 0x09f4,

    rightt = 0x09f5,

    bott = 0x09f6,

    topt = 0x09f7,

    vertbar = 0x09f8,

    emspace = 0x0aa1,

    enspace = 0x0aa2,

    em3space = 0x0aa3,

    em4space = 0x0aa4,

    digitspace = 0x0aa5,

    punctspace = 0x0aa6,

    thinspace = 0x0aa7,

    hairspace = 0x0aa8,

    emdash = 0x0aa9,

    endash = 0x0aaa,

    signifblank = 0x0aac,

    ellipsis = 0x0aae,

    doubbaselinedot = 0x0aaf,

    onethird = 0x0ab0,

    twothirds = 0x0ab1,

    onefifth = 0x0ab2,

    twofifths = 0x0ab3,

    threefifths = 0x0ab4,

    fourfifths = 0x0ab5,

    onesixth = 0x0ab6,

    fivesixths = 0x0ab7,

    careof = 0x0ab8,

    figdash = 0x0abb,

    leftanglebracket = 0x0abc,

    decimalpoint = 0x0abd,

    rightanglebracket = 0x0abe,
    marker = 0x0abf,

    oneeighth = 0x0ac3,

    threeeighths = 0x0ac4,

    fiveeighths = 0x0ac5,

    seveneighths = 0x0ac6,

    trademark = 0x0ac9,

    signaturemark = 0x0aca,
    trademarkincircle = 0x0acb,

    leftopentriangle = 0x0acc,

    rightopentriangle = 0x0acd,

    emopencircle = 0x0ace,

    emopenrectangle = 0x0acf,

    leftsinglequotemark = 0x0ad0,

    rightsinglequotemark = 0x0ad1,

    leftdoublequotemark = 0x0ad2,

    rightdoublequotemark = 0x0ad3,

    prescription = 0x0ad4,

    permille = 0x0ad5,

    minutes = 0x0ad6,

    seconds = 0x0ad7,

    latincross = 0x0ad9,
    hexagram = 0x0ada,

    filledrectbullet = 0x0adb,

    filledlefttribullet = 0x0adc,

    filledrighttribullet = 0x0add,

    emfilledcircle = 0x0ade,

    emfilledrect = 0x0adf,

    enopencircbullet = 0x0ae0,

    enopensquarebullet = 0x0ae1,

    openrectbullet = 0x0ae2,

    opentribulletup = 0x0ae3,

    opentribulletdown = 0x0ae4,

    openstar = 0x0ae5,

    enfilledcircbullet = 0x0ae6,

    enfilledsqbullet = 0x0ae7,

    filledtribulletup = 0x0ae8,

    filledtribulletdown = 0x0ae9,

    leftpointer = 0x0aea,

    rightpointer = 0x0aeb,

    club = 0x0aec,

    diamond = 0x0aed,

    heart = 0x0aee,

    maltesecross = 0x0af0,

    dagger = 0x0af1,

    doubledagger = 0x0af2,

    checkmark = 0x0af3,

    ballotcross = 0x0af4,

    musicalsharp = 0x0af5,

    musicalflat = 0x0af6,

    malesymbol = 0x0af7,

    femalesymbol = 0x0af8,

    telephone = 0x0af9,

    telephonerecorder = 0x0afa,

    phonographcopyright = 0x0afb,

    caret = 0x0afc,

    singlelowquotemark = 0x0afd,

    doublelowquotemark = 0x0afe,
    cursor = 0x0aff,

    leftcaret = 0x0ba3,

    rightcaret = 0x0ba6,

    downcaret = 0x0ba8,

    upcaret = 0x0ba9,

    overbar = 0x0bc0,

    downtack = 0x0bc2,

    upshoe = 0x0bc3,

    downstile = 0x0bc4,

    underbar = 0x0bc6,

    jot = 0x0bca,

    quad = 0x0bcc,

    uptack = 0x0bce,

    circle = 0x0bcf,

    upstile = 0x0bd3,

    downshoe = 0x0bd6,

    rightshoe = 0x0bd8,

    leftshoe = 0x0bda,

    lefttack = 0x0bdc,

    righttack = 0x0bfc,

    hebrew_doublelowline = 0x0cdf,

    hebrew_aleph = 0x0ce0,

    hebrew_bet = 0x0ce1,

    hebrew_beth = 0x0ce1,

    hebrew_gimel = 0x0ce2,

    hebrew_gimmel = 0x0ce2,

    hebrew_dalet = 0x0ce3,

    hebrew_daleth = 0x0ce3,

    hebrew_he = 0x0ce4,

    hebrew_waw = 0x0ce5,

    hebrew_zain = 0x0ce6,

    hebrew_zayin = 0x0ce6,

    hebrew_chet = 0x0ce7,

    hebrew_het = 0x0ce7,

    hebrew_tet = 0x0ce8,

    hebrew_teth = 0x0ce8,

    hebrew_yod = 0x0ce9,

    hebrew_finalkaph = 0x0cea,

    hebrew_kaph = 0x0ceb,

    hebrew_lamed = 0x0cec,

    hebrew_finalmem = 0x0ced,

    hebrew_mem = 0x0cee,

    hebrew_finalnun = 0x0cef,

    hebrew_nun = 0x0cf0,

    hebrew_samech = 0x0cf1,

    hebrew_samekh = 0x0cf1,

    hebrew_ayin = 0x0cf2,

    hebrew_finalpe = 0x0cf3,

    hebrew_pe = 0x0cf4,

    hebrew_finalzade = 0x0cf5,

    hebrew_finalzadi = 0x0cf5,

    hebrew_zade = 0x0cf6,

    hebrew_zadi = 0x0cf6,

    hebrew_qoph = 0x0cf7,

    hebrew_kuf = 0x0cf7,

    hebrew_resh = 0x0cf8,

    hebrew_shin = 0x0cf9,

    hebrew_taw = 0x0cfa,

    hebrew_taf = 0x0cfa,

    Hebrew_switch = 0xff7e,

    Thai_kokai = 0x0da1,

    Thai_khokhai = 0x0da2,

    Thai_khokhuat = 0x0da3,

    Thai_khokhwai = 0x0da4,

    Thai_khokhon = 0x0da5,

    Thai_khorakhang = 0x0da6,

    Thai_ngongu = 0x0da7,

    Thai_chochan = 0x0da8,

    Thai_choching = 0x0da9,

    Thai_chochang = 0x0daa,

    Thai_soso = 0x0dab,

    Thai_chochoe = 0x0dac,

    Thai_yoying = 0x0dad,

    Thai_dochada = 0x0dae,

    Thai_topatak = 0x0daf,

    Thai_thothan = 0x0db0,

    Thai_thonangmontho = 0x0db1,

    Thai_thophuthao = 0x0db2,

    Thai_nonen = 0x0db3,

    Thai_dodek = 0x0db4,

    Thai_totao = 0x0db5,

    Thai_thothung = 0x0db6,

    Thai_thothahan = 0x0db7,

    Thai_thothong = 0x0db8,

    Thai_nonu = 0x0db9,

    Thai_bobaimai = 0x0dba,

    Thai_popla = 0x0dbb,

    Thai_phophung = 0x0dbc,

    Thai_fofa = 0x0dbd,

    Thai_phophan = 0x0dbe,

    Thai_fofan = 0x0dbf,

    Thai_phosamphao = 0x0dc0,

    Thai_moma = 0x0dc1,

    Thai_yoyak = 0x0dc2,

    Thai_rorua = 0x0dc3,

    Thai_ru = 0x0dc4,

    Thai_loling = 0x0dc5,

    Thai_lu = 0x0dc6,

    Thai_wowaen = 0x0dc7,

    Thai_sosala = 0x0dc8,

    Thai_sorusi = 0x0dc9,

    Thai_sosua = 0x0dca,

    Thai_hohip = 0x0dcb,

    Thai_lochula = 0x0dcc,

    Thai_oang = 0x0dcd,

    Thai_honokhuk = 0x0dce,

    Thai_paiyannoi = 0x0dcf,

    Thai_saraa = 0x0dd0,

    Thai_maihanakat = 0x0dd1,

    Thai_saraaa = 0x0dd2,

    Thai_saraam = 0x0dd3,

    Thai_sarai = 0x0dd4,

    Thai_saraii = 0x0dd5,

    Thai_saraue = 0x0dd6,

    Thai_sarauee = 0x0dd7,

    Thai_sarau = 0x0dd8,

    Thai_sarauu = 0x0dd9,

    Thai_phinthu = 0x0dda,
    Thai_maihanakat_maitho = 0x0dde,

    Thai_baht = 0x0ddf,

    Thai_sarae = 0x0de0,

    Thai_saraae = 0x0de1,

    Thai_sarao = 0x0de2,

    Thai_saraaimaimuan = 0x0de3,

    Thai_saraaimaimalai = 0x0de4,

    Thai_lakkhangyao = 0x0de5,

    Thai_maiyamok = 0x0de6,

    Thai_maitaikhu = 0x0de7,

    Thai_maiek = 0x0de8,

    Thai_maitho = 0x0de9,

    Thai_maitri = 0x0dea,

    Thai_maichattawa = 0x0deb,

    Thai_thanthakhat = 0x0dec,

    Thai_nikhahit = 0x0ded,

    Thai_leksun = 0x0df0,

    Thai_leknung = 0x0df1,

    Thai_leksong = 0x0df2,

    Thai_leksam = 0x0df3,

    Thai_leksi = 0x0df4,

    Thai_lekha = 0x0df5,

    Thai_lekhok = 0x0df6,

    Thai_lekchet = 0x0df7,

    Thai_lekpaet = 0x0df8,

    Thai_lekkao = 0x0df9,

    Hangul = 0xff31,

    Hangul_Start = 0xff32,

    Hangul_End = 0xff33,

    Hangul_Hanja = 0xff34,

    Hangul_Jamo = 0xff35,

    Hangul_Romaja = 0xff36,

    Hangul_Codeinput = 0xff37,

    Hangul_Jeonja = 0xff38,

    Hangul_Banja = 0xff39,

    Hangul_PreHanja = 0xff3a,

    Hangul_PostHanja = 0xff3b,

    Hangul_SingleCandidate = 0xff3c,

    Hangul_MultipleCandidate = 0xff3d,

    Hangul_PreviousCandidate = 0xff3e,

    Hangul_Special = 0xff3f,

    Hangul_switch = 0xff7e,

    Hangul_Kiyeog = 0x0ea1,
    Hangul_SsangKiyeog = 0x0ea2,
    Hangul_KiyeogSios = 0x0ea3,
    Hangul_Nieun = 0x0ea4,
    Hangul_NieunJieuj = 0x0ea5,
    Hangul_NieunHieuh = 0x0ea6,
    Hangul_Dikeud = 0x0ea7,
    Hangul_SsangDikeud = 0x0ea8,
    Hangul_Rieul = 0x0ea9,
    Hangul_RieulKiyeog = 0x0eaa,
    Hangul_RieulMieum = 0x0eab,
    Hangul_RieulPieub = 0x0eac,
    Hangul_RieulSios = 0x0ead,
    Hangul_RieulTieut = 0x0eae,
    Hangul_RieulPhieuf = 0x0eaf,
    Hangul_RieulHieuh = 0x0eb0,
    Hangul_Mieum = 0x0eb1,
    Hangul_Pieub = 0x0eb2,
    Hangul_SsangPieub = 0x0eb3,
    Hangul_PieubSios = 0x0eb4,
    Hangul_Sios = 0x0eb5,
    Hangul_SsangSios = 0x0eb6,
    Hangul_Ieung = 0x0eb7,
    Hangul_Jieuj = 0x0eb8,
    Hangul_SsangJieuj = 0x0eb9,
    Hangul_Cieuc = 0x0eba,
    Hangul_Khieuq = 0x0ebb,
    Hangul_Tieut = 0x0ebc,
    Hangul_Phieuf = 0x0ebd,
    Hangul_Hieuh = 0x0ebe,

    Hangul_A = 0x0ebf,
    Hangul_AE = 0x0ec0,
    Hangul_YA = 0x0ec1,
    Hangul_YAE = 0x0ec2,
    Hangul_EO = 0x0ec3,
    Hangul_E = 0x0ec4,
    Hangul_YEO = 0x0ec5,
    Hangul_YE = 0x0ec6,
    Hangul_O = 0x0ec7,
    Hangul_WA = 0x0ec8,
    Hangul_WAE = 0x0ec9,
    Hangul_OE = 0x0eca,
    Hangul_YO = 0x0ecb,
    Hangul_U = 0x0ecc,
    Hangul_WEO = 0x0ecd,
    Hangul_WE = 0x0ece,
    Hangul_WI = 0x0ecf,
    Hangul_YU = 0x0ed0,
    Hangul_EU = 0x0ed1,
    Hangul_YI = 0x0ed2,
    Hangul_I = 0x0ed3,

    Hangul_J_Kiyeog = 0x0ed4,
    Hangul_J_SsangKiyeog = 0x0ed5,
    Hangul_J_KiyeogSios = 0x0ed6,
    Hangul_J_Nieun = 0x0ed7,
    Hangul_J_NieunJieuj = 0x0ed8,
    Hangul_J_NieunHieuh = 0x0ed9,
    Hangul_J_Dikeud = 0x0eda,
    Hangul_J_Rieul = 0x0edb,
    Hangul_J_RieulKiyeog = 0x0edc,
    Hangul_J_RieulMieum = 0x0edd,
    Hangul_J_RieulPieub = 0x0ede,
    Hangul_J_RieulSios = 0x0edf,
    Hangul_J_RieulTieut = 0x0ee0,
    Hangul_J_RieulPhieuf = 0x0ee1,
    Hangul_J_RieulHieuh = 0x0ee2,
    Hangul_J_Mieum = 0x0ee3,
    Hangul_J_Pieub = 0x0ee4,
    Hangul_J_PieubSios = 0x0ee5,
    Hangul_J_Sios = 0x0ee6,
    Hangul_J_SsangSios = 0x0ee7,
    Hangul_J_Ieung = 0x0ee8,
    Hangul_J_Jieuj = 0x0ee9,
    Hangul_J_Cieuc = 0x0eea,
    Hangul_J_Khieuq = 0x0eeb,
    Hangul_J_Tieut = 0x0eec,
    Hangul_J_Phieuf = 0x0eed,
    Hangul_J_Hieuh = 0x0eee,

    Hangul_RieulYeorinHieuh = 0x0eef,
    Hangul_SunkyeongeumMieum = 0x0ef0,
    Hangul_SunkyeongeumPieub = 0x0ef1,
    Hangul_PanSios = 0x0ef2,
    Hangul_KkogjiDalrinIeung = 0x0ef3,
    Hangul_SunkyeongeumPhieuf = 0x0ef4,
    Hangul_YeorinHieuh = 0x0ef5,

    Hangul_AraeA = 0x0ef6,
    Hangul_AraeAE = 0x0ef7,

    Hangul_J_PanSios = 0x0ef8,
    Hangul_J_KkogjiDalrinIeung = 0x0ef9,
    Hangul_J_YeorinHieuh = 0x0efa,

    Korean_Won = 0x0eff,

    Armenian_ligature_ew = 0x01000587,

    Armenian_full_stop = 0x01000589,

    Armenian_verjaket = 0x01000589,

    Armenian_separation_mark = 0x0100055d,

    Armenian_but = 0x0100055d,

    Armenian_hyphen = 0x0100058a,

    Armenian_yentamna = 0x0100058a,

    Armenian_exclam = 0x0100055c,

    Armenian_amanak = 0x0100055c,

    Armenian_accent = 0x0100055b,

    Armenian_shesht = 0x0100055b,

    Armenian_question = 0x0100055e,

    Armenian_paruyk = 0x0100055e,

    Armenian_AYB = 0x01000531,

    Armenian_ayb = 0x01000561,

    Armenian_BEN = 0x01000532,

    Armenian_ben = 0x01000562,

    Armenian_GIM = 0x01000533,

    Armenian_gim = 0x01000563,

    Armenian_DA = 0x01000534,

    Armenian_da = 0x01000564,

    Armenian_YECH = 0x01000535,

    Armenian_yech = 0x01000565,

    Armenian_ZA = 0x01000536,

    Armenian_za = 0x01000566,

    Armenian_E = 0x01000537,

    Armenian_e = 0x01000567,

    Armenian_AT = 0x01000538,

    Armenian_at = 0x01000568,

    Armenian_TO = 0x01000539,

    Armenian_to = 0x01000569,

    Armenian_ZHE = 0x0100053a,

    Armenian_zhe = 0x0100056a,

    Armenian_INI = 0x0100053b,

    Armenian_ini = 0x0100056b,

    Armenian_LYUN = 0x0100053c,

    Armenian_lyun = 0x0100056c,

    Armenian_KHE = 0x0100053d,

    Armenian_khe = 0x0100056d,

    Armenian_TSA = 0x0100053e,

    Armenian_tsa = 0x0100056e,

    Armenian_KEN = 0x0100053f,

    Armenian_ken = 0x0100056f,

    Armenian_HO = 0x01000540,

    Armenian_ho = 0x01000570,

    Armenian_DZA = 0x01000541,

    Armenian_dza = 0x01000571,

    Armenian_GHAT = 0x01000542,

    Armenian_ghat = 0x01000572,

    Armenian_TCHE = 0x01000543,

    Armenian_tche = 0x01000573,

    Armenian_MEN = 0x01000544,

    Armenian_men = 0x01000574,

    Armenian_HI = 0x01000545,

    Armenian_hi = 0x01000575,

    Armenian_NU = 0x01000546,

    Armenian_nu = 0x01000576,

    Armenian_SHA = 0x01000547,

    Armenian_sha = 0x01000577,

    Armenian_VO = 0x01000548,

    Armenian_vo = 0x01000578,

    Armenian_CHA = 0x01000549,

    Armenian_cha = 0x01000579,

    Armenian_PE = 0x0100054a,

    Armenian_pe = 0x0100057a,

    Armenian_JE = 0x0100054b,

    Armenian_je = 0x0100057b,

    Armenian_RA = 0x0100054c,

    Armenian_ra = 0x0100057c,

    Armenian_SE = 0x0100054d,

    Armenian_se = 0x0100057d,

    Armenian_VEV = 0x0100054e,

    Armenian_vev = 0x0100057e,

    Armenian_TYUN = 0x0100054f,

    Armenian_tyun = 0x0100057f,

    Armenian_RE = 0x01000550,

    Armenian_re = 0x01000580,

    Armenian_TSO = 0x01000551,

    Armenian_tso = 0x01000581,

    Armenian_VYUN = 0x01000552,

    Armenian_vyun = 0x01000582,

    Armenian_PYUR = 0x01000553,

    Armenian_pyur = 0x01000583,

    Armenian_KE = 0x01000554,

    Armenian_ke = 0x01000584,

    Armenian_O = 0x01000555,

    Armenian_o = 0x01000585,

    Armenian_FE = 0x01000556,

    Armenian_fe = 0x01000586,

    Armenian_apostrophe = 0x0100055a,

    Georgian_an = 0x010010d0,

    Georgian_ban = 0x010010d1,

    Georgian_gan = 0x010010d2,

    Georgian_don = 0x010010d3,

    Georgian_en = 0x010010d4,

    Georgian_vin = 0x010010d5,

    Georgian_zen = 0x010010d6,

    Georgian_tan = 0x010010d7,

    Georgian_in = 0x010010d8,

    Georgian_kan = 0x010010d9,

    Georgian_las = 0x010010da,

    Georgian_man = 0x010010db,

    Georgian_nar = 0x010010dc,

    Georgian_on = 0x010010dd,

    Georgian_par = 0x010010de,

    Georgian_zhar = 0x010010df,

    Georgian_rae = 0x010010e0,

    Georgian_san = 0x010010e1,

    Georgian_tar = 0x010010e2,

    Georgian_un = 0x010010e3,

    Georgian_phar = 0x010010e4,

    Georgian_khar = 0x010010e5,

    Georgian_ghan = 0x010010e6,

    Georgian_qar = 0x010010e7,

    Georgian_shin = 0x010010e8,

    Georgian_chin = 0x010010e9,

    Georgian_can = 0x010010ea,

    Georgian_jil = 0x010010eb,

    Georgian_cil = 0x010010ec,

    Georgian_char = 0x010010ed,

    Georgian_xan = 0x010010ee,

    Georgian_jhan = 0x010010ef,

    Georgian_hae = 0x010010f0,

    Georgian_he = 0x010010f1,

    Georgian_hie = 0x010010f2,

    Georgian_we = 0x010010f3,

    Georgian_har = 0x010010f4,

    Georgian_hoe = 0x010010f5,

    Georgian_fi = 0x010010f6,

    Xabovedot = 0x01001e8a,

    Ibreve = 0x0100012c,

    Zstroke = 0x010001b5,

    Gcaron = 0x010001e6,

    Ocaron = 0x010001d1,

    Obarred = 0x0100019f,

    xabovedot = 0x01001e8b,

    ibreve = 0x0100012d,

    zstroke = 0x010001b6,

    gcaron = 0x010001e7,

    ocaron = 0x010001d2,

    obarred = 0x01000275,

    SCHWA = 0x0100018f,

    schwa = 0x01000259,

    EZH = 0x010001b7,

    ezh = 0x01000292,

    Lbelowdot = 0x01001e36,

    lbelowdot = 0x01001e37,

    Abelowdot = 0x01001ea0,

    abelowdot = 0x01001ea1,

    Ahook = 0x01001ea2,

    ahook = 0x01001ea3,

    Acircumflexacute = 0x01001ea4,

    acircumflexacute = 0x01001ea5,

    Acircumflexgrave = 0x01001ea6,

    acircumflexgrave = 0x01001ea7,

    Acircumflexhook = 0x01001ea8,

    acircumflexhook = 0x01001ea9,

    Acircumflextilde = 0x01001eaa,

    acircumflextilde = 0x01001eab,

    Acircumflexbelowdot = 0x01001eac,

    acircumflexbelowdot = 0x01001ead,

    Abreveacute = 0x01001eae,

    abreveacute = 0x01001eaf,

    Abrevegrave = 0x01001eb0,

    abrevegrave = 0x01001eb1,

    Abrevehook = 0x01001eb2,

    abrevehook = 0x01001eb3,

    Abrevetilde = 0x01001eb4,

    abrevetilde = 0x01001eb5,

    Abrevebelowdot = 0x01001eb6,

    abrevebelowdot = 0x01001eb7,

    Ebelowdot = 0x01001eb8,

    ebelowdot = 0x01001eb9,

    Ehook = 0x01001eba,

    ehook = 0x01001ebb,

    Etilde = 0x01001ebc,

    etilde = 0x01001ebd,

    Ecircumflexacute = 0x01001ebe,

    ecircumflexacute = 0x01001ebf,

    Ecircumflexgrave = 0x01001ec0,

    ecircumflexgrave = 0x01001ec1,

    Ecircumflexhook = 0x01001ec2,

    ecircumflexhook = 0x01001ec3,

    Ecircumflextilde = 0x01001ec4,

    ecircumflextilde = 0x01001ec5,

    Ecircumflexbelowdot = 0x01001ec6,

    ecircumflexbelowdot = 0x01001ec7,

    Ihook = 0x01001ec8,

    ihook = 0x01001ec9,

    Ibelowdot = 0x01001eca,

    ibelowdot = 0x01001ecb,

    Obelowdot = 0x01001ecc,

    obelowdot = 0x01001ecd,

    Ohook = 0x01001ece,

    ohook = 0x01001ecf,

    Ocircumflexacute = 0x01001ed0,

    ocircumflexacute = 0x01001ed1,

    Ocircumflexgrave = 0x01001ed2,

    ocircumflexgrave = 0x01001ed3,

    Ocircumflexhook = 0x01001ed4,

    ocircumflexhook = 0x01001ed5,

    Ocircumflextilde = 0x01001ed6,

    ocircumflextilde = 0x01001ed7,

    Ocircumflexbelowdot = 0x01001ed8,

    ocircumflexbelowdot = 0x01001ed9,

    Ohornacute = 0x01001eda,

    ohornacute = 0x01001edb,

    Ohorngrave = 0x01001edc,

    ohorngrave = 0x01001edd,

    Ohornhook = 0x01001ede,

    ohornhook = 0x01001edf,

    Ohorntilde = 0x01001ee0,

    ohorntilde = 0x01001ee1,

    Ohornbelowdot = 0x01001ee2,

    ohornbelowdot = 0x01001ee3,

    Ubelowdot = 0x01001ee4,

    ubelowdot = 0x01001ee5,

    Uhook = 0x01001ee6,

    uhook = 0x01001ee7,

    Uhornacute = 0x01001ee8,

    uhornacute = 0x01001ee9,

    Uhorngrave = 0x01001eea,

    uhorngrave = 0x01001eeb,

    Uhornhook = 0x01001eec,

    uhornhook = 0x01001eed,

    Uhorntilde = 0x01001eee,

    uhorntilde = 0x01001eef,

    Uhornbelowdot = 0x01001ef0,

    uhornbelowdot = 0x01001ef1,

    Ybelowdot = 0x01001ef4,

    ybelowdot = 0x01001ef5,

    Yhook = 0x01001ef6,

    yhook = 0x01001ef7,

    Ytilde = 0x01001ef8,

    ytilde = 0x01001ef9,

    Ohorn = 0x010001a0,

    ohorn = 0x010001a1,

    Uhorn = 0x010001af,

    uhorn = 0x010001b0,

    EcuSign = 0x010020a0,

    ColonSign = 0x010020a1,

    CruzeiroSign = 0x010020a2,

    FFrancSign = 0x010020a3,

    LiraSign = 0x010020a4,

    MillSign = 0x010020a5,

    NairaSign = 0x010020a6,

    PesetaSign = 0x010020a7,

    RupeeSign = 0x010020a8,

    WonSign = 0x010020a9,

    NewSheqelSign = 0x010020aa,

    DongSign = 0x010020ab,

    EuroSign = 0x20ac,

    zerosuperior = 0x01002070,

    foursuperior = 0x01002074,

    fivesuperior = 0x01002075,

    sixsuperior = 0x01002076,

    sevensuperior = 0x01002077,

    eightsuperior = 0x01002078,

    ninesuperior = 0x01002079,

    zerosubscript = 0x01002080,

    onesubscript = 0x01002081,

    twosubscript = 0x01002082,

    threesubscript = 0x01002083,

    foursubscript = 0x01002084,

    fivesubscript = 0x01002085,

    sixsubscript = 0x01002086,

    sevensubscript = 0x01002087,

    eightsubscript = 0x01002088,

    ninesubscript = 0x01002089,

    partdifferential = 0x01002202,

    emptyset = 0x01002205,

    elementof = 0x01002208,

    notelementof = 0x01002209,

    containsas = 0x0100220B,

    squareroot = 0x0100221A,

    cuberoot = 0x0100221B,

    fourthroot = 0x0100221C,

    dintegral = 0x0100222C,

    tintegral = 0x0100222D,

    because = 0x01002235,

    approxeq = 0x01002248,

    notapproxeq = 0x01002247,

    notidentical = 0x01002262,

    stricteq = 0x01002263,

    braille_dot_1 = 0xfff1,
    braille_dot_2 = 0xfff2,
    braille_dot_3 = 0xfff3,
    braille_dot_4 = 0xfff4,
    braille_dot_5 = 0xfff5,
    braille_dot_6 = 0xfff6,
    braille_dot_7 = 0xfff7,
    braille_dot_8 = 0xfff8,
    braille_dot_9 = 0xfff9,
    braille_dot_10 = 0xfffa,

    braille_blank = 0x01002800,

    braille_dots_1 = 0x01002801,

    braille_dots_2 = 0x01002802,

    braille_dots_12 = 0x01002803,

    braille_dots_3 = 0x01002804,

    braille_dots_13 = 0x01002805,

    braille_dots_23 = 0x01002806,

    braille_dots_123 = 0x01002807,

    braille_dots_4 = 0x01002808,

    braille_dots_14 = 0x01002809,

    braille_dots_24 = 0x0100280a,

    braille_dots_124 = 0x0100280b,

    braille_dots_34 = 0x0100280c,

    braille_dots_134 = 0x0100280d,

    braille_dots_234 = 0x0100280e,

    braille_dots_1234 = 0x0100280f,

    braille_dots_5 = 0x01002810,

    braille_dots_15 = 0x01002811,

    braille_dots_25 = 0x01002812,

    braille_dots_125 = 0x01002813,

    braille_dots_35 = 0x01002814,

    braille_dots_135 = 0x01002815,

    braille_dots_235 = 0x01002816,

    braille_dots_1235 = 0x01002817,

    braille_dots_45 = 0x01002818,

    braille_dots_145 = 0x01002819,

    braille_dots_245 = 0x0100281a,

    braille_dots_1245 = 0x0100281b,

    braille_dots_345 = 0x0100281c,

    braille_dots_1345 = 0x0100281d,

    braille_dots_2345 = 0x0100281e,

    braille_dots_12345 = 0x0100281f,

    braille_dots_6 = 0x01002820,

    braille_dots_16 = 0x01002821,

    braille_dots_26 = 0x01002822,

    braille_dots_126 = 0x01002823,

    braille_dots_36 = 0x01002824,

    braille_dots_136 = 0x01002825,

    braille_dots_236 = 0x01002826,

    braille_dots_1236 = 0x01002827,

    braille_dots_46 = 0x01002828,

    braille_dots_146 = 0x01002829,

    braille_dots_246 = 0x0100282a,

    braille_dots_1246 = 0x0100282b,

    braille_dots_346 = 0x0100282c,

    braille_dots_1346 = 0x0100282d,

    braille_dots_2346 = 0x0100282e,

    braille_dots_12346 = 0x0100282f,

    braille_dots_56 = 0x01002830,

    braille_dots_156 = 0x01002831,

    braille_dots_256 = 0x01002832,

    braille_dots_1256 = 0x01002833,

    braille_dots_356 = 0x01002834,

    braille_dots_1356 = 0x01002835,

    braille_dots_2356 = 0x01002836,

    braille_dots_12356 = 0x01002837,

    braille_dots_456 = 0x01002838,

    braille_dots_1456 = 0x01002839,

    braille_dots_2456 = 0x0100283a,

    braille_dots_12456 = 0x0100283b,

    braille_dots_3456 = 0x0100283c,

    braille_dots_13456 = 0x0100283d,

    braille_dots_23456 = 0x0100283e,

    braille_dots_123456 = 0x0100283f,

    braille_dots_7 = 0x01002840,

    braille_dots_17 = 0x01002841,

    braille_dots_27 = 0x01002842,

    braille_dots_127 = 0x01002843,

    braille_dots_37 = 0x01002844,

    braille_dots_137 = 0x01002845,

    braille_dots_237 = 0x01002846,

    braille_dots_1237 = 0x01002847,

    braille_dots_47 = 0x01002848,

    braille_dots_147 = 0x01002849,

    braille_dots_247 = 0x0100284a,

    braille_dots_1247 = 0x0100284b,

    braille_dots_347 = 0x0100284c,

    braille_dots_1347 = 0x0100284d,

    braille_dots_2347 = 0x0100284e,

    braille_dots_12347 = 0x0100284f,

    braille_dots_57 = 0x01002850,

    braille_dots_157 = 0x01002851,

    braille_dots_257 = 0x01002852,

    braille_dots_1257 = 0x01002853,

    braille_dots_357 = 0x01002854,

    braille_dots_1357 = 0x01002855,

    braille_dots_2357 = 0x01002856,

    braille_dots_12357 = 0x01002857,

    braille_dots_457 = 0x01002858,

    braille_dots_1457 = 0x01002859,

    braille_dots_2457 = 0x0100285a,

    braille_dots_12457 = 0x0100285b,

    braille_dots_3457 = 0x0100285c,

    braille_dots_13457 = 0x0100285d,

    braille_dots_23457 = 0x0100285e,

    braille_dots_123457 = 0x0100285f,

    braille_dots_67 = 0x01002860,

    braille_dots_167 = 0x01002861,

    braille_dots_267 = 0x01002862,

    braille_dots_1267 = 0x01002863,

    braille_dots_367 = 0x01002864,

    braille_dots_1367 = 0x01002865,

    braille_dots_2367 = 0x01002866,

    braille_dots_12367 = 0x01002867,

    braille_dots_467 = 0x01002868,

    braille_dots_1467 = 0x01002869,

    braille_dots_2467 = 0x0100286a,

    braille_dots_12467 = 0x0100286b,

    braille_dots_3467 = 0x0100286c,

    braille_dots_13467 = 0x0100286d,

    braille_dots_23467 = 0x0100286e,

    braille_dots_123467 = 0x0100286f,

    braille_dots_567 = 0x01002870,

    braille_dots_1567 = 0x01002871,

    braille_dots_2567 = 0x01002872,

    braille_dots_12567 = 0x01002873,

    braille_dots_3567 = 0x01002874,

    braille_dots_13567 = 0x01002875,

    braille_dots_23567 = 0x01002876,

    braille_dots_123567 = 0x01002877,

    braille_dots_4567 = 0x01002878,

    braille_dots_14567 = 0x01002879,

    braille_dots_24567 = 0x0100287a,

    braille_dots_124567 = 0x0100287b,

    braille_dots_34567 = 0x0100287c,

    braille_dots_134567 = 0x0100287d,

    braille_dots_234567 = 0x0100287e,

    braille_dots_1234567 = 0x0100287f,

    braille_dots_8 = 0x01002880,

    braille_dots_18 = 0x01002881,

    braille_dots_28 = 0x01002882,

    braille_dots_128 = 0x01002883,

    braille_dots_38 = 0x01002884,

    braille_dots_138 = 0x01002885,

    braille_dots_238 = 0x01002886,

    braille_dots_1238 = 0x01002887,

    braille_dots_48 = 0x01002888,

    braille_dots_148 = 0x01002889,

    braille_dots_248 = 0x0100288a,

    braille_dots_1248 = 0x0100288b,

    braille_dots_348 = 0x0100288c,

    braille_dots_1348 = 0x0100288d,

    braille_dots_2348 = 0x0100288e,

    braille_dots_12348 = 0x0100288f,

    braille_dots_58 = 0x01002890,

    braille_dots_158 = 0x01002891,

    braille_dots_258 = 0x01002892,

    braille_dots_1258 = 0x01002893,

    braille_dots_358 = 0x01002894,

    braille_dots_1358 = 0x01002895,

    braille_dots_2358 = 0x01002896,

    braille_dots_12358 = 0x01002897,

    braille_dots_458 = 0x01002898,

    braille_dots_1458 = 0x01002899,

    braille_dots_2458 = 0x0100289a,

    braille_dots_12458 = 0x0100289b,

    braille_dots_3458 = 0x0100289c,

    braille_dots_13458 = 0x0100289d,

    braille_dots_23458 = 0x0100289e,

    braille_dots_123458 = 0x0100289f,

    braille_dots_68 = 0x010028a0,

    braille_dots_168 = 0x010028a1,

    braille_dots_268 = 0x010028a2,

    braille_dots_1268 = 0x010028a3,

    braille_dots_368 = 0x010028a4,

    braille_dots_1368 = 0x010028a5,

    braille_dots_2368 = 0x010028a6,

    braille_dots_12368 = 0x010028a7,

    braille_dots_468 = 0x010028a8,

    braille_dots_1468 = 0x010028a9,

    braille_dots_2468 = 0x010028aa,

    braille_dots_12468 = 0x010028ab,

    braille_dots_3468 = 0x010028ac,

    braille_dots_13468 = 0x010028ad,

    braille_dots_23468 = 0x010028ae,

    braille_dots_123468 = 0x010028af,

    braille_dots_568 = 0x010028b0,

    braille_dots_1568 = 0x010028b1,

    braille_dots_2568 = 0x010028b2,

    braille_dots_12568 = 0x010028b3,

    braille_dots_3568 = 0x010028b4,

    braille_dots_13568 = 0x010028b5,

    braille_dots_23568 = 0x010028b6,

    braille_dots_123568 = 0x010028b7,

    braille_dots_4568 = 0x010028b8,

    braille_dots_14568 = 0x010028b9,

    braille_dots_24568 = 0x010028ba,

    braille_dots_124568 = 0x010028bb,

    braille_dots_34568 = 0x010028bc,

    braille_dots_134568 = 0x010028bd,

    braille_dots_234568 = 0x010028be,

    braille_dots_1234568 = 0x010028bf,

    braille_dots_78 = 0x010028c0,

    braille_dots_178 = 0x010028c1,

    braille_dots_278 = 0x010028c2,

    braille_dots_1278 = 0x010028c3,

    braille_dots_378 = 0x010028c4,

    braille_dots_1378 = 0x010028c5,

    braille_dots_2378 = 0x010028c6,

    braille_dots_12378 = 0x010028c7,

    braille_dots_478 = 0x010028c8,

    braille_dots_1478 = 0x010028c9,

    braille_dots_2478 = 0x010028ca,

    braille_dots_12478 = 0x010028cb,

    braille_dots_3478 = 0x010028cc,

    braille_dots_13478 = 0x010028cd,

    braille_dots_23478 = 0x010028ce,

    braille_dots_123478 = 0x010028cf,

    braille_dots_578 = 0x010028d0,

    braille_dots_1578 = 0x010028d1,

    braille_dots_2578 = 0x010028d2,

    braille_dots_12578 = 0x010028d3,

    braille_dots_3578 = 0x010028d4,

    braille_dots_13578 = 0x010028d5,

    braille_dots_23578 = 0x010028d6,

    braille_dots_123578 = 0x010028d7,

    braille_dots_4578 = 0x010028d8,

    braille_dots_14578 = 0x010028d9,

    braille_dots_24578 = 0x010028da,

    braille_dots_124578 = 0x010028db,

    braille_dots_34578 = 0x010028dc,

    braille_dots_134578 = 0x010028dd,

    braille_dots_234578 = 0x010028de,

    braille_dots_1234578 = 0x010028df,

    braille_dots_678 = 0x010028e0,

    braille_dots_1678 = 0x010028e1,

    braille_dots_2678 = 0x010028e2,

    braille_dots_12678 = 0x010028e3,

    braille_dots_3678 = 0x010028e4,

    braille_dots_13678 = 0x010028e5,

    braille_dots_23678 = 0x010028e6,

    braille_dots_123678 = 0x010028e7,

    braille_dots_4678 = 0x010028e8,

    braille_dots_14678 = 0x010028e9,

    braille_dots_24678 = 0x010028ea,

    braille_dots_124678 = 0x010028eb,

    braille_dots_34678 = 0x010028ec,

    braille_dots_134678 = 0x010028ed,

    braille_dots_234678 = 0x010028ee,

    braille_dots_1234678 = 0x010028ef,

    braille_dots_5678 = 0x010028f0,

    braille_dots_15678 = 0x010028f1,

    braille_dots_25678 = 0x010028f2,

    braille_dots_125678 = 0x010028f3,

    braille_dots_35678 = 0x010028f4,

    braille_dots_135678 = 0x010028f5,

    braille_dots_235678 = 0x010028f6,

    braille_dots_1235678 = 0x010028f7,

    braille_dots_45678 = 0x010028f8,

    braille_dots_145678 = 0x010028f9,

    braille_dots_245678 = 0x010028fa,

    braille_dots_1245678 = 0x010028fb,

    braille_dots_345678 = 0x010028fc,

    braille_dots_1345678 = 0x010028fd,

    braille_dots_2345678 = 0x010028fe,

    braille_dots_12345678 = 0x010028ff,

    Sinh_ng = 0x01000d82,

    Sinh_h2 = 0x01000d83,

    Sinh_a = 0x01000d85,

    Sinh_aa = 0x01000d86,

    Sinh_ae = 0x01000d87,

    Sinh_aee = 0x01000d88,

    Sinh_i = 0x01000d89,

    Sinh_ii = 0x01000d8a,

    Sinh_u = 0x01000d8b,

    Sinh_uu = 0x01000d8c,

    Sinh_ri = 0x01000d8d,

    Sinh_rii = 0x01000d8e,

    Sinh_lu = 0x01000d8f,

    Sinh_luu = 0x01000d90,

    Sinh_e = 0x01000d91,

    Sinh_ee = 0x01000d92,

    Sinh_ai = 0x01000d93,

    Sinh_o = 0x01000d94,

    Sinh_oo = 0x01000d95,

    Sinh_au = 0x01000d96,

    Sinh_ka = 0x01000d9a,

    Sinh_kha = 0x01000d9b,

    Sinh_ga = 0x01000d9c,

    Sinh_gha = 0x01000d9d,

    Sinh_ng2 = 0x01000d9e,

    Sinh_nga = 0x01000d9f,

    Sinh_ca = 0x01000da0,

    Sinh_cha = 0x01000da1,

    Sinh_ja = 0x01000da2,

    Sinh_jha = 0x01000da3,

    Sinh_nya = 0x01000da4,

    Sinh_jnya = 0x01000da5,

    Sinh_nja = 0x01000da6,

    Sinh_tta = 0x01000da7,

    Sinh_ttha = 0x01000da8,

    Sinh_dda = 0x01000da9,

    Sinh_ddha = 0x01000daa,

    Sinh_nna = 0x01000dab,

    Sinh_ndda = 0x01000dac,

    Sinh_tha = 0x01000dad,

    Sinh_thha = 0x01000dae,

    Sinh_dha = 0x01000daf,

    Sinh_dhha = 0x01000db0,

    Sinh_na = 0x01000db1,

    Sinh_ndha = 0x01000db3,

    Sinh_pa = 0x01000db4,

    Sinh_pha = 0x01000db5,

    Sinh_ba = 0x01000db6,

    Sinh_bha = 0x01000db7,

    Sinh_ma = 0x01000db8,

    Sinh_mba = 0x01000db9,

    Sinh_ya = 0x01000dba,

    Sinh_ra = 0x01000dbb,

    Sinh_la = 0x01000dbd,

    Sinh_va = 0x01000dc0,

    Sinh_sha = 0x01000dc1,

    Sinh_ssha = 0x01000dc2,

    Sinh_sa = 0x01000dc3,

    Sinh_ha = 0x01000dc4,

    Sinh_lla = 0x01000dc5,

    Sinh_fa = 0x01000dc6,

    Sinh_al = 0x01000dca,

    Sinh_aa2 = 0x01000dcf,

    Sinh_ae2 = 0x01000dd0,

    Sinh_aee2 = 0x01000dd1,

    Sinh_i2 = 0x01000dd2,

    Sinh_ii2 = 0x01000dd3,

    Sinh_u2 = 0x01000dd4,

    Sinh_uu2 = 0x01000dd6,

    Sinh_ru2 = 0x01000dd8,

    Sinh_e2 = 0x01000dd9,

    Sinh_ee2 = 0x01000dda,

    Sinh_ai2 = 0x01000ddb,
    Sinh_o2 = 0x01000ddc,
    Sinh_oo2 = 0x01000ddd,

    Sinh_au2 = 0x01000dde,

    Sinh_lu2 = 0x01000ddf,

    Sinh_ruu2 = 0x01000df2,

    Sinh_luu2 = 0x01000df3,

    Sinh_kunddaliya = 0x01000df4,

    XF86ModeLock = 0x1008FF01,

    XF86MonBrightnessUp = 0x1008FF02,

    XF86MonBrightnessDown = 0x1008FF03,

    XF86KbdLightOnOff = 0x1008FF04,

    XF86KbdBrightnessUp = 0x1008FF05,

    XF86KbdBrightnessDown = 0x1008FF06,

    XF86Standby = 0x1008FF10,

    XF86AudioLowerVolume = 0x1008FF11,

    XF86AudioMute = 0x1008FF12,

    XF86AudioRaiseVolume = 0x1008FF13,

    XF86AudioPlay = 0x1008FF14,

    XF86AudioStop = 0x1008FF15,

    XF86AudioPrev = 0x1008FF16,

    XF86AudioNext = 0x1008FF17,

    XF86HomePage = 0x1008FF18,

    XF86Mail = 0x1008FF19,

    XF86Start = 0x1008FF1A,

    XF86Search = 0x1008FF1B,

    XF86AudioRecord = 0x1008FF1C,

    XF86Calculator = 0x1008FF1D,

    XF86Memo = 0x1008FF1E,

    XF86ToDoList = 0x1008FF1F,

    XF86Calendar = 0x1008FF20,

    XF86PowerDown = 0x1008FF21,

    XF86ContrastAdjust = 0x1008FF22,

    XF86RockerUp = 0x1008FF23,

    XF86RockerDown = 0x1008FF24,

    XF86RockerEnter = 0x1008FF25,

    XF86Back = 0x1008FF26,

    XF86Forward = 0x1008FF27,

    XF86Stop = 0x1008FF28,

    XF86Refresh = 0x1008FF29,

    XF86PowerOff = 0x1008FF2A,

    XF86WakeUp = 0x1008FF2B,

    XF86Eject = 0x1008FF2C,

    XF86ScreenSaver = 0x1008FF2D,

    XF86WWW = 0x1008FF2E,

    XF86Sleep = 0x1008FF2F,

    XF86Favorites = 0x1008FF30,

    XF86AudioPause = 0x1008FF31,

    XF86AudioMedia = 0x1008FF32,

    XF86MyComputer = 0x1008FF33,

    XF86VendorHome = 0x1008FF34,

    XF86LightBulb = 0x1008FF35,

    XF86Shop = 0x1008FF36,

    XF86History = 0x1008FF37,

    XF86OpenURL = 0x1008FF38,

    XF86AddFavorite = 0x1008FF39,

    XF86HotLinks = 0x1008FF3A,

    XF86BrightnessAdjust = 0x1008FF3B,

    XF86Finance = 0x1008FF3C,

    XF86Community = 0x1008FF3D,

    XF86AudioRewind = 0x1008FF3E,

    XF86BackForward = 0x1008FF3F,

    XF86Launch0 = 0x1008FF40,

    XF86Launch1 = 0x1008FF41,

    XF86Launch2 = 0x1008FF42,

    XF86Launch3 = 0x1008FF43,

    XF86Launch4 = 0x1008FF44,

    XF86Launch5 = 0x1008FF45,

    XF86Launch6 = 0x1008FF46,

    XF86Launch7 = 0x1008FF47,

    XF86Launch8 = 0x1008FF48,

    XF86Launch9 = 0x1008FF49,

    XF86LaunchA = 0x1008FF4A,

    XF86LaunchB = 0x1008FF4B,

    XF86LaunchC = 0x1008FF4C,

    XF86LaunchD = 0x1008FF4D,

    XF86LaunchE = 0x1008FF4E,

    XF86LaunchF = 0x1008FF4F,

    XF86ApplicationLeft = 0x1008FF50,
    XF86ApplicationRight = 0x1008FF51,

    XF86Book = 0x1008FF52,

    XF86CD = 0x1008FF53,

    XF86Calculater = 0x1008FF54,

    XF86Clear = 0x1008FF55,

    XF86Close = 0x1008FF56,

    XF86Copy = 0x1008FF57,

    XF86Cut = 0x1008FF58,

    XF86Display = 0x1008FF59,

    XF86DOS = 0x1008FF5A,

    XF86Documents = 0x1008FF5B,

    XF86Excel = 0x1008FF5C,

    XF86Explorer = 0x1008FF5D,

    XF86Game = 0x1008FF5E,

    XF86Go = 0x1008FF5F,

    XF86iTouch = 0x1008FF60,

    XF86LogOff = 0x1008FF61,

    XF86Market = 0x1008FF62,

    XF86Meeting = 0x1008FF63,

    XF86MenuKB = 0x1008FF65,

    XF86MenuPB = 0x1008FF66,

    XF86MySites = 0x1008FF67,

    XF86New = 0x1008FF68,

    XF86News = 0x1008FF69,
    XF86OfficeHome = 0x1008FF6A,

    XF86Open = 0x1008FF6B,

    XF86Option = 0x1008FF6C,

    XF86Paste = 0x1008FF6D,

    XF86Phone = 0x1008FF6E,

    XF86Q = 0x1008FF70,

    XF86Reply = 0x1008FF72,

    XF86Reload = 0x1008FF73,

    XF86RotateWindows = 0x1008FF74,

    XF86RotationPB = 0x1008FF75,

    XF86RotationKB = 0x1008FF76,

    XF86Save = 0x1008FF77,

    XF86ScrollUp = 0x1008FF78,

    XF86ScrollDown = 0x1008FF79,

    XF86ScrollClick = 0x1008FF7A,

    XF86Send = 0x1008FF7B,

    XF86Spell = 0x1008FF7C,

    XF86SplitScreen = 0x1008FF7D,

    XF86Support = 0x1008FF7E,

    XF86TaskPane = 0x1008FF7F,

    XF86Terminal = 0x1008FF80,

    XF86Tools = 0x1008FF81,

    XF86Travel = 0x1008FF82,

    XF86UserPB = 0x1008FF84,

    XF86User1KB = 0x1008FF85,

    XF86User2KB = 0x1008FF86,

    XF86Video = 0x1008FF87,

    XF86WheelButton = 0x1008FF88,

    XF86Word = 0x1008FF89,
    XF86Xfer = 0x1008FF8A,

    XF86ZoomIn = 0x1008FF8B,

    XF86ZoomOut = 0x1008FF8C,

    XF86Away = 0x1008FF8D,

    XF86Messenger = 0x1008FF8E,

    XF86WebCam = 0x1008FF8F,

    XF86MailForward = 0x1008FF90,

    XF86Pictures = 0x1008FF91,

    XF86Music = 0x1008FF92,

    XF86Battery = 0x1008FF93,

    XF86Bluetooth = 0x1008FF94,

    XF86WLAN = 0x1008FF95,

    XF86UWB = 0x1008FF96,

    XF86AudioForward = 0x1008FF97,

    XF86AudioRepeat = 0x1008FF98,

    XF86AudioRandomPlay = 0x1008FF99,

    XF86Subtitle = 0x1008FF9A,

    XF86AudioCycleTrack = 0x1008FF9B,

    XF86CycleAngle = 0x1008FF9C,

    XF86FrameBack = 0x1008FF9D,

    XF86FrameForward = 0x1008FF9E,

    XF86Time = 0x1008FF9F,

    XF86Select = 0x1008FFA0,

    XF86View = 0x1008FFA1,

    XF86TopMenu = 0x1008FFA2,

    XF86Red = 0x1008FFA3,

    XF86Green = 0x1008FFA4,

    XF86Yellow = 0x1008FFA5,

    XF86Blue = 0x1008FFA6,

    XF86Suspend = 0x1008FFA7,

    XF86Hibernate = 0x1008FFA8,

    XF86TouchpadToggle = 0x1008FFA9,

    XF86TouchpadOn = 0x1008FFB0,

    XF86TouchpadOff = 0x1008FFB1,

    XF86AudioMicMute = 0x1008FFB2,

    XF86Switch_VT_1 = 0x1008FE01,
    XF86Switch_VT_2 = 0x1008FE02,
    XF86Switch_VT_3 = 0x1008FE03,
    XF86Switch_VT_4 = 0x1008FE04,
    XF86Switch_VT_5 = 0x1008FE05,
    XF86Switch_VT_6 = 0x1008FE06,
    XF86Switch_VT_7 = 0x1008FE07,
    XF86Switch_VT_8 = 0x1008FE08,
    XF86Switch_VT_9 = 0x1008FE09,
    XF86Switch_VT_10 = 0x1008FE0A,
    XF86Switch_VT_11 = 0x1008FE0B,
    XF86Switch_VT_12 = 0x1008FE0C,

    XF86Ungrab = 0x1008FE20,

    XF86ClearGrab = 0x1008FE21,

    XF86Next_VMode = 0x1008FE22,

    XF86Prev_VMode = 0x1008FE23,

    XF86LogWindowTree = 0x1008FE24,

    XF86LogGrabInfo = 0x1008FE25,

    SunFA_Grave = 0x1005FF00,
    SunFA_Circum = 0x1005FF01,
    SunFA_Tilde = 0x1005FF02,
    SunFA_Acute = 0x1005FF03,
    SunFA_Diaeresis = 0x1005FF04,
    SunFA_Cedilla = 0x1005FF05,

    SunF36 = 0x1005FF10,

    SunF37 = 0x1005FF11,

    SunSys_Req = 0x1005FF60,

    SunPrint_Screen = 0x0000FF61,

    SunCompose = 0x0000FF20,

    SunAltGraph = 0x0000FF7E,

    SunPageUp = 0x0000FF55,

    SunPageDown = 0x0000FF56,

    SunUndo = 0x0000FF65,

    SunAgain = 0x0000FF66,

    SunFind = 0x0000FF68,

    SunStop = 0x0000FF69,
    SunProps = 0x1005FF70,
    SunFront = 0x1005FF71,
    SunCopy = 0x1005FF72,
    SunOpen = 0x1005FF73,
    SunPaste = 0x1005FF74,
    SunCut = 0x1005FF75,

    SunPowerSwitch = 0x1005FF76,
    SunAudioLowerVolume = 0x1005FF77,
    SunAudioMute = 0x1005FF78,
    SunAudioRaiseVolume = 0x1005FF79,
    SunVideoDegauss = 0x1005FF7A,
    SunVideoLowerBrightness = 0x1005FF7B,
    SunVideoRaiseBrightness = 0x1005FF7C,
    SunPowerSwitchShift = 0x1005FF7D,

    Dring_accent = 0x1000FEB0,
    Dcircumflex_accent = 0x1000FE5E,
    Dcedilla_accent = 0x1000FE2C,
    Dacute_accent = 0x1000FE27,
    Dgrave_accent = 0x1000FE60,
    Dtilde = 0x1000FE7E,
    Ddiaeresis = 0x1000FE22,

    DRemove = 0x1000FF00,

    hpClearLine = 0x1000FF6F,
    hpInsertLine = 0x1000FF70,
    hpDeleteLine = 0x1000FF71,
    hpInsertChar = 0x1000FF72,
    hpDeleteChar = 0x1000FF73,
    hpBackTab = 0x1000FF74,
    hpKP_BackTab = 0x1000FF75,
    hpModelock1 = 0x1000FF48,
    hpModelock2 = 0x1000FF49,
    hpReset = 0x1000FF6C,
    hpSystem = 0x1000FF6D,
    hpUser = 0x1000FF6E,
    hpmute_acute = 0x100000A8,
    hpmute_grave = 0x100000A9,
    hpmute_asciicircum = 0x100000AA,
    hpmute_diaeresis = 0x100000AB,
    hpmute_asciitilde = 0x100000AC,
    hplira = 0x100000AF,
    hpguilder = 0x100000BE,
    hpYdiaeresis = 0x100000EE,
    hpIO = 0x100000EE,
    hplongminus = 0x100000F6,
    hpblock = 0x100000FC,

    osfCopy = 0x1004FF02,
    osfCut = 0x1004FF03,
    osfPaste = 0x1004FF04,
    osfBackTab = 0x1004FF07,
    osfBackSpace = 0x1004FF08,
    osfClear = 0x1004FF0B,
    osfEscape = 0x1004FF1B,
    osfAddMode = 0x1004FF31,
    osfPrimaryPaste = 0x1004FF32,
    osfQuickPaste = 0x1004FF33,
    osfPageLeft = 0x1004FF40,
    osfPageUp = 0x1004FF41,
    osfPageDown = 0x1004FF42,
    osfPageRight = 0x1004FF43,
    osfActivate = 0x1004FF44,
    osfMenuBar = 0x1004FF45,
    osfLeft = 0x1004FF51,
    osfUp = 0x1004FF52,
    osfRight = 0x1004FF53,
    osfDown = 0x1004FF54,
    osfEndLine = 0x1004FF57,
    osfBeginLine = 0x1004FF58,
    osfEndData = 0x1004FF59,
    osfBeginData = 0x1004FF5A,
    osfPrevMenu = 0x1004FF5B,
    osfNextMenu = 0x1004FF5C,
    osfPrevField = 0x1004FF5D,
    osfNextField = 0x1004FF5E,
    osfSelect = 0x1004FF60,
    osfInsert = 0x1004FF63,
    osfUndo = 0x1004FF65,
    osfMenu = 0x1004FF67,
    osfCancel = 0x1004FF69,
    osfHelp = 0x1004FF6A,
    osfSelectAll = 0x1004FF71,
    osfDeselectAll = 0x1004FF72,
    osfReselect = 0x1004FF73,
    osfExtend = 0x1004FF74,
    osfRestore = 0x1004FF78,
    osfDelete = 0x1004FFFF,

    Reset = 0x1000FF6C,
    System = 0x1000FF6D,
    User = 0x1000FF6E,
    ClearLine = 0x1000FF6F,
    InsertLine = 0x1000FF70,
    DeleteLine = 0x1000FF71,
    InsertChar = 0x1000FF72,
    DeleteChar = 0x1000FF73,
    BackTab = 0x1000FF74,
    KP_BackTab = 0x1000FF75,
    Ext16bit_L = 0x1000FF76,
    Ext16bit_R = 0x1000FF77,
    mute_acute = 0x100000a8,
    mute_grave = 0x100000a9,
    mute_asciicircum = 0x100000aa,
    mute_diaeresis = 0x100000ab,
    mute_asciitilde = 0x100000ac,
    lira = 0x100000af,
    guilder = 0x100000be,
    IO = 0x100000ee,
    longminus = 0x100000f6,
    block = 0x100000fc,
}

return keys
