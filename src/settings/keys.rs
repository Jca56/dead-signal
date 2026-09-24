//! What each key does in a run: an [`Action`] a key (or a mouse button)
//! each, one key an action, the player's to change. Binding a key that's
//! already another action's swaps the two, so nothing is left unbound.
//! Esc isn't among them: it's always the way out (pause, close, cancel).

use lntrn_ui::{Key, Ui};

/// What can be done with a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Sprint,
    Jump,
    Crouch,
    Fire,
    Aim,
    Reload,
    Bash,
    FireMode,
    Primary,
    Sidearm,
    Melee,
    Bandage,
    Medkit,
    Plate,
    Throw,
    NextThrowable,
    Interact,
    Inventory,
    Map,
}

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// What's pressed for an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bind {
    Key(Key),
    Mouse(Button),
}

impl Action {
    pub const ALL: [Action; 23] = [
        Action::Forward,
        Action::Back,
        Action::Left,
        Action::Right,
        Action::Sprint,
        Action::Jump,
        Action::Crouch,
        Action::Fire,
        Action::Aim,
        Action::Reload,
        Action::Bash,
        Action::FireMode,
        Action::Primary,
        Action::Sidearm,
        Action::Melee,
        Action::Bandage,
        Action::Medkit,
        Action::Plate,
        Action::Throw,
        Action::NextThrowable,
        Action::Interact,
        Action::Inventory,
        Action::Map,
    ];

    /// Its name in the file.
    pub fn key(self) -> &'static str {
        match self {
            Action::Forward => "forward",
            Action::Back => "back",
            Action::Left => "left",
            Action::Right => "right",
            Action::Sprint => "sprint",
            Action::Jump => "jump",
            Action::Crouch => "crouch",
            Action::Fire => "fire",
            Action::Aim => "aim",
            Action::Reload => "reload",
            Action::Bash => "bash",
            Action::FireMode => "fire_mode",
            Action::Primary => "primary",
            Action::Sidearm => "sidearm",
            Action::Melee => "melee",
            Action::Bandage => "bandage",
            Action::Medkit => "medkit",
            Action::Plate => "armor_plate",
            Action::Throw => "throw",
            Action::NextThrowable => "next_throwable",
            Action::Interact => "interact",
            Action::Inventory => "inventory",
            Action::Map => "map",
        }
    }

    /// Its name on screen.
    pub fn label(self) -> &'static str {
        match self {
            Action::Forward => "FORWARD",
            Action::Back => "BACK",
            Action::Left => "LEFT",
            Action::Right => "RIGHT",
            Action::Sprint => "SPRINT",
            Action::Jump => "JUMP",
            Action::Crouch => "CROUCH",
            Action::Fire => "FIRE / SWING",
            Action::Aim => "AIM DOWN SIGHTS",
            Action::Reload => "RELOAD",
            Action::Bash => "QUICK BASH",
            Action::FireMode => "FIRE MODE",
            Action::Primary => "PRIMARY WEAPON",
            Action::Sidearm => "SIDEARM",
            Action::Melee => "MELEE WEAPON",
            Action::Bandage => "USE BANDAGE",
            Action::Medkit => "USE MEDKIT",
            Action::Plate => "USE ARMOR PLATE",
            Action::Throw => "THROW (HOLD TO AIM)",
            Action::NextThrowable => "NEXT THROWABLE",
            Action::Interact => "INTERACT / SEARCH",
            Action::Inventory => "INVENTORY",
            Action::Map => "MAP",
        }
    }

    /// What it's bound to to begin with.
    pub fn default_bind(self) -> Bind {
        let c = |ch| Bind::Key(Key::Char(ch));
        match self {
            Action::Forward => c('w'),
            Action::Back => c('s'),
            Action::Left => c('a'),
            Action::Right => c('d'),
            Action::Sprint => Bind::Key(Key::Shift),
            Action::Jump => Bind::Key(Key::Space),
            Action::Crouch => c('c'),
            Action::Fire => Bind::Mouse(Button::Left),
            Action::Aim => Bind::Mouse(Button::Right),
            Action::Reload => c('r'),
            Action::Bash => c('v'),
            Action::FireMode => c('b'),
            Action::Primary => c('1'),
            Action::Sidearm => c('2'),
            Action::Melee => c('3'),
            Action::Bandage => c('4'),
            Action::Medkit => c('5'),
            Action::Plate => c('6'),
            Action::Throw => c('g'),
            Action::NextThrowable => c('t'),
            Action::Interact => c('e'),
            Action::Inventory => Bind::Key(Key::Tab),
            Action::Map => c('m'),
        }
    }
}

