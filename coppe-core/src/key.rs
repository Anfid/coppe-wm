use core::ops::{BitOr, BitOrAssign};

use crate::event::SubscriptionEvent;

pub struct Key {
    pub modmask: ModMask,
    pub keycode: Keycode,
}

impl Key {
    pub fn new(modmask: ModMask, keycode: Keycode) -> Self {
        Self { modmask, keycode }
    }

    pub fn press_subscription(self) -> SubscriptionEvent {
        SubscriptionEvent::KeyPress(self)
    }

    pub fn release_subscription(self) -> SubscriptionEvent {
        SubscriptionEvent::KeyRelease(self)
    }
}

impl From<(ModMask, Keycode)> for Key {
    fn from((modmask, keycode): (ModMask, Keycode)) -> Self {
        Self { modmask, keycode }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ModMask(u16);

impl ModMask {
    pub const SHIFT: Self = Self(1 << 0);
    pub const LOCK: Self = Self(1 << 1);
    pub const CONTROL: Self = Self(1 << 2);
    pub const M1: Self = Self(1 << 3);
    pub const M2: Self = Self(1 << 4);
    pub const M3: Self = Self(1 << 5);
    pub const M4: Self = Self(1 << 6);
    pub const M5: Self = Self(1 << 7);
    pub const ANY: Self = Self(1 << 15);
}

impl From<ModMask> for u16 {
    #[inline]
    fn from(input: ModMask) -> Self {
        input.0
    }
}

impl From<ModMask> for i32 {
    #[inline]
    fn from(input: ModMask) -> Self {
        i32::from(input.0)
    }
}

impl From<i32> for ModMask {
    #[inline]
    fn from(input: i32) -> Self {
        Self(input as u16)
    }
}

impl BitOr for ModMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ModMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Keycode(u8);

#[allow(non_upper_case_globals)]
impl Keycode {
    pub const Escape: Self = Self(9);
    pub const One: Self = Self(10);
    pub const Two: Self = Self(11);
    pub const Three: Self = Self(12);
    pub const Four: Self = Self(13);
    pub const Five: Self = Self(14);
    pub const Six: Self = Self(15);
    pub const Seven: Self = Self(16);
    pub const Eight: Self = Self(17);
    pub const Nine: Self = Self(18);
    pub const Zero: Self = Self(19);
    pub const Minus: Self = Self(20);
    pub const Equal: Self = Self(21);
    pub const BackSpace: Self = Self(22);
    pub const Tab: Self = Self(23);
    pub const Q: Self = Self(24);
    pub const W: Self = Self(25);
    pub const E: Self = Self(26);
    pub const R: Self = Self(27);
    pub const T: Self = Self(28);
    pub const Y: Self = Self(29);
    pub const U: Self = Self(30);
    pub const I: Self = Self(31);
    pub const O: Self = Self(32);
    pub const P: Self = Self(33);
    pub const BracketLeft: Self = Self(34);
    pub const BracketRight: Self = Self(35);
    pub const Return: Self = Self(36);
    pub const ControlL: Self = Self(37);
    pub const A: Self = Self(38);
    pub const S: Self = Self(39);
    pub const D: Self = Self(40);
    pub const F: Self = Self(41);
    pub const G: Self = Self(42);
    pub const H: Self = Self(43);
    pub const J: Self = Self(44);
    pub const K: Self = Self(45);
    pub const L: Self = Self(46);
    pub const Semicolon: Self = Self(47);
    pub const Apostrophe: Self = Self(48);
    pub const Grave: Self = Self(49);
    pub const ShiftL: Self = Self(50);
    pub const Backslash: Self = Self(51);
    pub const Z: Self = Self(52);
    pub const X: Self = Self(53);
    pub const C: Self = Self(54);
    pub const V: Self = Self(55);
    pub const B: Self = Self(56);
    pub const N: Self = Self(57);
    pub const M: Self = Self(58);
    pub const Comma: Self = Self(59);
    pub const Period: Self = Self(60);
    pub const Slash: Self = Self(61);
    pub const ShiftR: Self = Self(62);
    pub const KpMultiply: Self = Self(63);
    pub const AltL: Self = Self(64);
    pub const Space: Self = Self(65);
    pub const CapsLock: Self = Self(66);
    pub const F1: Self = Self(67);
    pub const F2: Self = Self(68);
    pub const F3: Self = Self(69);
    pub const F4: Self = Self(70);
    pub const F5: Self = Self(71);
    pub const F6: Self = Self(72);
    pub const F7: Self = Self(73);
    pub const F8: Self = Self(74);
    pub const F9: Self = Self(75);
    pub const F10: Self = Self(76);
    pub const NumLock: Self = Self(77);
    pub const ScrollLock: Self = Self(78);
    pub const KpHome: Self = Self(79);
    pub const KpUp: Self = Self(80);
    pub const KpPrior: Self = Self(81);
    pub const KpSubtract: Self = Self(82);
    pub const KpLeft: Self = Self(83);
    pub const KpBegin: Self = Self(84);
    pub const KpRight: Self = Self(85);
    pub const KpAdd: Self = Self(86);
    pub const KpEnd: Self = Self(87);
    pub const KpDown: Self = Self(88);
    pub const KpNext: Self = Self(89);
    pub const KpInsert: Self = Self(90);
    pub const KpDelete: Self = Self(91);
    pub const IsoLevel3Shift: Self = Self(92);
    pub const Less: Self = Self(94);
    pub const F11: Self = Self(95);
    pub const F12: Self = Self(96);
    pub const Katakana: Self = Self(98);
    pub const Hiragana: Self = Self(99);
    pub const HenkanMode: Self = Self(100);
    pub const HiraganaKatakana: Self = Self(101);
    pub const Muhenkan: Self = Self(102);
    pub const KpEnter: Self = Self(104);
    pub const ControlR: Self = Self(105);
    pub const KpDivide: Self = Self(106);
    pub const Print: Self = Self(107);
    pub const IsoNextGroup: Self = Self(108);
    pub const Linefeed: Self = Self(109);
    pub const Home: Self = Self(110);
    pub const Up: Self = Self(111);
    pub const Prior: Self = Self(112);
    pub const Left: Self = Self(113);
    pub const Right: Self = Self(114);
    pub const End: Self = Self(115);
    pub const Down: Self = Self(116);
    pub const Next: Self = Self(117);
    pub const Insert: Self = Self(118);
    pub const Delete: Self = Self(119);
    pub const XF86AudioMute: Self = Self(121);
    pub const XF86AudioLowerVolume: Self = Self(122);
    pub const XF86AudioRaiseVolume: Self = Self(123);
    pub const XF86PowerOff: Self = Self(124);
    pub const KpEqual: Self = Self(125);
    pub const PlusMinus: Self = Self(126);
    pub const Pause: Self = Self(127);
    pub const XF86LaunchA: Self = Self(128);
    pub const KpDecimal: Self = Self(129);
    pub const Hangul: Self = Self(130);
    pub const HangulHanja: Self = Self(131);
    pub const SuperL: Self = Self(133);
    pub const SuperR: Self = Self(134);
    pub const Menu: Self = Self(135);
    pub const Cancel: Self = Self(136);
    pub const Redo: Self = Self(137);
    pub const SunProps: Self = Self(138);
    pub const Undo: Self = Self(139);
    pub const SunFront: Self = Self(140);
    pub const XF86Copy: Self = Self(141);
    pub const XF86Open: Self = Self(142);
    pub const XF86Paste: Self = Self(143);
    pub const Find: Self = Self(144);
    pub const XF86Cut: Self = Self(145);
    pub const Help: Self = Self(146);
    pub const XF86MenuKB: Self = Self(147);
    pub const XF86Calculator: Self = Self(148);
    pub const XF86Sleep: Self = Self(150);
    pub const XF86WakeUp: Self = Self(151);
    pub const XF86Explorer: Self = Self(152);
    pub const XF86Send: Self = Self(153);
    pub const XF86Xfer: Self = Self(155);
    pub const XF86Launch1: Self = Self(156);
    pub const XF86Launch2: Self = Self(157);
    pub const XF86WWW: Self = Self(158);
    pub const XF86DOS: Self = Self(159);
    pub const XF86ScreenSaver: Self = Self(160);
    pub const XF86RotateWindows: Self = Self(161);
    pub const XF86TaskPane: Self = Self(162);
    pub const XF86Mail: Self = Self(163);
    pub const XF86Favorites: Self = Self(164);
    pub const XF86MyComputer: Self = Self(165);
    pub const XF86Back: Self = Self(166);
    pub const XF86Forward: Self = Self(167);
    pub const XF86Eject: Self = Self(169);
    pub const XF86AudioNext: Self = Self(171);
    pub const XF86AudioPlay: Self = Self(172);
    pub const XF86AudioPrev: Self = Self(173);
    pub const XF86AudioStop: Self = Self(174);
    pub const XF86AudioRecord: Self = Self(175);
    pub const XF86AudioRewind: Self = Self(176);
    pub const XF86Phone: Self = Self(177);
    pub const XF86Tools: Self = Self(179);
    pub const XF86HomePage: Self = Self(180);
    pub const XF86Reload: Self = Self(181);
    pub const XF86Close: Self = Self(182);
    pub const XF86ScrollUp: Self = Self(185);
    pub const XF86ScrollDown: Self = Self(186);
    pub const ParenLeft: Self = Self(187);
    pub const ParenRight: Self = Self(188);
    pub const XF86New: Self = Self(189);
    pub const XF86Launch5: Self = Self(192);
    pub const XF86Launch6: Self = Self(193);
    pub const XF86Launch7: Self = Self(194);
    pub const XF86Launch8: Self = Self(195);
    pub const XF86Launch9: Self = Self(196);
    pub const XF86AudioMicMute: Self = Self(198);
    pub const XF86TouchpadToggle: Self = Self(199);
    pub const XF86TouchpadOn: Self = Self(200);
    pub const XF86TouchpadOff: Self = Self(201);
    pub const ModeSwitch: Self = Self(203);
    pub const XF86AudioPause: Self = Self(209);
    pub const XF86Launch3: Self = Self(210);
    pub const XF86Launch4: Self = Self(211);
    pub const XF86LaunchB: Self = Self(212);
    pub const XF86Suspend: Self = Self(213);
    pub const XF86AudioForward: Self = Self(216);
    pub const XF86WebCam: Self = Self(220);
    pub const XF86AudioPreset: Self = Self(221);
    pub const XF86Messenger: Self = Self(224);
    pub const XF86Search: Self = Self(225);
    pub const XF86Go: Self = Self(226);
    pub const XF86Finance: Self = Self(227);
    pub const XF86Game: Self = Self(228);
    pub const XF86Shop: Self = Self(229);
    pub const XF86MonBrightnessDown: Self = Self(232);
    pub const XF86MonBrightnessUp: Self = Self(233);
    pub const XF86AudioMedia: Self = Self(234);
    pub const XF86Display: Self = Self(235);
    pub const XF86KbdLightOnOff: Self = Self(236);
    pub const XF86KbdBrightnessDown: Self = Self(237);
    pub const XF86KbdBrightnessUp: Self = Self(238);
    pub const XF86Reply: Self = Self(240);
    pub const XF86MailForward: Self = Self(241);
    pub const XF86Save: Self = Self(242);
    pub const XF86Documents: Self = Self(243);
    pub const XF86Battery: Self = Self(244);
    pub const XF86Bluetooth: Self = Self(245);
    pub const XF86WLAN: Self = Self(246);
    pub const XF86UWB: Self = Self(247);
    pub const XF86NextVMode: Self = Self(249);
    pub const XF86PrevVMode: Self = Self(250);
    pub const XF86MonBrightnessCycle: Self = Self(251);
    pub const XF86WWAN: Self = Self(254);
    pub const XF86RFKill: Self = Self(255);
}

impl From<u8> for Keycode {
    fn from(code: u8) -> Self {
        Self(code)
    }
}

impl From<Keycode> for u8 {
    fn from(keycode: Keycode) -> u8 {
        keycode.0
    }
}

impl From<i32> for Keycode {
    fn from(code: i32) -> Self {
        Self(code as u8)
    }
}

impl From<Keycode> for i32 {
    fn from(keycode: Keycode) -> i32 {
        keycode.0 as i32
    }
}
