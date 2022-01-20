use std::collections::HashMap;

use lazy_static::lazy_static;

lazy_static! {
    static ref CODE_TO_LANG: HashMap<&'static str, &'static str> =
        std::array::IntoIter::new([("de", "german"), ("en", "english"),]).collect();
}

pub(crate) fn get_name(code: &str) -> Option<String> {
    CODE_TO_LANG.get(code).map(|name| (*name).to_string())
}
