// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Proof size extension for tracking trie access sizes.
//! Stub implementation for API compatibility with cumulus-primitives-proof-size-hostfunction.

use sp_std::{marker::PhantomData, any::{Any, TypeId}};
// Use the same sp_externalities version as sp-trie
#[cfg(feature = "std")]
use sp_externalities::Extension as SpExternalitiesExtension;

/// Proof size tracker struct for use as a generic type in extension::<ProofSizeExt>().
/// Cumulus uses this as a marker type to enable proof size recording.
pub struct ProofSizeExt {
	size: u32,
}

impl ProofSizeExt {
	/// Create a new proof size extension.
	pub fn new() -> Self {
		Self { size: 0 }
	}

	/// Get the current proof size.
	pub fn storage_proof_size(&self) -> u64 {
		self.size as u64
	}

	/// Add to the proof size.
	pub fn add_size(&mut self, additional: u32) {
		self.size = self.size.saturating_add(additional);
	}
}

impl Default for ProofSizeExt {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(feature = "std")]
impl SpExternalitiesExtension for ProofSizeExt {
	fn as_mut_any(&mut self) -> &mut dyn Any {
		self
	}

	fn type_id(&self) -> TypeId {
		TypeId::of::<ProofSizeExt>()
	}
}

/// Proof size recorder - struct for compatibility.
pub struct ProofSizeRecorder<T> {
	_phantom: PhantomData<T>,
}

impl<T> ProofSizeRecorder<T> {
	/// Create a new proof size recorder.
	pub fn new() -> Self {
		Self {
			_phantom: PhantomData,
		}
	}
}

impl<T> Default for ProofSizeRecorder<T> {
	fn default() -> Self {
		Self::new()
	}
}
