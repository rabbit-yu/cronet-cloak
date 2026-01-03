use crate::cronet_c::*;
use crate::cronet_pb::proxy_config::ProxyType;
use std::ffi::{c_void, CStr, CString};
use std::ptr;
use tokio::sync::oneshot;

// -----------------------------------------------------------------------------
// Cronet Engine
// -----------------------------------------------------------------------------

// C Wrapper removed. Using pure Rust implementation.

pub struct CronetEngine {
    ptr: Cronet_EnginePtr,
}

impl CronetEngine {
    pub fn new(user_agent: &str) -> Self {
        unsafe {
            let engine_ptr = Cronet_Engine_Create();
            let params_ptr = Cronet_EngineParams_Create();

            let c_ua = CString::new(user_agent).unwrap();
            Cronet_EngineParams_user_agent_set(params_ptr, c_ua.as_ptr());

            // Use true for params
            Cronet_EngineParams_enable_quic_set(params_ptr, true);
            Cronet_EngineParams_enable_http2_set(params_ptr, true);
            Cronet_EngineParams_enable_brotli_set(params_ptr, true);

            // Start the engine
            let res = Cronet_Engine_StartWithParams(engine_ptr, params_ptr);
            Cronet_EngineParams_Destroy(params_ptr);

            if res != Cronet_RESULT_Cronet_RESULT_SUCCESS {
                panic!("Failed to start Cronet Engine: {:?}", res);
            }

            CronetEngine { ptr: engine_ptr }
        }
    }

