// Copyright 2019-2021 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use std::future::Future;

use jsonrpsee_core::server::rpc_module::ConnectionId;

tokio::task_local! {
	static CURRENT_CONNECTION_ID: ConnectionId;
}

pub(crate) async fn with_connection_id<F, T>(connection_id: ConnectionId, fut: F) -> T
where
	F: Future<Output = T>,
{
	CURRENT_CONNECTION_ID.scope(connection_id, fut).await
}

/// Returns the active RPC connection ID for the in-flight method call, if available.
pub fn current_connection_id() -> Option<ConnectionId> {
	CURRENT_CONNECTION_ID.try_with(|connection_id| *connection_id).ok()
}
