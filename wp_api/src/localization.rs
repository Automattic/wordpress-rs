use std::{fmt::Debug, sync::Arc};
use wp_localization::{WpLocale, WpLocalizable};

#[derive(Debug, uniffi::Object)]
struct UniffiLocalizable(Arc<dyn WpLocalizable>);

#[uniffi::export]
impl UniffiLocalizable {
    fn localize(&self, locale: Option<WpLocale>) -> String {
        self.0.localize(locale)
    }
}