    pub fn start_request(
        &self,
        target: &crate::cronet_pb::TargetRequest,
        config: &crate::cronet_pb::ExecutionConfig,
    ) -> (
        CronetRequest,
        oneshot::Receiver<Result<RequestResult, String>>,
    ) {
        unsafe {
            eprintln!("[DEBUG] start_request entered");
            // Determine Engine to use (Shared or New Proxy Engine)
            let (engine_ptr, owned_engine_ptr) = if let Some(proxy) = &config.proxy {
                // Create Ad-hoc Engine with Proxy
                let engine = Cronet_Engine_Create();
                let params = Cronet_EngineParams_Create();

                let scheme = match ProxyType::try_from(proxy.r#type).unwrap_or(ProxyType::Http) {
                    ProxyType::Http => "http",
                    ProxyType::Https => "https",
                    ProxyType::Socks5 => "socks5",
                };

                // Build proxy URL with optional authentication
                let rules = if !proxy.username.is_empty() && !proxy.password.is_empty() {
                    format!(
                        "{}://{}:{}@{}:{}",
                        scheme, proxy.username, proxy.password, proxy.host, proxy.port
                    )
                } else {
                    format!("{}://{}:{}", scheme, proxy.host, proxy.port)
                };
                let c_rules = CString::new(rules).expect("Invalid proxy string");

                Cronet_EngineParams_proxy_rules_set(params, c_rules.as_ptr());

                Cronet_EngineParams_enable_quic_set(params, true);
                Cronet_EngineParams_enable_http2_set(params, true);

                Cronet_Engine_StartWithParams(engine, params);
                Cronet_EngineParams_Destroy(params);

                (engine, Some(engine))
            } else {
                (self.ptr, None)
            };

            // Channel to receive the final result
            let (tx, rx) = oneshot::channel();

            // Create Context to hold state across callbacks
            let context = Box::new(RequestContext {
                tx: Some(tx),
                response_buffer: Vec::new(),
                status_code: 0,
            });

            let context_ptr = Box::into_raw(context);

            // Executor
            // We use the same executor for request and upload
            let executor_ptr = Cronet_Executor_CreateWith(Some(executor_execute));
            Cronet_Executor_SetClientContext(executor_ptr, context_ptr as *mut c_void);

            // Callback
            let callback_ptr = Cronet_UrlRequestCallback_CreateWith(
                Some(on_redirect_received),
                Some(on_response_started),
                Some(on_read_completed),
                Some(on_succeeded),
                Some(on_failed),
                Some(on_canceled),
            );
            Cronet_UrlRequestCallback_SetClientContext(callback_ptr, context_ptr as *mut c_void);

            // Request & Params
            let request_ptr = Cronet_UrlRequest_Create();
            let params_ptr = Cronet_UrlRequestParams_Create();

            let c_method = CString::new(target.method.as_str()).unwrap();
            Cronet_UrlRequestParams_http_method_set(params_ptr, c_method.as_ptr());

            let c_url = CString::new(target.url.as_str()).unwrap();

            // Headers
            for (key, header_values) in &target.headers {
                let c_key = CString::new(key.as_str()).unwrap();
                for val in &header_values.values {
                    let c_val = CString::new(val.as_str()).unwrap();

                    let header_ptr = Cronet_HttpHeader_Create();
                    Cronet_HttpHeader_name_set(header_ptr, c_key.as_ptr());
                    Cronet_HttpHeader_value_set(header_ptr, c_val.as_ptr());

                    Cronet_UrlRequestParams_request_headers_add(params_ptr, header_ptr);

                    Cronet_HttpHeader_Destroy(header_ptr);
                }
            }

            // Upload Data Provider (Body)
            let mut upload_data_provider_ptr: Option<Cronet_UploadDataProviderPtr> = None;

            // Keep body alive
            let upload_body_data = if !target.body.is_empty() {
                Some(target.body.clone())
            } else {
                None
            };

            if let Some(body) = &upload_body_data {
                eprintln!(
                    "[DEBUG] Creating Rust UploadDataProvider. Body len: {}",
                    body.len()
                );

                let upload_context = Box::new(UploadContext {
                    data: body.clone(),
                    position: 0,
                });
                let upload_context_ptr = Box::into_raw(upload_context);

                let provider = Cronet_UploadDataProvider_CreateWith(
                    Some(upload_get_length),
                    Some(upload_read),
                    Some(upload_rewind),
                    Some(upload_close),
                );
                Cronet_UploadDataProvider_SetClientContext(
                    provider,
                    upload_context_ptr as *mut c_void,
                );

                Cronet_UrlRequestParams_upload_data_provider_set(params_ptr, provider);
                Cronet_UrlRequestParams_upload_data_provider_executor_set(params_ptr, executor_ptr);

                upload_data_provider_ptr = Some(provider);
            }

            Cronet_UrlRequest_InitWithParams(
                request_ptr,
                engine_ptr,
                c_url.as_ptr(),
                params_ptr,
                callback_ptr,
                executor_ptr,
            );

            Cronet_UrlRequestParams_Destroy(params_ptr);

            // Start
            eprintln!("[DEBUG] Starting Cronet Request");
            Cronet_UrlRequest_Start(request_ptr);

            // Return Handle that owns the cleanup
            let request_handle = CronetRequest {
                ptr: request_ptr,
                callback_ptr,
                executor_ptr,
                owned_engine_ptr,
                upload_data_provider_ptr,
                upload_body_data,
            };

            (request_handle, rx)
        }
    }
}

impl Drop for CronetEngine {
    fn drop(&mut self) {
        unsafe {
            Cronet_Engine_Shutdown(self.ptr);
            Cronet_Engine_Destroy(self.ptr);
        }
    }
}

unsafe impl Send for CronetEngine {}
unsafe impl Sync for CronetEngine {}

// -----------------------------------------------------------------------------
// Request Infrastructure
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RequestResult {
    pub status_code: i32,
    pub body: Vec<u8>,
}

#[allow(dead_code)]
pub struct CronetRequest {
    ptr: Cronet_UrlRequestPtr,
    callback_ptr: Cronet_UrlRequestCallbackPtr,
    executor_ptr: Cronet_ExecutorPtr,
    owned_engine_ptr: Option<Cronet_EnginePtr>,
    upload_data_provider_ptr: Option<Cronet_UploadDataProviderPtr>,
    upload_body_data: Option<Vec<u8>>, // Owns the body data so pointers are valid
}

