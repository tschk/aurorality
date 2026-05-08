//! JavaScriptCore runtime wrapper.
//!
//! Wraps the JSC C API (JavaScriptCore.framework — system on macOS/iOS).
//! Each `JscRuntime` owns one global context. Not thread-safe by itself;
//! callers must hold a `Mutex` (see `JsPlugin`).

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

// ── Opaque JSC types ─────────────────────────────────────────────────────────
// The JSC C API defines all of these as pointer-to-opaque-struct.
// We declare them as enums (zero-variant, so they can never be constructed)
// and use raw pointers throughout.

// Opaque FFI types — never constructed, only used as *mut T behind a pointer.
// `#[repr(C)]` is not needed on the pointed-to type; the pointer itself is the ABI unit.
pub enum OpaqueJSContext {}
pub enum OpaqueJSString {}
pub enum OpaqueJSValue {}
pub enum OpaqueJSClass {}

pub type JSGlobalContextRef = *mut OpaqueJSContext;
pub type JSContextRef = *const OpaqueJSContext;
pub type JSStringRef = *mut OpaqueJSString;
// JSObjectRef and JSValueRef share the same underlying C type
pub type JSValueRef = *mut OpaqueJSValue;
pub type JSObjectRef = *mut OpaqueJSValue;
pub type JSClassRef = *mut OpaqueJSClass;
pub type JSPropertyAttributes = u32;

/// Signature for native callback functions registered with JSC.
pub type JSObjectCallAsFunctionCallback = unsafe extern "C" fn(
    ctx: JSContextRef,
    function: JSObjectRef,
    this_object: JSObjectRef,
    argument_count: usize,
    arguments: *const JSValueRef,
    exception: *mut JSValueRef,
) -> JSValueRef;

// ── JSC C API declarations ───────────────────────────────────────────────────

#[allow(dead_code)] // extern declarations: unused now but part of the stable API surface
#[link(name = "JavaScriptCore", kind = "framework")]
extern "C" {
    fn JSGlobalContextCreate(global_object_class: JSClassRef) -> JSGlobalContextRef;
    fn JSGlobalContextRelease(ctx: JSGlobalContextRef);
    fn JSContextGetGlobalObject(ctx: JSContextRef) -> JSObjectRef;

    fn JSStringCreateWithUTF8CString(string: *const c_char) -> JSStringRef;
    fn JSStringRelease(string: JSStringRef);
    fn JSStringGetMaximumUTF8CStringSize(string: JSStringRef) -> usize;
    fn JSStringGetUTF8CString(
        string: JSStringRef,
        buffer: *mut c_char,
        buffer_size: usize,
    ) -> usize;

    fn JSEvaluateScript(
        ctx: JSContextRef,
        script: JSStringRef,
        this_object: JSObjectRef,
        source_url: JSStringRef,
        starting_line_number: c_int,
        exception: *mut JSValueRef,
    ) -> JSValueRef;

    fn JSValueIsUndefined(ctx: JSContextRef, value: JSValueRef) -> bool;
    fn JSValueIsNull(ctx: JSContextRef, value: JSValueRef) -> bool;
    fn JSValueMakeUndefined(ctx: JSContextRef) -> JSValueRef;
    fn JSValueMakeString(ctx: JSContextRef, string: JSStringRef) -> JSValueRef;
    fn JSValueToStringCopy(
        ctx: JSContextRef,
        value: JSValueRef,
        exception: *mut JSValueRef,
    ) -> JSStringRef;

    fn JSObjectMake(
        ctx: JSContextRef,
        js_class: JSClassRef,
        data: *mut std::ffi::c_void,
    ) -> JSObjectRef;
    fn JSObjectMakeFunctionWithCallback(
        ctx: JSContextRef,
        name: JSStringRef,
        call_as_function: JSObjectCallAsFunctionCallback,
    ) -> JSObjectRef;
    fn JSObjectGetProperty(
        ctx: JSContextRef,
        object: JSObjectRef,
        property_name: JSStringRef,
        exception: *mut JSValueRef,
    ) -> JSValueRef;
    fn JSObjectSetProperty(
        ctx: JSContextRef,
        object: JSObjectRef,
        property_name: JSStringRef,
        value: JSValueRef,
        attributes: JSPropertyAttributes,
        exception: *mut JSValueRef,
    );
    fn JSObjectCallAsFunction(
        ctx: JSContextRef,
        object: JSObjectRef,
        this_object: JSObjectRef,
        argument_count: usize,
        arguments: *const JSValueRef,
        exception: *mut JSValueRef,
    ) -> JSValueRef;
}

