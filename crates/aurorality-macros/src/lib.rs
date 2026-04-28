//! Proc-macros for `aurorality`.
//!
//! Currently a thin wrapper around UniFFI's macros — future versions may add
//! plugin registration and codegen.

use proc_macro::TokenStream;
use quote::quote;

/// Export a function, method, or impl block to Swift via UniFFI.
///
/// This is a convenience alias for `#[uniffi::export]`.  Use it on any
/// function or impl block that should be callable from Swift:
///
/// ```ignore
/// #[aurorality::plugin]
/// pub fn greet(name: String) -> String {
///     format!("Hello, {name}!")
/// }
/// ```
///
/// For constructors inside exported impl blocks, annotate the constructor with
/// `#[uniffi::constructor]`:
///
/// ```ignore
/// #[derive(uniffi::Object)]
/// pub struct MyState;
///
/// #[aurorality::plugin]
/// impl MyState {
///     #[uniffi::constructor]
///     pub fn new() -> Self { Self }
///
///     pub fn value(&self) -> u32 { 42 }
/// }
/// ```
#[proc_macro_attribute]
pub fn plugin(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let input = proc_macro2::TokenStream::from(input);
    quote! {
        #[::uniffi::export(#args)]
        #input
    }
    .into()
}
