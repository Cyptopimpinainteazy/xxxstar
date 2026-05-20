use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {

        debug_mode: { any(feature = "debug_order") },

    }
}
