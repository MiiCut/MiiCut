macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into())
    }
}

macro_rules! init_element {
    ($doc:expr, $id:expr, $ty:ty) => {{
        $doc.get_element_by_id($id)
            .expect(concat!("missing ", $id))
            .dyn_into::<$ty>()?
    }};
}