unsafe impl Send for CronetRequest {}

impl Drop for CronetRequest {
    fn drop(&mut self) {
        unsafe {
            // Destroy Request first (blocks until callbacks complete, IF called from another thread)
            if !self.ptr.is_null() {
                Cronet_UrlRequest_Destroy(self.ptr);
            }
            if !self.callback_ptr.is_null() {
                Cronet_UrlRequestCallback_Destroy(self.callback_ptr);
            }
            if !self.executor_ptr.is_null() {
                Cronet_Executor_Destroy(self.executor_ptr);
            }
            if let Some(dp) = self.upload_data_provider_ptr {
                Cronet_UploadDataProvider_Destroy(dp);
            }
            // Finally destroy engine if we own it
            if let Some(engine_ptr) = self.owned_engine_ptr {
                Cronet_Engine_Shutdown(engine_ptr);
                Cronet_Engine_Destroy(engine_ptr);
            }
        }
    }
}

// Context passed to C callbacks
struct RequestContext {
    tx: Option<oneshot::Sender<Result<RequestResult, String>>>,
    response_buffer: Vec<u8>,
    status_code: i32,
}

// -----------------------------------------------------------------------------
// C Callbacks (Extern "C")
// -----------------------------------------------------------------------------

unsafe extern "C" fn executor_execute(_self: Cronet_ExecutorPtr, command: Cronet_RunnablePtr) {
    eprintln!("[DEBUG] executor_execute called");
    Cronet_Runnable_Run(command);
    eprintln!("[DEBUG] executor_execute finished command");
    Cronet_Runnable_Destroy(command);
}

// UrlRequest Callbacks
unsafe extern "C" fn on_redirect_received(
    _self: Cronet_UrlRequestCallbackPtr,
    request: Cronet_UrlRequestPtr,
    _info: Cronet_UrlResponseInfoPtr,
    _new_location_url: Cronet_String,
) {
    Cronet_UrlRequest_FollowRedirect(request);
}

unsafe extern "C" fn on_response_started(
    self_: Cronet_UrlRequestCallbackPtr,
    request: Cronet_UrlRequestPtr,
    info: Cronet_UrlResponseInfoPtr,
) {
    eprintln!("[DEBUG] on_response_started");
    let context_ptr = Cronet_UrlRequestCallback_GetClientContext(self_) as *mut RequestContext;
    let context = &mut *context_ptr;

    context.status_code = Cronet_UrlResponseInfo_http_status_code_get(info);

    let buffer_ptr = Cronet_Buffer_Create();
    Cronet_Buffer_InitWithAlloc(buffer_ptr, 32 * 1024);

    Cronet_UrlRequest_Read(request, buffer_ptr);
}

unsafe extern "C" fn on_read_completed(
    self_: Cronet_UrlRequestCallbackPtr,
    request: Cronet_UrlRequestPtr,
    _info: Cronet_UrlResponseInfoPtr,
    buffer: Cronet_BufferPtr,
    bytes_read: u64,
) {
    eprintln!("[DEBUG] on_read_completed: {} bytes", bytes_read);
    let context_ptr = Cronet_UrlRequestCallback_GetClientContext(self_) as *mut RequestContext;
    let context = &mut *context_ptr;

    let data_ptr = Cronet_Buffer_GetData(buffer);
    let slice = std::slice::from_raw_parts(data_ptr as *const u8, bytes_read as usize);
    context.response_buffer.extend_from_slice(slice);

    Cronet_Buffer_Destroy(buffer);

    let new_buffer = Cronet_Buffer_Create();
    Cronet_Buffer_InitWithAlloc(new_buffer, 32 * 1024);

    Cronet_UrlRequest_Read(request, new_buffer);
}

