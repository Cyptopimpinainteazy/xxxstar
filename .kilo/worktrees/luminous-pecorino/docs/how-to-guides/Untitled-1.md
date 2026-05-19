cargo update -p home@0.5.12 --precise 0.5.9 2>&1
SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | tail -100
The environmental from git isn't being used for sp-externalities. The issue is sp-externalities comes from polkadot-sdk (not our local workspace), and it's using its own environmental@1.1.3.

Let me check what version polkadot-sdk uses:

grep -A1 "environmental" ~/.cargo/git/checkouts/polkadot-sdk-*/*/substrate/primitives/externalities/Cargo.toml 2>/dev/null | head -10
The polkadot-sdk that Frontier pulls in uses environmental 1.1.3. Our patch should override this. Let me check if the patch is actually being applied:

grep -A2 'name = "environmental"' Cargo.lock
The patch IS being applied (v1.1.4 from git). Let me verify the git repo has the fix:

cat ~/.cargo/git/checkouts/environmental-*/*/src/lib.rs | head -100
I see the issue - the macro thread_local! now needs the thread_local attribute (#[thread_local]) in new Rust versions, but this crate is still using the macro form. Let me check if there's a newer branch:

cd ~/.cargo/git/checkouts/environmental-*/* && git log --oneline -10
ls ~/.cargo/git/checkouts/environmental-*/ | head -5
cd ~/.cargo/git/checkouts/environmental-0c539e1b02c37435/02d89d5 && git log --oneline -10
This IS the latest version with the nightly fix. But Rust 1.85 still has the issue. Let me check the actual dates:

cd ~/.cargo/git/checkouts/environmental-0c539e1b02c37435/02d89d5 && git log --format="%h %ad %s" --date=short -10
The last update was in 2022 - the thread_local macro change in Rust 1.82 (late 2024) happened after. Let me check if there's a PR or fork that fixes this:

There's an open issue #6 about this exact problem. Let me check the PR that might fix it:

That's just a documentation update, not the thread_local fix. Let me check the forks for one that has the fix:

curl -s "https://api.github.com/repos/paritytech/environmental/forks" | python3 -c "import sys,json; d=json.load(sys.stdin); [print(f['full_name'], f['pushed_at']) for f in d]" | head -10
Let me check if RomarQ's fork has the fix since it was pushed recently:

That fork is also on the same master commit. The issue is that the environmental crate hasn't been updated to work with Rust 1.82+.

Let me try a different approach - use a completely different polkadot-sdk version that might have this fixed:

curl -s "https://api.github.com/repos/paritytech/polkadot-sdk/tags" | python3 -c "import sys,json; d=json.load(sys.stdin); [print(t['name']) for t in d[:20]]"
Let me check if a newer release like 1.16 or 1.17 has the environmental fix:

curl -sL "https://raw.githubusercontent.com/paritytech/polkadot-sdk/v1.17.0-rc5/substrate/primitives/externalities/Cargo.toml" 2>/dev/null | grep -A1 environmental
curl -sL "https://raw.githubusercontent.com/paritytech/polkadot-sdk/v1.17.0-rc5/Cargo.toml" 2>/dev/null | grep -A1 "environmental"
polkadot-sdk v1.17 uses environmental 1.1.4. But that's still the crates.io version which has the thread_local issue. Let me check if they patched it differently:

curl -sL "https://raw.githubusercontent.com/paritytech/polkadot-sdk/v1.17.0-rc5/Cargo.toml" 2>/dev/null | grep -A10 '\[patch.crates-io\]' | head -20
No patches. Let me check a different approach - what if I just fix the environmental crate source locally:

ls ~/.cargo/git/checkouts/environmental-*/*/src/
I can't directly edit the cargo cache. But I can create a local patch. Let me download environmental and patch it:

cd /home/lojak/Desktop/X3-atlas-sphere && mkdir -p patches && git clone https://github.com/paritytech/environmental patches/environmental 2>&1 | tail -5
lib.rs

I see the issue - line 65-67 uses thread_local! macro which in Rust 1.82+ must be imported differently. The issue is that thread_local macro was moved/changed. Let me check what works in newer Rust:

rustc +1.85.0 --edition 2021 -e 'thread_local! { static FOO: u32 = 42; }' 2>&1 || echo "needs std import"
cat > /tmp/tl_test.rs << 'EOF'
use std::thread_local;
thread_local! {
    static FOO: u32 = 42;
}
fn main() {}
EOF
rustc +1.85.0 /tmp/tl_test.rs 2>&1
rustc +1.85.0 /tmp/tl_test.rs -o /tmp/tl_test 2>&1
Works! The key is use std::thread_local; before using the macro. Let me patch the environmental crate:

Edited
lib.rs+5-1
Now update Cargo.toml to use the local patch:

Edited
Cargo.toml+2-2
rm Cargo.lock && cargo update 2>&1 | tail -20
cargo update -p home@0.5.12 --precise 0.5.9 2>&1
SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | tail -80
Now a different error - use of undeclared crate or module 'std'. The issue is in the macro expansion - when the macro uses std::thread_local!, the calling crate (sp-externalities) doesn't have std in scope because the macro is expanded in the caller's context.

Let me fix this properly - the macro needs to use the fully qualified path:

Edited
lib.rs+1-5
cargo clean -p environmental && SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | tail -80
Still failing. The problem is sp-externalities is compiled with no_std where ::std isn't available. The environmental crate has two paths - one for std and one for no_std. Let me look at the full macro:

lib.rs

I see the structure. The issue is that when using no_std, the macro works fine, but when sp-externalities is built with std, it fails because ::std::thread_local! isn't being recognized correctly by newer Rust.

Let me check what sp-externalities features are enabled:

grep -r "sp-externalities" ~/.cargo/git/checkouts/polkadot-sdk-*/*/substrate/primitives/externalities/Cargo.toml | head -10
cat ~/.cargo/git/checkouts/polkadot-sdk-dee0edd6eefa0594/e57fad1/substrate/primitives/externalities/Cargo.toml
I see - sp-externalities uses environmental/std when std feature is enabled. The problem is the macro expansion in newer Rust. Let me check if ::std::thread_local! is really the issue by testing:

cat > /tmp/tl_test2.rs << 'EOF'
#![allow(unused)]
macro_rules! my_thread_local {
    ($name:ident: $t:ty = $init:expr) => {
        ::std::thread_local!(static $name: $t = $init);
    };
}
my_thread_local!(FOO: u32 = 42);
fn main() {}
EOF
rustc +1.85.0 /tmp/tl_test2.rs -o /tmp/tl_test2 2>&1
Works. So the issue must be something else in sp-externalities context. Let me check the error more carefully - it says "could not find std in the list of imported crates". This happens when the crate is compiled in no_std mode!

Let me check if sp-externalities is being compiled without std:

cargo tree -p sp-externalities -f '{p} [{f}]' 2>&1 | head -20
cargo tree -p 'git+https://github.com/paritytech/polkadot-sdk?branch=release-polkadot-v1.1.0#sp-externalities@0.19.0' -f '{p} [{f}]' 2>&1 | head -30
I see - sp-externalities IS compiled with std feature and environmental has std feature. So ::std::thread_local! should work. But it's still failing.

Wait - in the error message it said could not find 'std' in the list of imported crates. That happens when the thread_local! macro is using a path that isn't found. The issue might be that in Rust 1.82+, the thread_local! macro changed how it works.

Let me search for how other projects fixed this issue:

The issue description says thread_local is in scope, but it is an attribute: #[thread_local]. This is the exact message I'm seeing - the change in Rust 1.82.

The comment says if thread_local macro isn't available, it's probably a no_std build. But I verified std feature is enabled. Let me check more carefully:

cargo tree -p environmental -f '{p} [{f}]' 2>&1 | head -10
environmental

has std feature. So the macro SHOULD pick the #[cfg(feature = "std")] branch.

Let me look at how the macro invokes thread_local!:

lib.rs

I see the issue! The macro thread_local_impl is called via $crate::thread_local_impl!. When this macro is exported and used by another crate, $crate refers to environmental, not std.

The thread_local_impl! macro uses ::std::thread_local! but this path isn't available when the macro is expanded in the context of sp-externalities crate - even though sp-externalities has std available.

The problem is that ::std::thread_local! needs to be able to find the thread_local! macro in the std crate, but in Rust 1.82+, the thread_local! macro changed and the path resolution is different.

Let me try a different approach - use an inline path that's guaranteed to work:

lib.rs

Let me try exporting thread_local from the environmental crate itself and using that re-export:

Edited
lib.rs+6-1
cargo clean -p environmental && SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | tail -60
PROGRESS! The sp-externalities now compiles with the patched environmental! We got past the thread_local issue.

Now we have a new error - assert_eq_size!(usize, u32) is failing because we're on a 64-bit system. This is a check in sp-runtime-interface that expects WASM (32-bit) compilation. This only fails for native builds when WASM build is being checked.

Let me see if this is a feature flag issue:

SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node --features std 2>&1 | tail -60
cargo tree -p sp-runtime-interface -f '{p} [{f}]' 2>&1 | head -15
cargo tree -p 'git+https://github.com/paritytech/polkadot-sdk?branch=release-polkadot-v1.1.0#sp-runtime-interface@17.0.0' -f '{p} [{f}]' 2>&1 | head -10
It already has disable_target_static_assertions feature! Let me check what's enabling it without that flag:

grep -r "disable_target_static_assertions" ~/.cargo/git/checkouts/polkadot-sdk-*/*/substrate/primitives/runtime-interface/Cargo.toml
grep -B5 -A5 "assert_eq_size" ~/.cargo/git/checkouts/polkadot-sdk-dee0edd6eefa0594/e57fad1/substrate/primitives/runtime-interface/src/impls.rs | head -20
I see - the assertion is gated on #[cfg(all(not(feature = "std"), not(feature = "disable_target_static_assertions")))]. Since we're building with std, this shouldn't trigger.

Wait - the error says it's compiling sp-runtime-interface (lib). Let me check which one is failing:

SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | grep "sp-runtime-interface"
It's the polkadot-sdk one. Let me check if it has std feature:

cargo metadata --format-version 1 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); pkg=[p for p in d['packages'] if p['name']=='sp-runtime-interface' and 'polkadot-sdk' in p['source']][0]; print('features:', list(pkg['features'].keys())[:20])" 2>&1
cargo tree -i 'git+https://github.com/paritytech/polkadot-sdk?branch=release-polkadot-v1.1.0#sp-runtime-interface@17.0.0' -f '{p} [{f}]' 2>&1 | head -30
cargo tree -i sp-runtime-interface 2>&1 | head -30
I see - both polkadot-sdk and substrate have sp-runtime-interface. The error was from the polkadot-sdk one. Let me check if it's being compiled without std:

cargo tree --format '{p} [{f}]' 2>/dev/null | grep "sp-runtime-interface" | head -10
The substrate one has std. Let me check the polkadot-sdk one:

cargo tree --format '{p} [{f}]' 2>/dev/null | grep "polkadot-sdk.*sp-runtime-interface" | head -5
No match. Let me search differently:

cargo tree --format '{p} [{f}]' 2>/dev/null | grep "sp-runtime-interface.*polkadot" | head -5
cargo tree --format '{p} [{f}]' 2>/dev/null | grep "sp-runtime-interface v17.*\[" | head -10
Interesting - only one with std. The error says polkadot-sdk. Let me try running the build again to see the full error:

SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | grep -A20 "error\[E0512\]"
The error is in polkadot-sdk-dee0edd6eefa0594 which is a separate checkout from our substrate git dep. This is coming from Frontier! Let me check what brings in this polkadot-sdk:

cargo tree -i sp-runtime-interface 2>&1 | grep -A1 "polkadot-sdk"
Command output will appear here once available.
cargo tree 2>/dev/null | grep "sp-runtime-interface" | head -20
Wait, that only shows the substrate one. So where is the polkadot-sdk one coming from? Let me check what Frontier depends on:

grep polkadot-sdk ~/.cargo/git/checkouts/frontier-*/*/*/Cargo.toml 2>/dev/null | head -5
grep -r "polkadot-sdk" ~/.cargo/git/checkouts/frontier-* 2>/dev/null | grep "Cargo.toml" | head -5
Frontier depends on polkadot-sdk. The polkadot-sdk dependencies need to have std or disable_target_static_assertions features.

This is getting complicated - we have two different sources for Substrate deps: our direct substrate git rev and Frontier's polkadot-sdk branch. They're not feature-unified.

Let me check if we need to patch the polkadot-sdk deps with the right features. Actually, let me first check if there's a simpler fix - enabling the feature via patch:

Cargo.toml

The polkadot-sdk sources should use our Substrate rev via patches. But the sp-runtime-interface from polkadot-sdk has the wrong features.

Wait - looking at the error again, it's from checkouts/polkadot-sdk-dee0edd6eefa0594 which is the release-polkadot-v1.1.0 branch. This is used by Frontier. The assertion check should be skipped with std feature but it seems std isn't enabled for that particular build.

Let me add sp-runtime-interface to the patches with the right features:

Edited
Cargo.toml+6-0
rm Cargo.lock && cargo update 2>&1 | tail -20
cargo update -p home@0.5.12 --precise 0.5.9 2>&1
SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | tail -80
Now we have a different problem - two versions of sp_core are being used. The polkadot-sdk version and our substrate rev are incompatible because we only patched a few packages.

Let me add more packages to the patch:

Edited
Cargo.toml+11-1
rm Cargo.lock && cargo update 2>&1 | tail -20
cargo update -p home@0.5.12 --precise 0.5.9 && SKIP_WASM_BUILD=1 cargo check -p atlas-sphere-node 2>&1 | tail -80
🎉 IT COMPILED! The build finished successfully with only warnings!

Let me run a quick validation:

SKIP_WASM_BUILD=1 cargo build --release -p atlas-sphere-node 2>&1 | tail -20