impl Action {
    /// The key that takes `slot`'s weapon in hand.
    pub fn slot(slot: crate::loot::bag::Slot) -> Action {
        use crate::loot::bag::Slot;
        match slot {
            Slot::Primary => Action::Primary,
            Slot::Sidearm => Action::Sidearm,
            Slot::Melee => Action::Melee,
        }
    }
}

impl Bind {
    /// The same, as it's compared: a letter in lower case, the space bar
    /// one key however it arrives.
    fn plain(self) -> Bind {
        match self {
            Bind::Key(Key::Char(' ')) => Bind::Key(Key::Space),
            Bind::Key(Key::Char(c)) => Bind::Key(Key::Char(c.to_ascii_lowercase())),
            b => b,
        }
    }

    /// How it's written, on screen and in the file: `W`, `Space`, `Mouse Left`.
    pub fn label(self) -> String {
        match self.plain() {
            Bind::Mouse(Button::Left) => "Mouse Left".into(),
            Bind::Mouse(Button::Right) => "Mouse Right".into(),
            Bind::Mouse(Button::Middle) => "Mouse Middle".into(),
            Bind::Key(k) => k.label(),
        }
    }

    /// Back from how it's written (none if it's not a key the game knows).
    pub fn parse(text: &str) -> Option<Bind> {
        let t = text.trim();
        let named = [
            ("Mouse Left", Bind::Mouse(Button::Left)),
            ("Mouse Right", Bind::Mouse(Button::Right)),
            ("Mouse Middle", Bind::Mouse(Button::Middle)),
            ("Space", Bind::Key(Key::Space)),
            ("Tab", Bind::Key(Key::Tab)),
            ("Enter", Bind::Key(Key::Enter)),
            ("Backspace", Bind::Key(Key::Backspace)),
            ("Delete", Bind::Key(Key::Delete)),
            ("Insert", Bind::Key(Key::Insert)),
            ("Left", Bind::Key(Key::ArrowLeft)),
            ("Right", Bind::Key(Key::ArrowRight)),
            ("Up", Bind::Key(Key::ArrowUp)),
            ("Down", Bind::Key(Key::ArrowDown)),
            ("Home", Bind::Key(Key::Home)),
            ("End", Bind::Key(Key::End)),
            ("Page Up", Bind::Key(Key::PageUp)),
            ("Page Down", Bind::Key(Key::PageDown)),
            ("Shift", Bind::Key(Key::Shift)),
            ("Ctrl", Bind::Key(Key::Control)),
            ("Control", Bind::Key(Key::Control)),
            ("Alt", Bind::Key(Key::Alt)),
            ("Caps Lock", Bind::Key(Key::CapsLock)),
        ];
        if let Some(&(_, b)) = named.iter().find(|(n, _)| n.eq_ignore_ascii_case(t)) {
            return Some(b);
        }
        if let Some(n) = t.strip_prefix(['F', 'f']).and_then(|n| n.parse::<u8>().ok()).filter(|n| (1..=24).contains(n)) {
            return Some(Bind::Key(Key::F(n)));
        }
        let mut chars = t.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_whitespace() => Some(Bind::Key(Key::Char(c)).plain()),
            _ => None,
        }
    }

    /// Whether it can be bound at all (Esc, and keys the game can't tell
    /// apart, can't).
    pub fn bindable(self) -> bool {
        !matches!(self.plain(), Bind::Key(Key::Escape | Key::Unknown | Key::Super))
    }
}

/// Every action's key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keys([Bind; Action::ALL.len()]);

impl Default for Keys {
    fn default() -> Self {
        Self(Action::ALL.map(Action::default_bind))
    }
}

fn index(a: Action) -> usize {
    Action::ALL.iter().position(|&x| x == a).unwrap_or(0)
}

impl Keys {
    pub fn get(&self, a: Action) -> Bind {
        self.0[index(a)]
    }

    /// `a`'s key as the HUD and the bag write it: "E", "SPACE", "MOUSE MIDDLE".
    pub fn name(&self, a: Action) -> String {
        self.get(a).label().to_uppercase()
    }

    /// The keys of the weapon slots, in slot order.
    pub fn slot_names(&self) -> [String; 3] {
        crate::loot::bag::Slot::ALL.map(|s| self.name(Action::slot(s)))
    }