// ── Safety marker ────────────────────────────────────────────────────────────

/// `JscRuntime` holds a raw pointer but is always accessed behind a `Mutex`
/// in `JsPlugin`, so cross-thread access never occurs in practice.
unsafe impl Send for JscRuntime {}

// ── Public runtime ───────────────────────────────────────────────────────────

pub struct JscRuntime {
    ctx: JSGlobalContextRef,
}

impl Default for JscRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl JscRuntime {
    /// Create a new JSC global context.
    pub fn new() -> Self {
        // SAFETY: JSGlobalContextCreate with null class pointer is safe;
        // JSC handles null by using the default global object class.
        let ctx = unsafe { JSGlobalContextCreate(std::ptr::null_mut()) };
        Self { ctx }
    }

    /// Inject `globalThis.aurorality.invoke(pluginId, method, payloadJson)` as a
    /// native callback that routes calls through `aurorality_core::bridge::invoke`.
    pub fn install_bridge_callback(&mut self) {
        // SAFETY: All JSC C API calls use the same context pointer, and all
        // JSStringRef alloc/release pairs are correctly balanced. This runs
        // while self is uniquely borrowed (&mut), preventing concurrent access.
        unsafe {
            let global = JSContextGetGlobalObject(self.ctx as JSContextRef);

            // Create `aurorality` plain object
            let aurorality_key = jsstr("aurorality");
            let aurorality_obj = JSObjectMake(
                self.ctx as JSContextRef,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // Create `aurorality.invoke` as a native function
            let invoke_key = jsstr("invoke");
            let invoke_fn = JSObjectMakeFunctionWithCallback(
                self.ctx as JSContextRef,
                invoke_key,
                aurorality_invoke_callback,
            );
            JSStringRelease(invoke_key);

            // Set aurorality.invoke = invoke_fn
            let invoke_key2 = jsstr("invoke");
            JSObjectSetProperty(
                self.ctx as JSContextRef,
                aurorality_obj,
                invoke_key2,
                invoke_fn as JSValueRef,
                0,
                std::ptr::null_mut(),
            );
            JSStringRelease(invoke_key2);

            // Set globalThis.aurorality = aurorality_obj
            JSObjectSetProperty(
                self.ctx as JSContextRef,
                global,
                aurorality_key,
                aurorality_obj as JSValueRef,
                0,
                std::ptr::null_mut(),
            );
            JSStringRelease(aurorality_key);
        }
    }

    /// Evaluate JS source, defining any top-level functions in the global scope.
    pub fn load_code(&mut self, code: &str) -> Result<(), String> {
        let script = to_jsstring(code);
        let mut exc: JSValueRef = std::ptr::null_mut();
        // SAFETY: JSEvaluateScript uses a valid context and JSStringRef.
        // The `exc` pointer is a stack-local that JSC writes to.
        // JSStringRelease matches the JSStringCreate that produced `script`.
        unsafe {
            JSEvaluateScript(
                self.ctx as JSContextRef,
                script,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut exc,
            );
            JSStringRelease(script);
            if !exc.is_null() {
                return Err(format!(
                    "JS load error: {}",
                    jsvalue_to_string(self.ctx as JSContextRef, exc)
                ));
            }
        }
        Ok(())
    }

    /// Call a top-level function named `name` with `payload_json` as its argument.
    ///
    /// The payload is passed as a JS object by evaling `(payload_json)` as an
    /// expression (valid since JSON is a subset of JS expressions when wrapped
    /// in parens). The return value is serialised to JSON via `JSON.stringify`.
    pub fn call_fn(&mut self, name: &str, payload_json: &str) -> Result<String, String> {
        // Wrap in IIFE: avoids any string escaping issues with the payload.
        // payload_json is valid JSON → valid JS expression when wrapped in `()`.
        let script_src = format!(
            "(function() {{ var __p = ({}); return JSON.stringify({}(__p)); }})()",
            payload_json, name
        );
        self.eval_to_string(&script_src)
    }

    /// Evaluate a JS expression and return its string representation.
    pub fn eval_to_string(&mut self, code: &str) -> Result<String, String> {
        let script = to_jsstring(code);
        let mut exc: JSValueRef = std::ptr::null_mut();
        // SAFETY: Same invariants as load_code: valid context, valid JSStringRef,
        // stack-local exception pointer, balanced JSStringRelease.
        let result = unsafe {
            let r = JSEvaluateScript(
                self.ctx as JSContextRef,
                script,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut exc,
            );
            JSStringRelease(script);
            if !exc.is_null() {
                return Err(format!(
                    "JS eval error: {}",
                    jsvalue_to_string(self.ctx as JSContextRef, exc)
                ));
            }
            r
        };
        if result.is_null() || unsafe { JSValueIsUndefined(self.ctx as JSContextRef, result) } {
            return Ok("null".to_string());
        }
        Ok(unsafe { jsvalue_to_string(self.ctx as JSContextRef, result) })
    }
}

impl Drop for JscRuntime {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            // SAFETY: JSGlobalContextRelease is called exactly once on the
            // non-null context created by JSGlobalContextCreate.
            unsafe { JSGlobalContextRelease(self.ctx) }
        }
    }
}