unsafe extern "C" fn on_succeeded(
    self_: Cronet_UrlRequestCallbackPtr,
    _request: Cronet_UrlRequestPtr,
    _info: Cronet_UrlResponseInfoPtr,
) {
    eprintln!("[DEBUG] on_succeeded");
    complete_request(self_, Ok(()));
}

unsafe extern "C" fn on_failed(
    self_: Cronet_UrlRequestCallbackPtr,
    _request: Cronet_UrlRequestPtr,
    _info: Cronet_UrlResponseInfoPtr,
    error: Cronet_ErrorPtr,
) {
    eprintln!("[DEBUG] on_failed");
    let msg = CStr::from_ptr(Cronet_Error_message_get(error))
        .to_string_lossy()
        .into_owned();
    complete_request(self_, Err(msg));
}

unsafe extern "C" fn on_canceled(
    self_: Cronet_UrlRequestCallbackPtr,
    _request: Cronet_UrlRequestPtr,
    _info: Cronet_UrlResponseInfoPtr,
) {
    eprintln!("[DEBUG] on_canceled");
    complete_request(self_, Err("Canceled".to_string()));
}

unsafe fn complete_request(callback_ptr: Cronet_UrlRequestCallbackPtr, result: Result<(), String>) {
    let context_ptr =
        Cronet_UrlRequestCallback_GetClientContext(callback_ptr) as *mut RequestContext;
    // Take ownership back to drop it.
    let context = Box::from_raw(context_ptr);

    eprintln!("[DEBUG] complete_request: {:?}", result);

    if let Some(tx) = context.tx {
        match result {
            Ok(_) => {
                let res = RequestResult {
                    status_code: context.status_code,
                    body: context.response_buffer.clone(),
                };
                let _ = tx.send(Ok(res));
            }
            Err(e) => {
                let _ = tx.send(Err(e));
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Upload Data Provider Callbacks
// -----------------------------------------------------------------------------

struct UploadContext {
    data: Vec<u8>,
    position: u64,
}

unsafe extern "C" fn upload_get_length(self_: Cronet_UploadDataProviderPtr) -> i64 {
    let context_ptr = Cronet_UploadDataProvider_GetClientContext(self_) as *mut UploadContext;
    let context = &*context_ptr;
    context.data.len() as i64
}

unsafe extern "C" fn upload_read(
    self_: Cronet_UploadDataProviderPtr,
    sink: Cronet_UploadDataSinkPtr,
    buffer: Cronet_BufferPtr,
) {
    let context_ptr = Cronet_UploadDataProvider_GetClientContext(self_) as *mut UploadContext;
    let context = &mut *context_ptr;

    let buffer_size = Cronet_Buffer_GetSize(buffer);
    let buffer_data = Cronet_Buffer_GetData(buffer) as *mut u8;

    let remaining = (context.data.len() as u64) - context.position;
    let to_read = std::cmp::min(buffer_size, remaining);

    if to_read > 0 {
        ptr::copy_nonoverlapping(
            context.data.as_ptr().add(context.position as usize),
            buffer_data,
            to_read as usize,
        );
        context.position += to_read;
    }

    Cronet_UploadDataSink_OnReadSucceeded(sink, to_read, false);
}

unsafe extern "C" fn upload_rewind(
    self_: Cronet_UploadDataProviderPtr,
    sink: Cronet_UploadDataSinkPtr,
) {
    let context_ptr = Cronet_UploadDataProvider_GetClientContext(self_) as *mut UploadContext;
    let context = &mut *context_ptr;
    context.position = 0;
    Cronet_UploadDataSink_OnRewindSucceeded(sink);
}

unsafe extern "C" fn upload_close(self_: Cronet_UploadDataProviderPtr) {
    let context_ptr = Cronet_UploadDataProvider_GetClientContext(self_) as *mut UploadContext;
    // Take ownership to drop
    let _ = Box::from_raw(context_ptr);
}
