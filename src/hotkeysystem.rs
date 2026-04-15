struct ModifierState {
    alt_pressed: bool,
}

impl ModifierState {
    fn new() -> Self {
        Self {
            alt_pressed: false,
        }
    }
}