// ── Native callback for aurorality.invoke ────────────────────────────────────

/// Native C callback: `aurorality.invoke(pluginId, method, payloadJson) → string`
///
/// Routes the call through `aurorality_core::bridge::invoke` (the global Rust bridge)
/// and returns the JSON envelope string as a JSValue string.
///
/// # Safety
/// Called by JSC on the JS thread. Accesses the global bridge via `RwLock::read()`.
unsafe extern "C" fn aurorality_invoke_callback(
    ctx: JSContextRef,
    _function: JSObjectRef,
    _this: JSObjectRef,
    argument_count: usize,
    arguments: *const JSValueRef,
    exception: *mut JSValueRef,
) -> JSValueRef {
    if argument_count < 3 {
        let err =
            to_jsstring("aurorality.invoke requires 3 arguments: pluginId, method, payloadJson");
        let err_val = JSValueMakeString(ctx, err);
        JSStringRelease(err);
        *exception = err_val;
        return JSValueMakeUndefined(ctx);
    }

    let args = std::slice::from_raw_parts(arguments, argument_count);
    let plugin_id = jsvalue_to_string(ctx, args[0]);
    let method = jsvalue_to_string(ctx, args[1]);
    let payload_json = jsvalue_to_string(ctx, args[2]);

    let result_str = match crate::bridge::invoke(&plugin_id, &method, &payload_json) {
        Ok(s) => s,
        Err(e) => {
            let err_obj = serde_json::json!({"ok": false, "error": e.to_string()});
            serde_json::to_string(&err_obj)
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialization failed\"}".to_string())
        }
    };

    let result_jsstr = to_jsstring(&result_str);
    let result_val = JSValueMakeString(ctx, result_jsstr);
    JSStringRelease(result_jsstr);
    result_val
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Create a JSStringRef from a Rust &str. Caller must `JSStringRelease` it.
fn to_jsstring(s: &str) -> JSStringRef {
    let cstr = CString::new(s).unwrap_or_else(|_| CString::new("").unwrap());
    // SAFETY: JSStringCreateWithUTF8CString reads a valid NUL-terminated C
    // string pointer. The CString outlives the FFI call.
    unsafe { JSStringCreateWithUTF8CString(cstr.as_ptr()) }
}

/// Convenience: create a JSStringRef. Same as `to_jsstring` but shorter name for local use.
fn jsstr(s: &str) -> JSStringRef {
    to_jsstring(s)
}

/// Convert a JSValueRef to a Rust String via JSValueToStringCopy.
/// Returns "(error)" on failure.
unsafe fn jsvalue_to_string(ctx: JSContextRef, value: JSValueRef) -> String {
    if value.is_null() {
        return "(null)".to_string();
    }
    let mut exc: JSValueRef = std::ptr::null_mut();
    let js_str = JSValueToStringCopy(ctx, value, &mut exc);
    if js_str.is_null() || !exc.is_null() {
        return "(conversion error)".to_string();
    }
    let max = JSStringGetMaximumUTF8CStringSize(js_str);
    let mut buf = vec![0u8; max];
    JSStringGetUTF8CString(js_str, buf.as_mut_ptr() as *mut c_char, max);
    JSStringRelease(js_str);
    // Find null terminator
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..len]).into_owned()
}
