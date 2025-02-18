pub mod localization {
    use fluent_static::MessageBundle;
    #[cfg(not(trybuild))]
    const _: &str = "foo_bar = Foo is bar\nfoo_bar_with_arg = Foo is {$bar_arg}\nfoo_error_bar = Foo is bar\nfoo_error_baz = Foo is {$baz}\nfoo_error_bazzz = Foo is {$value1} & {$value2}\n";
    pub enum MessagesBundleLanguage {
        LangEnUs,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for MessagesBundleLanguage {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "LangEnUs")
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for MessagesBundleLanguage {
        #[inline]
        fn clone(&self) -> MessagesBundleLanguage {
            MessagesBundleLanguage::LangEnUs
        }
    }
    impl MessagesBundleLanguage {
        const LANGUAGE_IDS: [&'static str; 1] = ["en-US"];
        fn get(lang_id: &str) -> Option<Self> {
            match lang_id {
                "en-US" => Some(Self::LangEnUs),
                _ => None,
            }
        }
        fn language_ids() -> &'static [&'static str] {
            &Self::LANGUAGE_IDS
        }
        fn plural_rules_cardinal(
            &self,
        ) -> &'static ::fluent_static::intl_pluralrules::PluralRules {
            match self {
                Self::LangEnUs => {
                    static RULES: ::fluent_static::once_cell::sync::Lazy<
                        ::fluent_static::intl_pluralrules::PluralRules,
                    > = ::fluent_static::once_cell::sync::Lazy::new(|| {
                        ::fluent_static::intl_pluralrules::PluralRules::create(
                                ::fluent_static::unic_langid::LanguageIdentifier::from_bytes(
                                        "en-US".as_bytes(),
                                    )
                                    .unwrap(),
                                ::fluent_static::intl_pluralrules::PluralRuleType::CARDINAL,
                            )
                            .unwrap()
                    });
                    &RULES
                }
            }
        }
    }
    impl ::fluent_static::LanguageAware for self::MessagesBundleLanguage {
        fn language_id(&self) -> &str {
            match self {
                Self::LangEnUs => "en-US",
            }
        }
    }
    impl ::core::default::Default for self::MessagesBundleLanguage {
        fn default() -> Self {
            Self::LangEnUs
        }
    }
    pub struct Messages {
        language: self::MessagesBundleLanguage,
        formatter: Option<::fluent_static::formatter::FormatterFn>,
        use_isolating: bool,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Messages {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Messages",
                "language",
                &self.language,
                "formatter",
                &self.formatter,
                "use_isolating",
                &&self.use_isolating,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Messages {
        #[inline]
        fn clone(&self) -> Messages {
            Messages {
                language: ::core::clone::Clone::clone(&self.language),
                formatter: ::core::clone::Clone::clone(&self.formatter),
                use_isolating: ::core::clone::Clone::clone(&self.use_isolating),
            }
        }
    }
    impl ::fluent_static::LanguageAware for self::Messages {
        fn language_id(&self) -> &str {
            self.language.language_id()
        }
    }
    impl ::fluent_static::MessageBundle for self::Messages {
        fn get(language_id: &str) -> Option<Self> {
            self::MessagesBundleLanguage::get(language_id)
                .map(|language| Self {
                    language,
                    ..Default::default()
                })
        }
        fn default_language_id() -> &'static str {
            "en-US"
        }
        fn supported_language_ids() -> &'static [&'static str] {
            self::MessagesBundleLanguage::language_ids()
        }
    }
    impl ::core::default::Default for self::Messages {
        fn default() -> Self {
            Self {
                language: self::MessagesBundleLanguage::default(),
                formatter: None,
                use_isolating: true,
            }
        }
    }
    impl Messages {
        fn _write_<W: ::std::fmt::Write>(
            &self,
            value: &::fluent_static::value::Value,
            out: &mut W,
        ) -> ::std::fmt::Result {
            if self.use_isolating {
                out.write_char('\u{2068}')?;
            }
            if let Some(formatter) = self.formatter.as_ref() {
                (formatter)(
                    ::fluent_static::LanguageAware::language_id(self),
                    value,
                    out,
                )?;
            } else {
                ::fluent_static::formatter::format(
                    ::fluent_static::LanguageAware::language_id(self),
                    value,
                    out,
                )?;
            }
            if self.use_isolating {
                out.write_char('\u{2069}')?;
            }
            Ok(())
        }
        pub fn set_use_isolating(&mut self, value: bool) {
            self.use_isolating = value;
        }
        pub fn set_value_formatter(
            &mut self,
            formatter_fn: Option<::fluent_static::formatter::FormatterFn>,
        ) {
            self.formatter = formatter_fn;
        }
    }
    impl Messages {
        pub fn foo_bar(&self) -> ::fluent_static::Message {
            let mut out = String::new();
            match self.language {
                self::MessagesBundleLanguage::LangEnUs => self.en_us_foo_bar(&mut out),
            }
                .unwrap();
            ::fluent_static::Message::from(out)
        }
        pub fn foo_bar_with_arg<'a>(
            &self,
            bar_arg: impl Into<::fluent_static::value::Value<'a>>,
        ) -> ::fluent_static::Message {
            let bar_arg = bar_arg.into();
            let mut out = String::new();
            match self.language {
                self::MessagesBundleLanguage::LangEnUs => {
                    self.en_us_foo_bar_with_arg(&mut out, bar_arg)
                }
            }
                .unwrap();
            ::fluent_static::Message::from(out)
        }
        pub fn foo_error_bar(&self) -> ::fluent_static::Message {
            let mut out = String::new();
            match self.language {
                self::MessagesBundleLanguage::LangEnUs => {
                    self.en_us_foo_error_bar(&mut out)
                }
            }
                .unwrap();
            ::fluent_static::Message::from(out)
        }
        pub fn foo_error_baz<'a>(
            &self,
            baz: impl Into<::fluent_static::value::Value<'a>>,
        ) -> ::fluent_static::Message {
            let baz = baz.into();
            let mut out = String::new();
            match self.language {
                self::MessagesBundleLanguage::LangEnUs => {
                    self.en_us_foo_error_baz(&mut out, baz)
                }
            }
                .unwrap();
            ::fluent_static::Message::from(out)
        }
        pub fn foo_error_bazzz<'a>(
            &self,
            value_1: impl Into<::fluent_static::value::Value<'a>>,
            value_2: impl Into<::fluent_static::value::Value<'a>>,
        ) -> ::fluent_static::Message {
            let value_1 = value_1.into();
            let value_2 = value_2.into();
            let mut out = String::new();
            match self.language {
                self::MessagesBundleLanguage::LangEnUs => {
                    self.en_us_foo_error_bazzz(&mut out, value_1, value_2)
                }
            }
                .unwrap();
            ::fluent_static::Message::from(out)
        }
    }
    impl Messages {
        #[inline]
        fn en_us_foo_bar<W: ::std::fmt::Write>(
            &self,
            out: &mut W,
        ) -> ::std::fmt::Result {
            out.write_str("Foo is bar")?;
            Ok(())
        }
        #[inline]
        fn en_us_foo_bar_with_arg<'a, W: ::std::fmt::Write>(
            &self,
            out: &mut W,
            bar_arg: ::fluent_static::value::Value<'a>,
        ) -> ::std::fmt::Result {
            out.write_str("Foo is ")?;
            self._write_(&bar_arg, out)?;
            Ok(())
        }
        #[inline]
        fn en_us_foo_error_bar<W: ::std::fmt::Write>(
            &self,
            out: &mut W,
        ) -> ::std::fmt::Result {
            out.write_str("Foo is bar")?;
            Ok(())
        }
        #[inline]
        fn en_us_foo_error_baz<'a, W: ::std::fmt::Write>(
            &self,
            out: &mut W,
            baz: ::fluent_static::value::Value<'a>,
        ) -> ::std::fmt::Result {
            out.write_str("Foo is ")?;
            self._write_(&baz, out)?;
            Ok(())
        }
        #[inline]
        fn en_us_foo_error_bazzz<'a, W: ::std::fmt::Write>(
            &self,
            out: &mut W,
            value_1: ::fluent_static::value::Value<'a>,
            value_2: ::fluent_static::value::Value<'a>,
        ) -> ::std::fmt::Result {
            out.write_str("Foo is ")?;
            self._write_(&value_1, out)?;
            out.write_str(" & ")?;
            self._write_(&value_2, out)?;
            Ok(())
        }
    }
    pub trait WpLocalizedError: Send + Sync {
        fn localized_error_message(&self, locale_id: String) -> String;
    }
    const UNIFFI_META_CONST_WP_API_INTERFACE_WPLOCALIZEDERROR: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::CALLBACK_TRAIT_INTERFACE,
        )
        .concat_str("wp_api")
        .concat_str("WpLocalizedError")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_INTERFACE_WPLOCALIZEDERROR: [u8; UNIFFI_META_CONST_WP_API_INTERFACE_WPLOCALIZEDERROR
        .size] = UNIFFI_META_CONST_WP_API_INTERFACE_WPLOCALIZEDERROR.into_array();
    #[doc(hidden)]
    #[no_mangle]
    /// Clone a pointer to this object type
    ///
    /// Safety: Only pass pointers returned by a UniFFI call.  Do not pass pointers that were
    /// passed to the free function.
    pub unsafe extern "C" fn uniffi_wp_api_fn_clone_wplocalizederror(
        ptr: *const ::std::ffi::c_void,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> *const ::std::ffi::c_void {
        ::uniffi::rust_call(
            call_status,
            || {
                let ptr = ptr as *mut ::std::sync::Arc<dyn WpLocalizedError>;
                let arc: ::std::sync::Arc<_> = unsafe {
                    ::std::clone::Clone::clone(&*ptr)
                };
                ::std::result::Result::Ok(
                    ::std::boxed::Box::into_raw(::std::boxed::Box::new(arc))
                        as *const ::std::ffi::c_void,
                )
            },
        )
    }
    #[doc(hidden)]
    #[no_mangle]
    /// Free a pointer to this object type
    ///
    /// Safety: Only pass pointers returned by a UniFFI call.  Do not pass pointers that were
    /// passed to the free function.
    ///
    /// Note: clippy doesn't complain about this being unsafe, but it definitely is since it
    /// calls `Box::from_raw`.
    pub unsafe extern "C" fn uniffi_wp_api_fn_free_wplocalizederror(
        ptr: *const ::std::ffi::c_void,
        call_status: &mut ::uniffi::RustCallStatus,
    ) {
        ::uniffi::rust_call(
            call_status,
            || {
                if !!ptr.is_null() {
                    ::core::panicking::panic("assertion failed: !ptr.is_null()")
                }
                ::std::mem::drop(unsafe {
                    ::std::boxed::Box::from_raw(
                        ptr as *mut ::std::sync::Arc<dyn WpLocalizedError>,
                    )
                });
                ::std::result::Result::Ok(())
            },
        );
    }
    pub struct UniFfiTraitVtableWpLocalizedError {
        pub localized_error_message: extern "C" fn(
            uniffi_handle: u64,
            locale_id: <String as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
            uniffi_out_return: &mut <String as ::uniffi::LiftReturn<
                crate::UniFfiTag,
            >>::ReturnType,
            uniffi_out_call_status: &mut ::uniffi::RustCallStatus,
        ),
        pub uniffi_free: extern "C" fn(handle: u64),
    }
    static UNIFFI_TRAIT_CELL_WPLOCALIZEDERROR: ::uniffi::UniffiForeignPointerCell<
        UniFfiTraitVtableWpLocalizedError,
    > = ::uniffi::UniffiForeignPointerCell::<UniFfiTraitVtableWpLocalizedError>::new();
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_init_callback_vtable_wplocalizederror(
        vtable: ::std::ptr::NonNull<UniFfiTraitVtableWpLocalizedError>,
    ) {
        UNIFFI_TRAIT_CELL_WPLOCALIZEDERROR.set(vtable);
    }
    struct UniFFICallbackHandlerWpLocalizedError {
        handle: u64,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UniFFICallbackHandlerWpLocalizedError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "UniFFICallbackHandlerWpLocalizedError",
                "handle",
                &&self.handle,
            )
        }
    }
    impl UniFFICallbackHandlerWpLocalizedError {
        fn new(handle: u64) -> Self {
            Self { handle }
        }
    }
    const _: fn() = || {
        fn assert_impl_all<T: ?Sized + ::core::marker::Send>() {}
        assert_impl_all::<UniFFICallbackHandlerWpLocalizedError>();
    };
    impl WpLocalizedError for UniFFICallbackHandlerWpLocalizedError {
        fn localized_error_message(&self, locale_id: String) -> String {
            let vtable = UNIFFI_TRAIT_CELL_WPLOCALIZEDERROR.get();
            let mut uniffi_call_status: ::uniffi::RustCallStatus = ::std::default::Default::default();
            let mut uniffi_return_value: <String as ::uniffi::LiftReturn<
                crate::UniFfiTag,
            >>::ReturnType = ::uniffi::FfiDefault::ffi_default();
            (vtable
                .localized_error_message)(
                self.handle,
                <String as ::uniffi::Lower<crate::UniFfiTag>>::lower(locale_id),
                &mut uniffi_return_value,
                &mut uniffi_call_status,
            );
            <String as ::uniffi::LiftReturn<
                crate::UniFfiTag,
            >>::lift_foreign_return(uniffi_return_value, uniffi_call_status)
        }
    }
    impl ::std::ops::Drop for UniFFICallbackHandlerWpLocalizedError {
        fn drop(&mut self) {
            let vtable = UNIFFI_TRAIT_CELL_WPLOCALIZEDERROR.get();
            (vtable.uniffi_free)(self.handle);
        }
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_wplocalizederror_localized_error_message(
        uniffi_self_lowered: <::std::sync::Arc<
            dyn WpLocalizedError,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        locale_id: <String as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <String as ::uniffi::LowerReturn<crate::UniFfiTag>>::ReturnType {
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match {
                let boxed_foreign_arc = unsafe {
                    ::std::boxed::Box::from_raw(
                        uniffi_self_lowered
                            as *mut ::std::sync::Arc<dyn WpLocalizedError>,
                    )
                };
                ::std::result::Result::Ok(*boxed_foreign_arc)
            } {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <String as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(locale_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("locale_id", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                let result = match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .localized_error_message(uniffi_args.1);
                        <String as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <String as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                };
                result
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_WPLOCALIZEDERROR_LOCALIZED_ERROR_MESSAGE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::TRAIT_METHOD,
        )
        .concat_str("wp_api")
        .concat_str("WpLocalizedError")
        .concat_u32(0u32)
        .concat_str("localized_error_message")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("locale_id")
        .concat(<String as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat(<String as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_WPLOCALIZEDERROR_LOCALIZED_ERROR_MESSAGE: [u8; UNIFFI_META_CONST_WP_API_METHOD_WPLOCALIZEDERROR_LOCALIZED_ERROR_MESSAGE
        .size] = UNIFFI_META_CONST_WP_API_METHOD_WPLOCALIZEDERROR_LOCALIZED_ERROR_MESSAGE
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_wplocalizederror_localized_error_message() -> u16 {
        const CHECKSUM: u16 = UNIFFI_META_CONST_WP_API_METHOD_WPLOCALIZEDERROR_LOCALIZED_ERROR_MESSAGE
            .checksum();
        CHECKSUM
    }
    const _: fn() = || {
        fn assert_impl_all<T: ?Sized + ::core::marker::Sync + ::core::marker::Send>() {}
        assert_impl_all::<dyn WpLocalizedError>();
    };
    unsafe impl<T> ::uniffi::FfiConverterArc<T> for dyn WpLocalizedError {
        type FfiType = *const ::std::os::raw::c_void;
        fn lower(obj: ::std::sync::Arc<Self>) -> Self::FfiType {
            ::std::boxed::Box::into_raw(::std::boxed::Box::new(obj))
                as *const ::std::os::raw::c_void
        }
        fn try_lift(
            v: Self::FfiType,
        ) -> ::uniffi::deps::anyhow::Result<::std::sync::Arc<Self>> {
            ::std::result::Result::Ok(
                ::std::sync::Arc::new(
                    <UniFFICallbackHandlerWpLocalizedError>::new(v as u64),
                ),
            )
        }
        fn write(obj: ::std::sync::Arc<Self>, buf: &mut ::std::vec::Vec<u8>) {
            #[allow(unknown_lints, eq_op)]
            const _: [(); 0
                - !{
                    const ASSERT: bool = ::std::mem::size_of::<
                        *const ::std::ffi::c_void,
                    >() <= 8;
                    ASSERT
                } as usize] = [];
            ::uniffi::deps::bytes::BufMut::put_u64(
                buf,
                <::std::sync::Arc<Self> as ::uniffi::Lower<crate::UniFfiTag>>::lower(obj)
                    as ::std::primitive::u64,
            );
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi::Result<::std::sync::Arc<Self>> {
            #[allow(unknown_lints, eq_op)]
            const _: [(); 0
                - !{
                    const ASSERT: bool = ::std::mem::size_of::<
                        *const ::std::ffi::c_void,
                    >() <= 8;
                    ASSERT
                } as usize] = [];
            ::uniffi::check_remaining(buf, 8)?;
            <::std::sync::Arc<
                Self,
            > as ::uniffi::Lift<
                crate::UniFfiTag,
            >>::try_lift(::uniffi::deps::bytes::Buf::get_u64(buf) as Self::FfiType)
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_CALLBACK_TRAIT_INTERFACE,
            )
            .concat_str("wp_api")
            .concat_str("WpLocalizedError");
    }
    unsafe impl<T> ::uniffi::LiftRef<T> for dyn WpLocalizedError {
        type LiftType = ::std::sync::Arc<dyn WpLocalizedError>;
    }
    pub enum FooError {
        #[error("{}", Messages::default().foo_error_bar())]
        Bar,
        #[error("{}", Messages::default().foo_error_baz(value))]
        Baz { value: String },
        #[error("{}", Messages::default().foo_error_bazzz(value1, value2))]
        Bazzz { value1: String, value2: String },
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for FooError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                FooError::Bar => ::core::fmt::Formatter::write_str(f, "Bar"),
                FooError::Baz { value: __self_0 } => {
                    ::core::fmt::Formatter::debug_struct_field1_finish(
                        f,
                        "Baz",
                        "value",
                        &__self_0,
                    )
                }
                FooError::Bazzz { value1: __self_0, value2: __self_1 } => {
                    ::core::fmt::Formatter::debug_struct_field2_finish(
                        f,
                        "Bazzz",
                        "value1",
                        __self_0,
                        "value2",
                        &__self_1,
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for FooError {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for FooError {
        #[inline]
        fn eq(&self, other: &FooError) -> bool {
            let __self_discr = ::core::intrinsics::discriminant_value(self);
            let __arg1_discr = ::core::intrinsics::discriminant_value(other);
            __self_discr == __arg1_discr
                && match (self, other) {
                    (
                        FooError::Baz { value: __self_0 },
                        FooError::Baz { value: __arg1_0 },
                    ) => __self_0 == __arg1_0,
                    (
                        FooError::Bazzz { value1: __self_0, value2: __self_1 },
                        FooError::Bazzz { value1: __arg1_0, value2: __arg1_1 },
                    ) => __self_0 == __arg1_0 && __self_1 == __arg1_1,
                    _ => true,
                }
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for FooError {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<String>;
        }
    }
    #[allow(unused_qualifications)]
    #[automatically_derived]
    impl ::thiserror::__private::Error for FooError {}
    #[allow(unused_qualifications)]
    #[automatically_derived]
    impl ::core::fmt::Display for FooError {
        fn fmt(&self, __formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            #[allow(unused_variables, deprecated, clippy::used_underscore_binding)]
            match self {
                FooError::Bar {} => {
                    __formatter
                        .write_fmt(
                            format_args!("{0}", Messages::default().foo_error_bar()),
                        )
                }
                FooError::Baz { value } => {
                    __formatter
                        .write_fmt(
                            format_args!("{0}", Messages::default().foo_error_baz(value)),
                        )
                }
                FooError::Bazzz { value1, value2 } => {
                    __formatter
                        .write_fmt(
                            format_args!(
                                "{0}",
                                Messages::default().foo_error_bazzz(value1, value2),
                            ),
                        )
                }
            }
        }
    }
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT> for FooError {
        type FfiType = ::uniffi_core::RustBuffer;
        fn lower(v: Self) -> ::uniffi_core::RustBuffer {
            let mut buf = ::std::vec::Vec::new();
            <Self as ::uniffi_core::FfiConverter<crate::UniFfiTag>>::write(v, &mut buf);
            ::uniffi_core::RustBuffer::from_vec(buf)
        }
        fn try_lift(buf: ::uniffi_core::RustBuffer) -> ::uniffi_core::Result<Self> {
            let vec = buf.destroy_into_vec();
            let mut buf = vec.as_slice();
            let value = <Self as ::uniffi_core::FfiConverter<
                crate::UniFfiTag,
            >>::try_read(&mut buf)?;
            match ::uniffi_core::deps::bytes::Buf::remaining(&buf) {
                0 => ::std::result::Result::Ok(value),
                n => {
                    return ::anyhow::__private::Err({
                        let error = ::anyhow::__private::format_err(
                            format_args!(
                                "junk data left in buffer after lifting (count: {0})",
                                n,
                            ),
                        );
                        error
                    });
                }
            }
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            match obj {
                Self::Bar {} => {
                    ::uniffi::deps::bytes::BufMut::put_i32(buf, 1);
                }
                Self::Baz { value } => {
                    ::uniffi::deps::bytes::BufMut::put_i32(buf, 2);
                    <String as ::uniffi::Lower<crate::UniFfiTag>>::write(value, buf);
                }
                Self::Bazzz { value1, value2 } => {
                    ::uniffi::deps::bytes::BufMut::put_i32(buf, 3);
                    <String as ::uniffi::Lower<crate::UniFfiTag>>::write(value1, buf);
                    <String as ::uniffi::Lower<crate::UniFfiTag>>::write(value2, buf);
                }
            }
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::uniffi::check_remaining(buf, 4)?;
            ::std::result::Result::Ok(
                match ::uniffi::deps::bytes::Buf::get_i32(buf) {
                    1 => Self::Bar {},
                    2 => {
                        Self::Baz {
                            value: <String as ::uniffi::Lift<
                                crate::UniFfiTag,
                            >>::try_read(buf)?,
                        }
                    }
                    3 => {
                        Self::Bazzz {
                            value1: <String as ::uniffi::Lift<
                                crate::UniFfiTag,
                            >>::try_read(buf)?,
                            value2: <String as ::uniffi::Lift<
                                crate::UniFfiTag,
                            >>::try_read(buf)?,
                        }
                    }
                    v => {
                        return ::anyhow::__private::Err(
                            ::anyhow::Error::msg(
                                ::alloc::__export::must_use({
                                    let res = ::alloc::fmt::format(
                                        format_args!("Invalid FooError enum value: {0}", v),
                                    );
                                    res
                                }),
                            ),
                        );
                    }
                },
            )
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_ENUM,
            )
            .concat_str("wp_api")
            .concat_str("FooError");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT> for FooError {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT> for FooError {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT> for FooError {
        type ReturnType = <Self as ::uniffi_core::Lower<UT>>::FfiType;
        fn lower_return(
            v: Self,
        ) -> ::uniffi_core::deps::anyhow::Result<
            Self::ReturnType,
            ::uniffi_core::RustCallError,
        > {
            ::std::result::Result::Ok(<Self as ::uniffi_core::Lower<UT>>::lower(v))
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerError<UT> for FooError {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT> for FooError {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT> for FooError {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT> for FooError {
        fn try_convert_unexpected_callback_error(
            e: ::uniffi_core::UnexpectedUniFFICallbackError,
        ) -> ::uniffi_core::deps::anyhow::Result<Self> {
            {
                pub trait GetConverterGeneric {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterGeneric;
                }
                impl<T> GetConverterGeneric for &T {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterGeneric {
                        ::uniffi_core::UnexpectedUniFFICallbackErrorConverterGeneric
                    }
                }
                #[allow(dead_code)]
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<T: ::std::convert::Into<FooError>> GetConverterSpecialized for T {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized {
                        ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized
                    }
                }
                (&e).get_converter().try_convert_unexpected_callback_error(e)
            }
        }
    }
    impl<UT> ::uniffi_core::TypeId<UT> for FooError {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_ERROR_FOOERROR: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::ENUM,
        )
        .concat_str("wp_api")
        .concat_str("FooError")
        .concat_value(1u8)
        .concat_bool(false)
        .concat_value(3u8)
        .concat_str("Bar")
        .concat_bool(false)
        .concat_value(0u8)
        .concat_long_str("")
        .concat_str("Baz")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("value")
        .concat(<String as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("")
        .concat_str("Bazzz")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("value1")
        .concat(<String as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("value2")
        .concat(<String as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("")
        .concat_bool(false)
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_ERROR_FOOERROR: [u8; UNIFFI_META_CONST_WP_API_ERROR_FOOERROR
        .size] = UNIFFI_META_CONST_WP_API_ERROR_FOOERROR.into_array();
    impl WpLocalizedError for FooError {
        fn localized_error_message(&self, locale_id: String) -> String {
            let messages = Messages::get(&locale_id).unwrap_or_default();
            match self {
                Self::Bar => messages.foo_bar().to_string(),
                Self::Baz { value } => messages.foo_error_baz(value).to_string(),
                Self::Bazzz { value1, value2 } => {
                    messages.foo_error_bazzz(value1, value2).to_string()
                }
            }
        }
    }
}
