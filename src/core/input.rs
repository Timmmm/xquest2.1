//! Emulation of the keyboard interrupt handler in `keyboard.pas` and of the mouse driver.

/// PC scan codes (set 1). Keys sent with an `$E0` prefix have 128 added.
pub mod scan {
    pub const ESC: u8 = 0x01;
    pub const MINUS: u8 = 0x0C;
    pub const EQUAL: u8 = 0x0D;
    pub const BACKSPACE: u8 = 0x0E;
    pub const ENTER: u8 = 0x1C;
    pub const P: u8 = 0x19;
    pub const Y: u8 = 0x15;
    pub const S: u8 = 0x1F;
    pub const J: u8 = 0x24;
    pub const N: u8 = 0x31;
    pub const LEFT_SHIFT: u8 = 0x2A;
    pub const RIGHT_SHIFT: u8 = 0x36;
    pub const SPACE: u8 = 0x39;
    pub const KEYPAD7: u8 = 0x47;
    pub const KEYPAD8: u8 = 0x48;
    pub const KEYPAD9: u8 = 0x49;
    pub const KEYPAD_MINUS: u8 = 0x4A;
    pub const KEYPAD4: u8 = 0x4B;
    pub const KEYPAD5: u8 = 0x4C;
    pub const KEYPAD6: u8 = 0x4D;
    pub const KEYPAD_PLUS: u8 = 0x4E;
    pub const KEYPAD1: u8 = 0x4F;
    pub const KEYPAD2: u8 = 0x50;
    pub const KEYPAD3: u8 = 0x51;
    pub const KEYPAD_DEL: u8 = 0x53;
    pub const UARROW: u8 = 0xC8;
    pub const LARROW: u8 = 0xCB;
    pub const RARROW: u8 = 0xCD;
    pub const DARROW: u8 = 0xD0;
    pub const DEL: u8 = 0xD3;
}

/// `toascii` from `keyboard.pas`: unshifted (0..127) and shifted (128..255) characters.
const TO_ASCII: [u8; 256] = {
    let mut t = [0u8; 256];
    let lower: &[u8; 64] = b"\0\x1b1234567890-=\x0e\x0fqwertyuiop\0\0\r\0asdfghjkl;\0'\0\0zxcvbnm,./\0\0\0 \0\x01\x02\x03\x04\x05";
    let upper: &[u8; 64] = b"\0\x1b!@#$%^&*()_+\x0e\x0fQWERTYUIOP\0\0\r\0ASDFGHJKL:\0\"\0\0ZXCVBNM<>?\0\0\0 \0\x01\x02\x03\x04\x05";
    let rest: [u8; 32] = [6, 7, 8, 9, 10, 0, 0, 24, 25, 26, b'-', 21, 22, 23, 0, 18, 19, 20, 16, 17, 0, 0, 0, 11, 12, 0, 0, 0, 0, 0, 0, 0];

    let mut i = 0;
    while i < 64 {
        t[i] = lower[i];
        t[128 + i] = upper[i];
        i += 1;
    }
    let mut i = 0;
    while i < 32 {
        t[64 + i] = rest[i];
        t[192 + i] = rest[i];
        i += 1;
    }
    t
};

pub struct Keyboard {
    pub keys: [bool; 256],
    pub key_press: [u8; 256],
    pub any_key_pressed: bool,
    pub last_key_hit: u8,
    pub last_scan_code: u8,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self { keys: [false; 256], key_press: [0; 256], any_key_pressed: false, last_key_hit: 0, last_scan_code: 0 }
    }
}

impl Keyboard {
    /// The body of the `NewInt9` interrupt handler.
    pub fn event(&mut self, scan: u8, pressed: bool) {
        let bx = scan as usize;

        if pressed && !self.keys[bx] {
            self.key_press[bx] = self.key_press[bx].wrapping_add(1);
        }
        self.keys[bx] = pressed;
        if pressed {
            self.any_key_pressed = true;
        }
        self.last_scan_code = scan;

        let shift = self.keys[scan::LEFT_SHIFT as usize] || self.keys[scan::RIGHT_SHIFT as usize];
        let index = scan.wrapping_add(if shift { 128 } else { 0 });
        self.last_key_hit = TO_ASCII[index as usize];
    }

    pub fn numkeypresses(&mut self, key: u8) -> u8 {
        std::mem::take(&mut self.key_press[key as usize])
    }

    pub fn keypressed(&mut self) -> bool {
        std::mem::take(&mut self.any_key_pressed)
    }

    pub fn clear_key_presses(&mut self) {
        self.key_press = [0; 256];
        self.any_key_pressed = false;
    }
}

/// The parts of the INT 33h mouse driver that the game uses.
#[derive(Default)]
pub struct Mouse {
    motion_x: i32,
    motion_y: i32,
    buttons: u8,
    presses: [u32; 2],
}

impl Mouse {
    pub fn motion(&mut self, dx: i32, dy: i32) {
        self.motion_x += dx;
        self.motion_y += dy;
    }

    pub fn button_event(&mut self, button: u8, pressed: bool) {
        let bit = 1 << button;
        if pressed {
            if self.buttons & bit == 0 {
                self.presses[button as usize] += 1;
            }
            self.buttons |= bit;
        } else {
            self.buttons &= !bit;
        }
    }

    /// `mmotion`: motion counters since the last call.
    pub fn mmotion(&mut self) -> (i32, i32) {
        let m = (self.motion_x, self.motion_y);
        self.motion_x = 0;
        self.motion_y = 0;
        m
    }

    /// `mbutpress`: number of presses of a button since the last call.
    pub fn mbutpress(&mut self, button: usize) -> u32 {
        std::mem::take(&mut self.presses[button])
    }

    /// `mpos`: the buttons currently held down.
    pub fn buttons(&self) -> i32 {
        self.buttons as i32
    }

    /// `mstatus`: resets the driver.
    pub fn reset(&mut self) {
        self.motion_x = 0;
        self.motion_y = 0;
        self.presses = [0; 2];
    }
}