    /// Bind `a` to `bind`; the action that had it (if another) takes `a`'s
    /// old key in trade. Which action that was.
    pub fn bind(&mut self, a: Action, bind: Bind) -> Option<Action> {
        let bind = bind.plain();
        let old = self.get(a);
        let other = Action::ALL.into_iter().find(|&o| o != a && self.get(o) == bind);
        if let Some(o) = other {
            self.0[index(o)] = old;
        }
        self.0[index(a)] = bind;
        other
    }

    /// Whether `a`'s key is held down.
    pub fn held(&self, ui: &Ui, a: Action) -> bool {
        match self.get(a) {
            Bind::Mouse(Button::Left) => ui.state.down,
            Bind::Mouse(Button::Right) => ui.state.right_down,
            Bind::Mouse(Button::Middle) => ui.state.middle_down,
            b => ui.state.keys_down.iter().any(|&k| Bind::Key(k).plain() == b),
        }
    }

    /// Whether `a`'s key went down this frame (a key press is taken, so
    /// nothing else acts on it too).
    pub fn pressed(&self, ui: &mut Ui, a: Action) -> bool {
        match self.get(a) {
            Bind::Mouse(Button::Left) => ui.state.pressed,
            Bind::Mouse(Button::Right) => ui.state.right_pressed,
            Bind::Mouse(Button::Middle) => ui.state.middle_pressed,
            b => ui.state.take_key(|k| !k.repeat && Bind::Key(k.key).plain() == b).is_some(),
        }
    }

    /// Written into a settings file's `keys` table.
    pub fn to_doc(self) -> lntrn_data::Doc {
        let mut d = lntrn_data::Doc::map();
        for a in Action::ALL {
            d.set(a.key(), self.get(a).label().into());
        }
        d
    }

    /// From a settings file's `keys` table: what's missing or strange is
    /// its default; a key given twice goes to the first that has it.
    pub fn from_doc(d: Option<&lntrn_data::Doc>) -> Keys {
        let mut keys = Keys::default();
        let Some(d) = d else { return keys };
        for a in Action::ALL {
            if let Some(b) = d.get(a.key()).and_then(lntrn_data::Doc::as_str).and_then(Bind::parse).filter(|b| b.bindable()) {
                keys.bind(a, b);
            }
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_starts_on_a_key_of_its_own() {
        let k = Keys::default();
        for a in Action::ALL {
            for b in Action::ALL {
                assert!(a == b || k.get(a) != k.get(b), "{a:?} and {b:?} share a key");
            }
        }
    }

    #[test]
    fn binding_a_taken_key_trades_them() {
        let mut k = Keys::default();
        assert_eq!(k.bind(Action::Reload, Bind::Key(Key::Char('E'))), Some(Action::Interact));
        assert_eq!(k.get(Action::Reload), Bind::Key(Key::Char('e')));
        assert_eq!(k.get(Action::Interact), Bind::Key(Key::Char('r')));
        assert_eq!(k.bind(Action::Jump, Bind::Key(Key::Char(' '))), None, "the space bar is the space bar");
        assert_eq!(k.bind(Action::Bash, Bind::Mouse(Button::Middle)), None);
    }

    #[test]
    fn every_key_is_written_the_way_it_reads_back() {
        let mut binds: Vec<Bind> = Action::ALL.map(Action::default_bind).to_vec();
        binds.extend([Bind::Key(Key::F(5)), Bind::Key(Key::Control), Bind::Key(Key::ArrowUp), Bind::Key(Key::PageDown), Bind::Mouse(Button::Middle), Bind::Key(Key::Char('.'))]);
        for b in binds {
            assert_eq!(Bind::parse(&b.label()), Some(b.plain()), "{}", b.label());
        }
        assert_eq!(Bind::parse("nonsense"), None);
        assert!(!Bind::Key(Key::Escape).bindable());
    }

    #[test]
    fn keys_come_back_from_the_file_and_nonsense_is_let_go() {
        let mut k = Keys::default();
        k.bind(Action::Crouch, Bind::Key(Key::Control));
        k.bind(Action::Bash, Bind::Mouse(Button::Middle));
        assert_eq!(Keys::from_doc(Some(&k.to_doc())), k);
        let d = lntrn_data::toml::parse("forward = \"Up\"\nback = \"What\"\njump = \"Esc\"").unwrap();
        let k = Keys::from_doc(Some(&d));
        assert_eq!(k.get(Action::Forward), Bind::Key(Key::ArrowUp));
        assert_eq!(k.get(Action::Back), Action::Back.default_bind());
        assert_eq!(k.get(Action::Jump), Action::Jump.default_bind());
    }
}
