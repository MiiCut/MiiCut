macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, WebSocket};

fn looks_ascii(buf: &[u8]) -> bool {
    buf.iter()
        .all(|&b| b == b'\r' || b == b'\n' || (b >= 0x20 && b <= 0x7E))
}

pub struct CncLink {
    pub base_http: String, // ex "http://192.168.1.36"
    pub ws_url: String,    // ex "ws://192.168.1.36:81/"
    ws: WebSocket,
}

impl CncLink {
    pub fn connect(base_http: &str, ws_url: &str) -> Result<Self, JsValue> {
        // sous-protocole webui-v3
        let protocols = js_sys::Array::new();
        protocols.push(&JsValue::from_str("webui-v3"));

        let ws = WebSocket::new_with_str_sequence(ws_url, &protocols)?;
        ws.set_binary_type(BinaryType::Arraybuffer);

        // onopen
        {
            let onopen = Closure::<dyn FnMut(Event)>::new(move |_| {
                log!("WS open");
            });
            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();
        }

        // onmessage (UNIQUE)
        {
            let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                let data = e.data();

                // 1) string direct
                if let Some(s) = data.as_string() {
                    log!("[WS text] {}", s);
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
                    wasm_bindgen_futures::spawn_local(async move {
                        let p = blob.array_buffer();
                        let js = wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
                        let abuf: js_sys::ArrayBuffer = js.dyn_into().unwrap();
                        let u8 = js_sys::Uint8Array::new(&abuf);

                        let mut buf = vec![0u8; u8.length() as usize];
                        u8.copy_to(&mut buf[..]);

                        let s = String::from_utf8_lossy(&buf);
                        log!("[WS blob] {}", s);
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
            let onclose = Closure::<dyn FnMut(CloseEvent)>::new(move |e: CloseEvent| {
                web_sys::console::log_3(
                    &"WS close".into(),
                    &JsValue::from_f64(e.code() as f64),
                    &e.reason().into(),
                );
            });
            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            onclose.forget();
        }

        // onerror
        {
            let onerror = Closure::<dyn FnMut(Event)>::new(move |_| {
                web_sys::console::log_1(&"WS error".into());
            });
            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();
        }

        Ok(Self {
            base_http: base_http.to_string(),
            ws_url: ws_url.to_string(),
            ws,
        })
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
}
