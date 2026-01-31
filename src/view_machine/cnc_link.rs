macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

use js_sys::Date;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, WebSocket};

fn looks_ascii(buf: &[u8]) -> bool {
    buf.iter()
        .all(|&b| b == b'\r' || b == b'\n' || (0x20..=0x7E).contains(&b))
}

type CncCallback = Rc<RefCell<Option<Box<dyn FnMut(String)>>>>;

pub struct CncLink {
    pub base_http: String, // ex "http://192.168.1.36"
    pub ws_url: String,    // ex "ws://192.168.1.36:81/"
    _ws: WebSocket,
    on_text: CncCallback,
    on_status: CncCallback,
}

impl CncLink {
    pub fn connect(base_http: &str, ws_url: &str) -> Result<Self, JsValue> {
        // sous-protocole webui-v3
        let protocols = js_sys::Array::new();
        protocols.push(&JsValue::from_str("webui-v3"));

        let ws = WebSocket::new_with_str_sequence(ws_url, &protocols)?;
        ws.set_binary_type(BinaryType::Arraybuffer);
        let on_text: CncCallback = Rc::new(RefCell::new(None));
        let on_status: CncCallback = Rc::new(RefCell::new(None));

        // onopen
        {
            let on_status = on_status.clone();
            let onopen = Closure::<dyn FnMut(Event)>::new(move |_| {
                log!("WS open");
                if let Some(cb) = on_status.borrow_mut().as_mut() {
                    cb("WS connected".to_string());
                }
            });
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();
        }

        // onmessage
        {
            let on_text = on_text.clone();
            let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                let data = e.data();

                // 1) string direct
                if let Some(s) = data.as_string() {
                    log!("[WS text] {}", s);
                    if let Some(cb) = on_text.borrow_mut().as_mut() {
                        cb(s);
                    }
                    return;
                }

                // 2) ArrayBuffer direct
                if let Ok(abuf) = data.clone().dyn_into::<js_sys::ArrayBuffer>() {
                    let u8 = js_sys::Uint8Array::new(&abuf);

                    // IMPORTANT: pas de to_vec() temporaire emprunté
                    let mut buf = vec![0u8; u8.length() as usize];
                    u8.copy_to(&mut buf[..]);

                    if looks_ascii(&buf) {
                        let s = String::from_utf8_lossy(&buf);
                        log!("[WS bin-ascii] {}", s.trim_end());
                        if let Some(cb) = on_text.borrow_mut().as_mut() {
                            cb(s.to_string());
                        }
                    } else {
                        // vrai binaire
                        let head = buf
                            .iter()
                            .take(32)
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        log!("[WS bin] len={} head={}", buf.len(), head);
                    }
                    return;
                }

                // 3) Blob (selon navigateur / config)
                if let Ok(blob) = data.clone().dyn_into::<web_sys::Blob>() {
                    let on_text_async = on_text.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        let p = blob.array_buffer();
                        let js = wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
                        let abuf: js_sys::ArrayBuffer = js.dyn_into().unwrap();
                        let u8 = js_sys::Uint8Array::new(&abuf);

                        let mut buf = vec![0u8; u8.length() as usize];
                        u8.copy_to(&mut buf[..]);

                        let s = String::from_utf8_lossy(&buf);
                        log!("[WS blob] {}", s);
                        if let Some(cb) = on_text_async.borrow_mut().as_mut() {
                            cb(s.to_string());
                        }
                    });
                    return;
                }

                // fallback
                web_sys::console::log_2(&"[WS other]".into(), &data);
            });

            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();
        }

        // onclose
        {
            let on_status = on_status.clone();
            let onclose = Closure::<dyn FnMut(CloseEvent)>::new(move |e: CloseEvent| {
                web_sys::console::log_3(
                    &"WS close".into(),
                    &JsValue::from_f64(e.code() as f64),
                    &e.reason().into(),
                );
                if let Some(cb) = on_status.borrow_mut().as_mut() {
                    cb(format!("WS closed ({} {})", e.code(), e.reason()));
                }
            });
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            onclose.forget();
        }

        // onerror
        {
            let on_status = on_status.clone();
            let onerror = Closure::<dyn FnMut(Event)>::new(move |_| {
                web_sys::console::log_1(&"WS error".into());
                if let Some(cb) = on_status.borrow_mut().as_mut() {
                    cb("WS error".to_string());
                }
            });
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();
        }

        Ok(Self {
            base_http: base_http.to_string(),
            ws_url: ws_url.to_string(),
            _ws: ws,
            on_text,
            on_status,
        })
    }

    pub fn set_on_text_handler(&self, handler: Option<Box<dyn FnMut(String)>>) {
        *self.on_text.borrow_mut() = handler;
    }

    pub fn set_on_status_handler(&self, handler: Option<Box<dyn FnMut(String)>>) {
        *self.on_status.borrow_mut() = handler;
    }

    pub async fn send_http_cmd(&self, cmd: &str) -> Result<(), JsValue> {
        let url = format!(
            "{}/command?cmd={}",
            self.base_http,
            urlencoding::encode(cmd)
        );

        let resp_js =
            wasm_bindgen_futures::JsFuture::from(web_sys::window().unwrap().fetch_with_str(&url))
                .await?;

        // optionnel: vérifier status
        let _resp: web_sys::Response = resp_js.dyn_into()?;

        Ok(())
    }

    pub async fn send_http_cmd_ts(&self, cmd: &str) -> Result<bool, JsValue> {
        let url = format!(
            "{}/command?cmd={}&t={}",
            self.base_http,
            urlencoding::encode(cmd),
            Date::now() as u64
        );

        let resp_js =
            wasm_bindgen_futures::JsFuture::from(web_sys::window().unwrap().fetch_with_str(&url))
                .await?;

        let resp: web_sys::Response = resp_js.dyn_into()?;
        let ok_status = resp.ok();
        let text_js = wasm_bindgen_futures::JsFuture::from(resp.text()?).await?;
        let text = text_js.as_string().unwrap_or_default();
        let text = text.trim();
        let ok_body = text.is_empty() || text.eq_ignore_ascii_case("ok") || text.contains("ok");
        Ok(ok_status && ok_body)
    }
}
