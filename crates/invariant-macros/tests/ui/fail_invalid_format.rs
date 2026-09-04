use invariant_macros::invariant;

#[invariant("BADFORMAT")]
fn test_invalid() {
    // invalid format should fail to compile
}

fn main() {}
