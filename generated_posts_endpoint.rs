pub(crate) mod posts_endpoint {
    use crate::{
        posts::{
            PostId, PostListParams, PostUpdateParams, PostWithEditContext,
            SparsePostFieldWithEditContext, SparsePostFieldWithEmbedContext,
            SparsePostFieldWithViewContext,
        },
        SparseField,
    };
    use wp_derive_request_builder::WpDerivedRequest;
    use super::{DerivedRequest, Namespace};
    enum PostsRequest {
        #[contextual_get(
            url = "/posts",
            params = &PostListParams,
            output = Vec<crate::posts::SparsePost>,
            filter_by = crate::posts::SparsePostField
        )]
        List,
        #[contextual_get(
            url = "/posts/<post_id>",
            params = &crate::posts::PostRetrieveParams,
            output = crate::posts::SparsePost,
            filter_by = crate::posts::SparsePostField
        )]
        Retrieve,
        #[post(
            url = "/posts",
            params = &crate::posts::PostCreateParams,
            output = crate::posts::PostWithEditContext
        )]
        Create,
        #[delete(url = "/posts/<post_id>", output = crate::posts::PostDeleteResponse)]
        Delete,
        #[delete(url = "/posts/<post_id>", output = crate::posts::PostWithEditContext)]
        Trash,
        #[post(
            url = "/posts/<post_id>",
            params = &PostUpdateParams,
            output = PostWithEditContext
        )]
        Update,
    }
    pub struct PostsRequestEndpoint {
        api_base_url: std::sync::Arc<crate::request::endpoint::ApiBaseUrl>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestEndpoint {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "PostsRequestEndpoint",
                "api_base_url",
                &&self.api_base_url,
            )
        }
    }
    impl PostsRequestEndpoint {
        pub fn new(
            api_base_url: std::sync::Arc<crate::request::endpoint::ApiBaseUrl>,
        ) -> Self {
            Self { api_base_url }
        }
        pub fn list_with_edit_context(
            &self,
            params: &PostListParams,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Edit.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::List,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn filter_list_with_edit_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithEditContext],
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Edit.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::List,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            use crate::SparseField;
            url.query_pairs_mut()
                .append_pair(
                    "_fields",
                    fields
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                        .as_str(),
                );
            url.into()
        }
        pub fn list_with_embed_context(
            &self,
            params: &PostListParams,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Embed.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::List,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn filter_list_with_embed_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithEmbedContext],
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Embed.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::List,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            use crate::SparseField;
            url.query_pairs_mut()
                .append_pair(
                    "_fields",
                    fields
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                        .as_str(),
                );
            url.into()
        }
        pub fn list_with_view_context(
            &self,
            params: &PostListParams,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::View.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::List,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn filter_list_with_view_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithViewContext],
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::View.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::List,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            use crate::SparseField;
            url.query_pairs_mut()
                .append_pair(
                    "_fields",
                    fields
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                        .as_str(),
                );
            url.into()
        }
        pub fn retrieve_with_edit_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Edit.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Retrieve,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn filter_retrieve_with_edit_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithEditContext],
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Edit.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Retrieve,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            use crate::SparseField;
            url.query_pairs_mut()
                .append_pair(
                    "_fields",
                    fields
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                        .as_str(),
                );
            url.into()
        }
        pub fn retrieve_with_embed_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Embed.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Retrieve,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn filter_retrieve_with_embed_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithEmbedContext],
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::Embed.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Retrieve,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            use crate::SparseField;
            url.query_pairs_mut()
                .append_pair(
                    "_fields",
                    fields
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                        .as_str(),
                );
            url.into()
        }
        pub fn retrieve_with_view_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::View.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Retrieve,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn filter_retrieve_with_view_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithViewContext],
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            url.query_pairs_mut()
                .append_pair("context", crate::WpContext::View.as_str());
            use crate::url_query::AppendUrlQueryPairs;
            params.append_query_pairs(&mut url.query_pairs_mut());
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Retrieve,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            use crate::SparseField;
            url.query_pairs_mut()
                .append_pair(
                    "_fields",
                    fields
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<&str>>()
                        .join(",")
                        .as_str(),
                );
            url.into()
        }
        pub fn create(&self) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                ]);
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Create,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn delete(
            &self,
            post_id: &PostId,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Delete,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn trash(
            &self,
            post_id: &PostId,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Trash,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
        pub fn update(
            &self,
            post_id: &PostId,
        ) -> crate::request::endpoint::ApiEndpointUrl {
            let mut url = self
                .api_base_url
                .by_extending_and_splitting_by_forward_slash([
                    PostsRequest::namespace().as_str(),
                    "posts",
                    &post_id.to_string(),
                ]);
            let additional_query_pairs = PostsRequest::additional_query_pairs(
                &PostsRequest::Update,
            );
            if !additional_query_pairs.is_empty() {
                url.query_pairs_mut().extend_pairs(additional_query_pairs);
            }
            url.into()
        }
    }
    pub struct PostsRequestBuilder {
        endpoint: PostsRequestEndpoint,
        inner: crate::request::InnerRequestBuilder,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestBuilder {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestBuilder",
                "endpoint",
                &self.endpoint,
                "inner",
                &&self.inner,
            )
        }
    }
    #[doc(hidden)]
    #[no_mangle]
    pub unsafe extern "C" fn uniffi_wp_api_fn_clone_postsrequestbuilder(
        ptr: *const ::std::ffi::c_void,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> *const ::std::ffi::c_void {
        ::uniffi::rust_call(
            call_status,
            || {
                unsafe { ::std::sync::Arc::increment_strong_count(ptr) };
                ::std::result::Result::Ok(ptr)
            },
        )
    }
    #[doc(hidden)]
    #[no_mangle]
    pub unsafe extern "C" fn uniffi_wp_api_fn_free_postsrequestbuilder(
        ptr: *const ::std::ffi::c_void,
        call_status: &mut ::uniffi::RustCallStatus,
    ) {
        ::uniffi::rust_call(
            call_status,
            || {
                if !!ptr.is_null() {
                    ::core::panicking::panic("assertion failed: !ptr.is_null()")
                }
                let ptr = ptr.cast::<PostsRequestBuilder>();
                unsafe {
                    ::std::sync::Arc::decrement_strong_count(ptr);
                }
                ::std::result::Result::Ok(())
            },
        );
    }
    const _: fn() = || {
        fn assert_impl_all<T: ?Sized + ::core::marker::Sync + ::core::marker::Send>() {}
        assert_impl_all::<PostsRequestBuilder>();
    };
    #[doc(hidden)]
    #[automatically_derived]
    /// Support for passing reference-counted shared objects via the FFI.
    ///
    /// To avoid dealing with complex lifetime semantics over the FFI, any data passed
    /// by reference must be encapsulated in an `Arc`, and must be safe to share
    /// across threads.
    unsafe impl<UT> ::uniffi::FfiConverterArc<UT> for PostsRequestBuilder {
        type FfiType = *const ::std::os::raw::c_void;
        /// When lowering, we have an owned `Arc` and we transfer that ownership
        /// to the foreign-language code, "leaking" it out of Rust's ownership system
        /// as a raw pointer. This works safely because we have unique ownership of `self`.
        /// The foreign-language code is responsible for freeing this by calling the
        /// `ffi_object_free` FFI function provided by the corresponding UniFFI type.
        ///
        /// Safety: when freeing the resulting pointer, the foreign-language code must
        /// call the destructor function specific to the type `T`. Calling the destructor
        /// function for other types may lead to undefined behaviour.
        fn lower(obj: ::std::sync::Arc<Self>) -> Self::FfiType {
            ::std::sync::Arc::into_raw(obj) as Self::FfiType
        }
        /// When lifting, we receive an owned `Arc` that the foreign language code cloned.
        fn try_lift(v: Self::FfiType) -> ::uniffi::Result<::std::sync::Arc<Self>> {
            let v = v as *const PostsRequestBuilder;
            ::std::result::Result::Ok(unsafe { ::std::sync::Arc::<Self>::from_raw(v) })
        }
        /// When writing as a field of a complex structure, make a clone and transfer ownership
        /// of it to the foreign-language code by writing its pointer into the buffer.
        /// The foreign-language code is responsible for freeing this by calling the
        /// `ffi_object_free` FFI function provided by the corresponding UniFFI type.
        ///
        /// Safety: when freeing the resulting pointer, the foreign-language code must
        /// call the destructor function specific to the type `T`. Calling the destructor
        /// function for other types may lead to undefined behaviour.
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
        /// When reading as a field of a complex structure, we receive a "borrow" of the `Arc`
        /// that is owned by the foreign-language code, and make a clone for our own use.
        ///
        /// Safety: the buffer must contain a pointer previously obtained by calling
        /// the `lower()` or `write()` method of this impl.
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
                ::uniffi::metadata::codes::TYPE_INTERFACE,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestBuilder");
    }
    unsafe impl<UT> ::uniffi::LowerReturn<UT> for PostsRequestBuilder {
        type ReturnType = <::std::sync::Arc<
            Self,
        > as ::uniffi::LowerReturn<crate::UniFfiTag>>::ReturnType;
        fn lower_return(
            obj: Self,
        ) -> ::std::result::Result<Self::ReturnType, ::uniffi::RustCallError> {
            <::std::sync::Arc<
                Self,
            > as ::uniffi::LowerReturn<
                crate::UniFfiTag,
            >>::lower_return(::std::sync::Arc::new(obj))
        }
    }
    unsafe impl<UT> ::uniffi::LowerError<UT> for PostsRequestBuilder {
        fn lower_error(obj: Self) -> ::uniffi::RustBuffer {
            <::std::sync::Arc<
                Self,
            > as ::uniffi::LowerError<
                crate::UniFfiTag,
            >>::lower_error(::std::sync::Arc::new(obj))
        }
    }
    unsafe impl<UT> ::uniffi::LiftRef<UT> for PostsRequestBuilder {
        type LiftType = ::std::sync::Arc<Self>;
    }
    impl<UT> ::uniffi::TypeId<UT> for PostsRequestBuilder {
        const TYPE_ID_META: ::uniffi::MetadataBuffer = <::std::sync::Arc<
            Self,
        > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_INTERFACE_POSTSREQUESTBUILDER: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::INTERFACE,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_INTERFACE_POSTSREQUESTBUILDER: [u8; UNIFFI_META_CONST_WP_API_INTERFACE_POSTSREQUESTBUILDER
        .size] = UNIFFI_META_CONST_WP_API_INTERFACE_POSTSREQUESTBUILDER.into_array();
    impl PostsRequestBuilder {
        pub fn new(
            api_base_url: std::sync::Arc<crate::request::endpoint::ApiBaseUrl>,
            authentication: crate::WpAuthentication,
        ) -> Self {
            Self {
                endpoint: PostsRequestEndpoint::new(api_base_url),
                inner: crate::request::InnerRequestBuilder::new(authentication),
            }
        }
    }
    impl PostsRequestBuilder {
        pub fn list_with_edit_context(
            &self,
            params: &PostListParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.list_with_edit_context(params);
            self.inner.get(url)
        }
        pub fn filter_list_with_edit_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithEditContext],
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.filter_list_with_edit_context(params, fields);
            self.inner.get(url)
        }
        pub fn list_with_embed_context(
            &self,
            params: &PostListParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.list_with_embed_context(params);
            self.inner.get(url)
        }
        pub fn filter_list_with_embed_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithEmbedContext],
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.filter_list_with_embed_context(params, fields);
            self.inner.get(url)
        }
        pub fn list_with_view_context(
            &self,
            params: &PostListParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.list_with_view_context(params);
            self.inner.get(url)
        }
        pub fn filter_list_with_view_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithViewContext],
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.filter_list_with_view_context(params, fields);
            self.inner.get(url)
        }
        pub fn retrieve_with_edit_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.retrieve_with_edit_context(post_id, params);
            self.inner.get(url)
        }
        pub fn filter_retrieve_with_edit_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithEditContext],
        ) -> crate::request::WpNetworkRequest {
            let url = self
                .endpoint
                .filter_retrieve_with_edit_context(post_id, params, fields);
            self.inner.get(url)
        }
        pub fn retrieve_with_embed_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.retrieve_with_embed_context(post_id, params);
            self.inner.get(url)
        }
        pub fn filter_retrieve_with_embed_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithEmbedContext],
        ) -> crate::request::WpNetworkRequest {
            let url = self
                .endpoint
                .filter_retrieve_with_embed_context(post_id, params, fields);
            self.inner.get(url)
        }
        pub fn retrieve_with_view_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.retrieve_with_view_context(post_id, params);
            self.inner.get(url)
        }
        pub fn filter_retrieve_with_view_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithViewContext],
        ) -> crate::request::WpNetworkRequest {
            let url = self
                .endpoint
                .filter_retrieve_with_view_context(post_id, params, fields);
            self.inner.get(url)
        }
        pub fn create(
            &self,
            params: &crate::posts::PostCreateParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.create();
            self.inner.post(url, params)
        }
        pub fn delete(&self, post_id: &PostId) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.delete(post_id);
            self.inner.delete(url)
        }
        pub fn trash(&self, post_id: &PostId) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.trash(post_id);
            self.inner.delete(url)
        }
        pub fn update(
            &self,
            post_id: &PostId,
            params: &PostUpdateParams,
        ) -> crate::request::WpNetworkRequest {
            let url = self.endpoint.update(post_id);
            self.inner.post(url, params)
        }
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_list_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("list_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .list_with_edit_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("list_with_edit_context")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_list_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_filter_list_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_list_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_list_with_edit_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                                <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEditContext],
                                >>::borrow(&uniffi_args.2),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("filter_list_with_edit_context")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_filter_list_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_list_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("list_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .list_with_embed_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("list_with_embed_context")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_list_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_filter_list_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_list_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_list_with_embed_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                                <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEmbedContext],
                                >>::borrow(&uniffi_args.2),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("filter_list_with_embed_context")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_filter_list_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_list_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("list_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .list_with_view_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("list_with_view_context")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_list_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_LIST_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_filter_list_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_list_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_list_with_view_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                                <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithViewContext],
                                >>::borrow(&uniffi_args.2),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("filter_list_with_view_context")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_filter_list_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_LIST_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_retrieve_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("retrieve_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .retrieve_with_edit_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("retrieve_with_edit_context")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_retrieve_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_filter_retrieve_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_retrieve_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_retrieve_with_edit_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                                <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEditContext],
                                >>::borrow(&uniffi_args.3),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("filter_retrieve_with_edit_context")
        .concat_bool(false)
        .concat_value(3u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_filter_retrieve_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_retrieve_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("retrieve_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .retrieve_with_embed_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("retrieve_with_embed_context")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_retrieve_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_filter_retrieve_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_retrieve_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_retrieve_with_embed_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                                <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEmbedContext],
                                >>::borrow(&uniffi_args.3),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("filter_retrieve_with_embed_context")
        .concat_bool(false)
        .concat_value(3u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_filter_retrieve_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_retrieve_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("retrieve_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .retrieve_with_view_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("retrieve_with_view_context")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_retrieve_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_RETRIEVE_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_filter_retrieve_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_retrieve_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_retrieve_with_view_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                                <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithViewContext],
                                >>::borrow(&uniffi_args.3),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("filter_retrieve_with_view_context")
        .concat_bool(false)
        .concat_value(3u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_filter_retrieve_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_FILTER_RETRIEVE_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_create(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("create"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .create(
                                <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostCreateParams,
                                >>::borrow(&uniffi_args.1),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_CREATE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("create")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_CREATE: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_CREATE
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_CREATE.into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_create() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_CREATE.checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_delete(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("delete"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .delete(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_DELETE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("delete")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_DELETE: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_DELETE
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_DELETE.into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_delete() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_DELETE.checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_trash(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("trash"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .trash(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_TRASH: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("trash")
        .concat_bool(false)
        .concat_value(1u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_TRASH: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_TRASH
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_TRASH.into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_trash() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_TRASH.checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestbuilder_update(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestBuilder,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostUpdateParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
        crate::UniFfiTag,
    >>::ReturnType {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("update"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lift_args = move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestBuilder,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<PostUpdateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        ));
        ::uniffi::rust_call(
            call_status,
            || {
                match uniffi_lift_args() {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .update(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<PostUpdateParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostUpdateParams,
                                >>::borrow(&uniffi_args.2),
                            );
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::lower_return(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        <crate::request::WpNetworkRequest as ::uniffi::LowerReturn<
                            crate::UniFfiTag,
                        >>::handle_failed_lift(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_UPDATE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestBuilder")
        .concat_str("update")
        .concat_bool(false)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<PostUpdateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <crate::request::WpNetworkRequest as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTBUILDER_UPDATE: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_UPDATE
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_UPDATE.into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestbuilder_update() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTBUILDER_UPDATE.checksum()
    }
    #[serde(transparent)]
    pub struct PostsRequestListWithEditContextResponse {
        pub data: Vec<crate::posts::PostWithEditContext>,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestListWithEditContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestListWithEditContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestListWithEditContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for PostsRequestListWithEditContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestListWithEditContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestListWithEditContextResponse {
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
            <Vec<
                crate::posts::PostWithEditContext,
            > as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <Vec<
                    crate::posts::PostWithEditContext,
                > as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestListWithEditContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestListWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT> for PostsRequestListWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestListWithEditContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestListWithEditContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestListWithEditContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestListWithEditContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestListWithEditContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestListWithEditContextResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestListWithEditContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHEDITCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestListWithEditContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <Vec<
                crate::posts::PostWithEditContext,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTLISTWITHEDITCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHEDITCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHEDITCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestFilterListWithEditContextResponse {
        pub data: Vec<crate::posts::SparsePostWithEditContext>,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestFilterListWithEditContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestFilterListWithEditContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestFilterListWithEditContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestFilterListWithEditContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestFilterListWithEditContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestFilterListWithEditContextResponse {
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
            <Vec<
                crate::posts::SparsePostWithEditContext,
            > as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <Vec<
                    crate::posts::SparsePostWithEditContext,
                > as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestFilterListWithEditContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestFilterListWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestFilterListWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestFilterListWithEditContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestFilterListWithEditContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestFilterListWithEditContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestFilterListWithEditContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestFilterListWithEditContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<
                            PostsRequestFilterListWithEditContextResponse,
                        >,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT>
    for PostsRequestFilterListWithEditContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEDITCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestFilterListWithEditContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <Vec<
                crate::posts::SparsePostWithEditContext,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEDITCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEDITCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEDITCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestListWithEmbedContextResponse {
        pub data: Vec<crate::posts::PostWithEmbedContext>,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestListWithEmbedContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestListWithEmbedContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestListWithEmbedContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for PostsRequestListWithEmbedContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestListWithEmbedContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestListWithEmbedContextResponse {
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
            <Vec<
                crate::posts::PostWithEmbedContext,
            > as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <Vec<
                    crate::posts::PostWithEmbedContext,
                > as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestListWithEmbedContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestListWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestListWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestListWithEmbedContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestListWithEmbedContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestListWithEmbedContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestListWithEmbedContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestListWithEmbedContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestListWithEmbedContextResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestListWithEmbedContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHEMBEDCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestListWithEmbedContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <Vec<
                crate::posts::PostWithEmbedContext,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTLISTWITHEMBEDCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHEMBEDCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHEMBEDCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestFilterListWithEmbedContextResponse {
        pub data: Vec<crate::posts::SparsePostWithEmbedContext>,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestFilterListWithEmbedContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestFilterListWithEmbedContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestFilterListWithEmbedContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestFilterListWithEmbedContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestFilterListWithEmbedContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
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
            <Vec<
                crate::posts::SparsePostWithEmbedContext,
            > as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <Vec<
                    crate::posts::SparsePostWithEmbedContext,
                > as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestFilterListWithEmbedContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<
                            PostsRequestFilterListWithEmbedContextResponse,
                        >,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT>
    for PostsRequestFilterListWithEmbedContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEMBEDCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestFilterListWithEmbedContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <Vec<
                crate::posts::SparsePostWithEmbedContext,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEMBEDCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEMBEDCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHEMBEDCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestListWithViewContextResponse {
        pub data: Vec<crate::posts::PostWithViewContext>,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestListWithViewContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestListWithViewContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestListWithViewContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for PostsRequestListWithViewContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestListWithViewContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestListWithViewContextResponse {
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
            <Vec<
                crate::posts::PostWithViewContext,
            > as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <Vec<
                    crate::posts::PostWithViewContext,
                > as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestListWithViewContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestListWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT> for PostsRequestListWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestListWithViewContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestListWithViewContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestListWithViewContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestListWithViewContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestListWithViewContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestListWithViewContextResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestListWithViewContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHVIEWCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestListWithViewContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <Vec<
                crate::posts::PostWithViewContext,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTLISTWITHVIEWCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHVIEWCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTLISTWITHVIEWCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestFilterListWithViewContextResponse {
        pub data: Vec<crate::posts::SparsePostWithViewContext>,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestFilterListWithViewContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestFilterListWithViewContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestFilterListWithViewContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestFilterListWithViewContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestFilterListWithViewContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestFilterListWithViewContextResponse {
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
            <Vec<
                crate::posts::SparsePostWithViewContext,
            > as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <Vec<
                    crate::posts::SparsePostWithViewContext,
                > as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestFilterListWithViewContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestFilterListWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestFilterListWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestFilterListWithViewContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestFilterListWithViewContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestFilterListWithViewContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestFilterListWithViewContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestFilterListWithViewContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<
                            PostsRequestFilterListWithViewContextResponse,
                        >,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT>
    for PostsRequestFilterListWithViewContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHVIEWCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestFilterListWithViewContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <Vec<
                crate::posts::SparsePostWithViewContext,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHVIEWCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHVIEWCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERLISTWITHVIEWCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestRetrieveWithEditContextResponse {
        pub data: crate::posts::PostWithEditContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestRetrieveWithEditContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestRetrieveWithEditContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestRetrieveWithEditContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestRetrieveWithEditContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestRetrieveWithEditContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestRetrieveWithEditContextResponse {
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
            <crate::posts::PostWithEditContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::PostWithEditContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestRetrieveWithEditContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestRetrieveWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestRetrieveWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestRetrieveWithEditContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestRetrieveWithEditContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestRetrieveWithEditContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestRetrieveWithEditContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestRetrieveWithEditContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestRetrieveWithEditContextResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestRetrieveWithEditContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEDITCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestRetrieveWithEditContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::PostWithEditContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEDITCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEDITCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEDITCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestFilterRetrieveWithEditContextResponse {
        pub data: crate::posts::SparsePostWithEditContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestFilterRetrieveWithEditContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestFilterRetrieveWithEditContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestFilterRetrieveWithEditContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestFilterRetrieveWithEditContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestFilterRetrieveWithEditContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
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
            <crate::posts::SparsePostWithEditContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::SparsePostWithEditContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestFilterRetrieveWithEditContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<
                            PostsRequestFilterRetrieveWithEditContextResponse,
                        >,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT>
    for PostsRequestFilterRetrieveWithEditContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEDITCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestFilterRetrieveWithEditContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::SparsePostWithEditContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEDITCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEDITCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEDITCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestRetrieveWithEmbedContextResponse {
        pub data: crate::posts::PostWithEmbedContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestRetrieveWithEmbedContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestRetrieveWithEmbedContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestRetrieveWithEmbedContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestRetrieveWithEmbedContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestRetrieveWithEmbedContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
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
            <crate::posts::PostWithEmbedContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::PostWithEmbedContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestRetrieveWithEmbedContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestRetrieveWithEmbedContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestRetrieveWithEmbedContextResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestRetrieveWithEmbedContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEMBEDCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestRetrieveWithEmbedContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::PostWithEmbedContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEMBEDCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEMBEDCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHEMBEDCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestFilterRetrieveWithEmbedContextResponse {
        pub data: crate::posts::SparsePostWithEmbedContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestFilterRetrieveWithEmbedContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestFilterRetrieveWithEmbedContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestFilterRetrieveWithEmbedContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestFilterRetrieveWithEmbedContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestFilterRetrieveWithEmbedContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
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
            <crate::posts::SparsePostWithEmbedContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::SparsePostWithEmbedContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestFilterRetrieveWithEmbedContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<
                            PostsRequestFilterRetrieveWithEmbedContextResponse,
                        >,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT>
    for PostsRequestFilterRetrieveWithEmbedContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEMBEDCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestFilterRetrieveWithEmbedContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::SparsePostWithEmbedContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEMBEDCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEMBEDCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHEMBEDCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestRetrieveWithViewContextResponse {
        pub data: crate::posts::PostWithViewContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestRetrieveWithViewContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestRetrieveWithViewContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestRetrieveWithViewContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestRetrieveWithViewContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestRetrieveWithViewContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestRetrieveWithViewContextResponse {
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
            <crate::posts::PostWithViewContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::PostWithViewContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestRetrieveWithViewContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestRetrieveWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestRetrieveWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestRetrieveWithViewContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestRetrieveWithViewContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestRetrieveWithViewContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestRetrieveWithViewContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestRetrieveWithViewContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestRetrieveWithViewContextResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestRetrieveWithViewContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHVIEWCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestRetrieveWithViewContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::PostWithViewContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHVIEWCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHVIEWCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTRETRIEVEWITHVIEWCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestFilterRetrieveWithViewContextResponse {
        pub data: crate::posts::SparsePostWithViewContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestFilterRetrieveWithViewContextResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestFilterRetrieveWithViewContextResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestFilterRetrieveWithViewContextResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de>
        for PostsRequestFilterRetrieveWithViewContextResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestFilterRetrieveWithViewContextResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
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
            <crate::posts::SparsePostWithViewContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::SparsePostWithViewContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestFilterRetrieveWithViewContextResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<
                            PostsRequestFilterRetrieveWithViewContextResponse,
                        >,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT>
    for PostsRequestFilterRetrieveWithViewContextResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHVIEWCONTEXTRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestFilterRetrieveWithViewContextResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::SparsePostWithViewContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHVIEWCONTEXTRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHVIEWCONTEXTRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTFILTERRETRIEVEWITHVIEWCONTEXTRESPONSE
        .into_array();
    #[serde(transparent)]
    pub struct PostsRequestCreateResponse {
        pub data: crate::posts::PostWithEditContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestCreateResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestCreateResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestCreateResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for PostsRequestCreateResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestCreateResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT> for PostsRequestCreateResponse {
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
            <crate::posts::PostWithEditContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::PostWithEditContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestCreateResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT> for PostsRequestCreateResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT> for PostsRequestCreateResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT> for PostsRequestCreateResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT> for PostsRequestCreateResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT> for PostsRequestCreateResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT> for PostsRequestCreateResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT> for PostsRequestCreateResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestCreateResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestCreateResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTCREATERESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestCreateResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::PostWithEditContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTCREATERESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTCREATERESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTCREATERESPONSE.into_array();
    #[serde(transparent)]
    pub struct PostsRequestDeleteResponse {
        pub data: crate::posts::PostDeleteResponse,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestDeleteResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestDeleteResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestDeleteResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for PostsRequestDeleteResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestDeleteResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT> for PostsRequestDeleteResponse {
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
            <crate::posts::PostDeleteResponse as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::PostDeleteResponse as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestDeleteResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT> for PostsRequestDeleteResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT> for PostsRequestDeleteResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT> for PostsRequestDeleteResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT> for PostsRequestDeleteResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT> for PostsRequestDeleteResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT> for PostsRequestDeleteResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT> for PostsRequestDeleteResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestDeleteResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestDeleteResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTDELETERESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestDeleteResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::PostDeleteResponse as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTDELETERESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTDELETERESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTDELETERESPONSE.into_array();
    #[serde(transparent)]
    pub struct PostsRequestTrashResponse {
        pub data: crate::posts::PostWithEditContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestTrashResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestTrashResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestTrashResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for PostsRequestTrashResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestTrashResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT> for PostsRequestTrashResponse {
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
            <crate::posts::PostWithEditContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <crate::posts::PostWithEditContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestTrashResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT> for PostsRequestTrashResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT> for PostsRequestTrashResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT> for PostsRequestTrashResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT> for PostsRequestTrashResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT> for PostsRequestTrashResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT> for PostsRequestTrashResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT> for PostsRequestTrashResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestTrashResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestTrashResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTTRASHRESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestTrashResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <crate::posts::PostWithEditContext as ::uniffi::TypeId<
                crate::UniFfiTag,
            >>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTTRASHRESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTTRASHRESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTTRASHRESPONSE.into_array();
    #[serde(transparent)]
    pub struct PostsRequestUpdateResponse {
        pub data: PostWithEditContext,
        #[serde(skip)]
        pub foo: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestUpdateResponse {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestUpdateResponse",
                "data",
                &self.data,
                "foo",
                &&self.foo,
            )
        }
    }
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for PostsRequestUpdateResponse {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                _serde::Serialize::serialize(&self.data, __serializer)
            }
        }
    };
    #[doc(hidden)]
    #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for PostsRequestUpdateResponse {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::__private::Result::map(
                    _serde::Deserialize::deserialize(__deserializer),
                    |__transparent| PostsRequestUpdateResponse {
                        data: __transparent,
                        foo: _serde::__private::Default::default(),
                    },
                )
            }
        }
    };
    #[automatically_derived]
    unsafe impl<UT> ::uniffi::FfiConverter<UT> for PostsRequestUpdateResponse {
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
            <PostWithEditContext as ::uniffi::Lower<
                crate::UniFfiTag,
            >>::write(obj.data, buf);
            <u32 as ::uniffi::Lower<crate::UniFfiTag>>::write(obj.foo, buf);
        }
        fn try_read(
            buf: &mut &[::std::primitive::u8],
        ) -> ::uniffi::deps::anyhow::Result<Self> {
            ::std::result::Result::Ok(Self {
                data: <PostWithEditContext as ::uniffi::Lift<
                    crate::UniFfiTag,
                >>::try_read(buf)?,
                foo: <u32 as ::uniffi::Lift<crate::UniFfiTag>>::try_read(buf)?,
            })
        }
        const TYPE_ID_META: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
                ::uniffi::metadata::codes::TYPE_RECORD,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestUpdateResponse");
    }
    unsafe impl<UT> ::uniffi_core::Lower<UT> for PostsRequestUpdateResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn lower(obj: Self) -> Self::FfiType {
            <Self as ::uniffi_core::FfiConverter<UT>>::lower(obj)
        }
        fn write(obj: Self, buf: &mut ::std::vec::Vec<u8>) {
            <Self as ::uniffi_core::FfiConverter<UT>>::write(obj, buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::Lift<UT> for PostsRequestUpdateResponse {
        type FfiType = <Self as ::uniffi_core::FfiConverter<UT>>::FfiType;
        fn try_lift(v: Self::FfiType) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_lift(v)
        }
        fn try_read(buf: &mut &[u8]) -> ::uniffi_core::deps::anyhow::Result<Self> {
            <Self as ::uniffi_core::FfiConverter<UT>>::try_read(buf)
        }
    }
    unsafe impl<UT> ::uniffi_core::LowerReturn<UT> for PostsRequestUpdateResponse {
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
    unsafe impl<UT> ::uniffi_core::LowerError<UT> for PostsRequestUpdateResponse {
        fn lower_error(obj: Self) -> ::uniffi_core::RustBuffer {
            <Self as ::uniffi_core::Lower<UT>>::lower_into_rust_buffer(obj)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftReturn<UT> for PostsRequestUpdateResponse {
        type ReturnType = <Self as ::uniffi_core::Lift<UT>>::FfiType;
        fn try_lift_successful_return(
            v: Self::ReturnType,
        ) -> ::uniffi_core::Result<Self> {
            <Self as ::uniffi_core::Lift<UT>>::try_lift(v)
        }
    }
    unsafe impl<UT> ::uniffi_core::LiftRef<UT> for PostsRequestUpdateResponse {
        type LiftType = Self;
    }
    impl<UT> ::uniffi_core::ConvertError<UT> for PostsRequestUpdateResponse {
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
                pub trait GetConverterSpecialized {
                    fn get_converter(
                        &self,
                    ) -> ::uniffi_core::UnexpectedUniFFICallbackErrorConverterSpecialized;
                }
                impl<
                    T: ::std::convert::Into<PostsRequestUpdateResponse>,
                > GetConverterSpecialized for T {
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
    impl<UT> ::uniffi_core::TypeId<UT> for PostsRequestUpdateResponse {
        const TYPE_ID_META: ::uniffi_core::MetadataBuffer = <Self as ::uniffi_core::FfiConverter<
            UT,
        >>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTUPDATERESPONSE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::RECORD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestUpdateResponse")
        .concat_value(2u8)
        .concat_str("data")
        .concat(
            <PostWithEditContext as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_long_str("")
        .concat_str("foo")
        .concat(<u32 as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META)
        .concat_bool(false)
        .concat_long_str("")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_RECORD_POSTSREQUESTUPDATERESPONSE: [u8; UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTUPDATERESPONSE
        .size] = UNIFFI_META_CONST_WP_API_RECORD_POSTSREQUESTUPDATERESPONSE.into_array();
    pub struct PostsRequestExecutor {
        request_builder: PostsRequestBuilder,
        request_executor: std::sync::Arc<dyn crate::request::RequestExecutor>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for PostsRequestExecutor {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "PostsRequestExecutor",
                "request_builder",
                &self.request_builder,
                "request_executor",
                &&self.request_executor,
            )
        }
    }
    #[doc(hidden)]
    #[no_mangle]
    pub unsafe extern "C" fn uniffi_wp_api_fn_clone_postsrequestexecutor(
        ptr: *const ::std::ffi::c_void,
        call_status: &mut ::uniffi::RustCallStatus,
    ) -> *const ::std::ffi::c_void {
        ::uniffi::rust_call(
            call_status,
            || {
                unsafe { ::std::sync::Arc::increment_strong_count(ptr) };
                ::std::result::Result::Ok(ptr)
            },
        )
    }
    #[doc(hidden)]
    #[no_mangle]
    pub unsafe extern "C" fn uniffi_wp_api_fn_free_postsrequestexecutor(
        ptr: *const ::std::ffi::c_void,
        call_status: &mut ::uniffi::RustCallStatus,
    ) {
        ::uniffi::rust_call(
            call_status,
            || {
                if !!ptr.is_null() {
                    ::core::panicking::panic("assertion failed: !ptr.is_null()")
                }
                let ptr = ptr.cast::<PostsRequestExecutor>();
                unsafe {
                    ::std::sync::Arc::decrement_strong_count(ptr);
                }
                ::std::result::Result::Ok(())
            },
        );
    }
    const _: fn() = || {
        fn assert_impl_all<T: ?Sized + ::core::marker::Sync + ::core::marker::Send>() {}
        assert_impl_all::<PostsRequestExecutor>();
    };
    #[doc(hidden)]
    #[automatically_derived]
    /// Support for passing reference-counted shared objects via the FFI.
    ///
    /// To avoid dealing with complex lifetime semantics over the FFI, any data passed
    /// by reference must be encapsulated in an `Arc`, and must be safe to share
    /// across threads.
    unsafe impl<UT> ::uniffi::FfiConverterArc<UT> for PostsRequestExecutor {
        type FfiType = *const ::std::os::raw::c_void;
        /// When lowering, we have an owned `Arc` and we transfer that ownership
        /// to the foreign-language code, "leaking" it out of Rust's ownership system
        /// as a raw pointer. This works safely because we have unique ownership of `self`.
        /// The foreign-language code is responsible for freeing this by calling the
        /// `ffi_object_free` FFI function provided by the corresponding UniFFI type.
        ///
        /// Safety: when freeing the resulting pointer, the foreign-language code must
        /// call the destructor function specific to the type `T`. Calling the destructor
        /// function for other types may lead to undefined behaviour.
        fn lower(obj: ::std::sync::Arc<Self>) -> Self::FfiType {
            ::std::sync::Arc::into_raw(obj) as Self::FfiType
        }
        /// When lifting, we receive an owned `Arc` that the foreign language code cloned.
        fn try_lift(v: Self::FfiType) -> ::uniffi::Result<::std::sync::Arc<Self>> {
            let v = v as *const PostsRequestExecutor;
            ::std::result::Result::Ok(unsafe { ::std::sync::Arc::<Self>::from_raw(v) })
        }
        /// When writing as a field of a complex structure, make a clone and transfer ownership
        /// of it to the foreign-language code by writing its pointer into the buffer.
        /// The foreign-language code is responsible for freeing this by calling the
        /// `ffi_object_free` FFI function provided by the corresponding UniFFI type.
        ///
        /// Safety: when freeing the resulting pointer, the foreign-language code must
        /// call the destructor function specific to the type `T`. Calling the destructor
        /// function for other types may lead to undefined behaviour.
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
        /// When reading as a field of a complex structure, we receive a "borrow" of the `Arc`
        /// that is owned by the foreign-language code, and make a clone for our own use.
        ///
        /// Safety: the buffer must contain a pointer previously obtained by calling
        /// the `lower()` or `write()` method of this impl.
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
                ::uniffi::metadata::codes::TYPE_INTERFACE,
            )
            .concat_str("wp_api")
            .concat_str("PostsRequestExecutor");
    }
    unsafe impl<UT> ::uniffi::LowerReturn<UT> for PostsRequestExecutor {
        type ReturnType = <::std::sync::Arc<
            Self,
        > as ::uniffi::LowerReturn<crate::UniFfiTag>>::ReturnType;
        fn lower_return(
            obj: Self,
        ) -> ::std::result::Result<Self::ReturnType, ::uniffi::RustCallError> {
            <::std::sync::Arc<
                Self,
            > as ::uniffi::LowerReturn<
                crate::UniFfiTag,
            >>::lower_return(::std::sync::Arc::new(obj))
        }
    }
    unsafe impl<UT> ::uniffi::LowerError<UT> for PostsRequestExecutor {
        fn lower_error(obj: Self) -> ::uniffi::RustBuffer {
            <::std::sync::Arc<
                Self,
            > as ::uniffi::LowerError<
                crate::UniFfiTag,
            >>::lower_error(::std::sync::Arc::new(obj))
        }
    }
    unsafe impl<UT> ::uniffi::LiftRef<UT> for PostsRequestExecutor {
        type LiftType = ::std::sync::Arc<Self>;
    }
    impl<UT> ::uniffi::TypeId<UT> for PostsRequestExecutor {
        const TYPE_ID_META: ::uniffi::MetadataBuffer = <::std::sync::Arc<
            Self,
        > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META;
    }
    const UNIFFI_META_CONST_WP_API_INTERFACE_POSTSREQUESTEXECUTOR: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::INTERFACE,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_INTERFACE_POSTSREQUESTEXECUTOR: [u8; UNIFFI_META_CONST_WP_API_INTERFACE_POSTSREQUESTEXECUTOR
        .size] = UNIFFI_META_CONST_WP_API_INTERFACE_POSTSREQUESTEXECUTOR.into_array();
    impl PostsRequestExecutor {
        pub fn new(
            api_base_url: std::sync::Arc<crate::request::endpoint::ApiBaseUrl>,
            authentication: crate::WpAuthentication,
            request_executor: std::sync::Arc<dyn crate::request::RequestExecutor>,
        ) -> Self {
            Self {
                request_builder: PostsRequestBuilder::new(api_base_url, authentication),
                request_executor,
            }
        }
    }
    impl PostsRequestExecutor {
        pub async fn list_with_edit_context(
            &self,
            params: &PostListParams,
        ) -> Result<PostsRequestListWithEditContextResponse, crate::WpApiError> {
            let request = self.request_builder.list_with_edit_context(params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn filter_list_with_edit_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithEditContext],
        ) -> Result<PostsRequestFilterListWithEditContextResponse, crate::WpApiError> {
            let request = self
                .request_builder
                .filter_list_with_edit_context(params, fields);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn list_with_embed_context(
            &self,
            params: &PostListParams,
        ) -> Result<PostsRequestListWithEmbedContextResponse, crate::WpApiError> {
            let request = self.request_builder.list_with_embed_context(params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn filter_list_with_embed_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithEmbedContext],
        ) -> Result<PostsRequestFilterListWithEmbedContextResponse, crate::WpApiError> {
            let request = self
                .request_builder
                .filter_list_with_embed_context(params, fields);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn list_with_view_context(
            &self,
            params: &PostListParams,
        ) -> Result<PostsRequestListWithViewContextResponse, crate::WpApiError> {
            let request = self.request_builder.list_with_view_context(params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn filter_list_with_view_context(
            &self,
            params: &PostListParams,
            fields: &[crate::posts::SparsePostFieldWithViewContext],
        ) -> Result<PostsRequestFilterListWithViewContextResponse, crate::WpApiError> {
            let request = self
                .request_builder
                .filter_list_with_view_context(params, fields);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn retrieve_with_edit_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> Result<PostsRequestRetrieveWithEditContextResponse, crate::WpApiError> {
            let request = self
                .request_builder
                .retrieve_with_edit_context(post_id, params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn filter_retrieve_with_edit_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithEditContext],
        ) -> Result<
            PostsRequestFilterRetrieveWithEditContextResponse,
            crate::WpApiError,
        > {
            let request = self
                .request_builder
                .filter_retrieve_with_edit_context(post_id, params, fields);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn retrieve_with_embed_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> Result<PostsRequestRetrieveWithEmbedContextResponse, crate::WpApiError> {
            let request = self
                .request_builder
                .retrieve_with_embed_context(post_id, params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn filter_retrieve_with_embed_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithEmbedContext],
        ) -> Result<
            PostsRequestFilterRetrieveWithEmbedContextResponse,
            crate::WpApiError,
        > {
            let request = self
                .request_builder
                .filter_retrieve_with_embed_context(post_id, params, fields);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn retrieve_with_view_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
        ) -> Result<PostsRequestRetrieveWithViewContextResponse, crate::WpApiError> {
            let request = self
                .request_builder
                .retrieve_with_view_context(post_id, params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn filter_retrieve_with_view_context(
            &self,
            post_id: &PostId,
            params: &crate::posts::PostRetrieveParams,
            fields: &[crate::posts::SparsePostFieldWithViewContext],
        ) -> Result<
            PostsRequestFilterRetrieveWithViewContextResponse,
            crate::WpApiError,
        > {
            let request = self
                .request_builder
                .filter_retrieve_with_view_context(post_id, params, fields);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn create(
            &self,
            params: &crate::posts::PostCreateParams,
        ) -> Result<PostsRequestCreateResponse, crate::WpApiError> {
            let request = self.request_builder.create(params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn delete(
            &self,
            post_id: &PostId,
        ) -> Result<PostsRequestDeleteResponse, crate::WpApiError> {
            let request = self.request_builder.delete(post_id);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn trash(
            &self,
            post_id: &PostId,
        ) -> Result<PostsRequestTrashResponse, crate::WpApiError> {
            let request = self.request_builder.trash(post_id);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
        pub async fn update(
            &self,
            post_id: &PostId,
            params: &PostUpdateParams,
        ) -> Result<PostsRequestUpdateResponse, crate::WpApiError> {
            let request = self.request_builder.update(post_id, params);
            self.request_executor.execute(std::sync::Arc::new(request)).await?.parse()
        }
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_list_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("list_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestListWithEditContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .list_with_edit_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("list_with_edit_context")
        .concat_bool(true)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestListWithEditContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_list_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_filter_list_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_list_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestFilterListWithEditContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_list_with_edit_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                                <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEditContext],
                                >>::borrow(&uniffi_args.2),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("filter_list_with_edit_context")
        .concat_bool(true)
        .concat_value(2u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestFilterListWithEditContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_filter_list_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_list_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("list_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestListWithEmbedContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .list_with_embed_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("list_with_embed_context")
        .concat_bool(true)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestListWithEmbedContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_list_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_filter_list_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_list_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestFilterListWithEmbedContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_list_with_embed_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                                <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEmbedContext],
                                >>::borrow(&uniffi_args.2),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("filter_list_with_embed_context")
        .concat_bool(true)
        .concat_value(2u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestFilterListWithEmbedContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_filter_list_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_list_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("list_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestListWithViewContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .list_with_view_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("list_with_view_context")
        .concat_bool(true)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestListWithViewContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_list_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_LIST_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_filter_list_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostListParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_list_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestFilterListWithViewContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_list_with_view_context(
                                <<PostListParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostListParams,
                                >>::borrow(&uniffi_args.1),
                                <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithViewContext],
                                >>::borrow(&uniffi_args.2),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("filter_list_with_view_context")
        .concat_bool(true)
        .concat_value(2u8)
        .concat_str("params")
        .concat(
            <<PostListParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestFilterListWithViewContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_filter_list_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_LIST_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_retrieve_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("retrieve_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestRetrieveWithEditContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .retrieve_with_edit_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("retrieve_with_edit_context")
        .concat_bool(true)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestRetrieveWithEditContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_retrieve_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_filter_retrieve_with_edit_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_retrieve_with_edit_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestFilterRetrieveWithEditContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_retrieve_with_edit_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                                <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEditContext],
                                >>::borrow(&uniffi_args.3),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EDIT_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("filter_retrieve_with_edit_context")
        .concat_bool(true)
        .concat_value(3u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEditContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestFilterRetrieveWithEditContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EDIT_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EDIT_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EDIT_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_filter_retrieve_with_edit_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EDIT_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_retrieve_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("retrieve_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestRetrieveWithEmbedContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .retrieve_with_embed_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("retrieve_with_embed_context")
        .concat_bool(true)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestRetrieveWithEmbedContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_retrieve_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_filter_retrieve_with_embed_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_retrieve_with_embed_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<
                PostsRequestFilterRetrieveWithEmbedContextResponse,
                crate::WpApiError,
            >,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_retrieve_with_embed_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                                <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithEmbedContext],
                                >>::borrow(&uniffi_args.3),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EMBED_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("filter_retrieve_with_embed_context")
        .concat_bool(true)
        .concat_value(3u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithEmbedContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestFilterRetrieveWithEmbedContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EMBED_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EMBED_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EMBED_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_filter_retrieve_with_embed_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_EMBED_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_retrieve_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("retrieve_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestRetrieveWithViewContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .retrieve_with_view_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("retrieve_with_view_context")
        .concat_bool(true)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestRetrieveWithViewContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_retrieve_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_RETRIEVE_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_filter_retrieve_with_view_context(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        fields: <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("filter_retrieve_with_view_context"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
            match <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(fields) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("fields", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestFilterRetrieveWithViewContextResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .filter_retrieve_with_view_context(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostRetrieveParams,
                                >>::borrow(&uniffi_args.2),
                                <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    [crate::posts::SparsePostFieldWithViewContext],
                                >>::borrow(&uniffi_args.3),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_VIEW_CONTEXT: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("filter_retrieve_with_view_context")
        .concat_bool(true)
        .concat_value(3u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<crate::posts::PostRetrieveParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("fields")
        .concat(
            <<[crate::posts::SparsePostFieldWithViewContext] as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestFilterRetrieveWithViewContextResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_VIEW_CONTEXT: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_VIEW_CONTEXT
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_VIEW_CONTEXT
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_filter_retrieve_with_view_context() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_FILTER_RETRIEVE_WITH_VIEW_CONTEXT
            .checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_create(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("create"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestCreateResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .create(
                                <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    crate::posts::PostCreateParams,
                                >>::borrow(&uniffi_args.1),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_CREATE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("create")
        .concat_bool(true)
        .concat_value(1u8)
        .concat_str("params")
        .concat(
            <<crate::posts::PostCreateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestCreateResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_CREATE: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_CREATE
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_CREATE
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_create() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_CREATE.checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_delete(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("delete"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestDeleteResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .delete(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_DELETE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("delete")
        .concat_bool(true)
        .concat_value(1u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestDeleteResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_DELETE: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_DELETE
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_DELETE
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_delete() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_DELETE.checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_trash(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("trash"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestTrashResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .trash(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_TRASH: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("trash")
        .concat_bool(true)
        .concat_value(1u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestTrashResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_TRASH: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_TRASH
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_TRASH.into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_trash() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_TRASH.checksum()
    }
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_fn_method_postsrequestexecutor_update(
        uniffi_self_lowered: <::std::sync::Arc<
            PostsRequestExecutor,
        > as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        post_id: <<PostId as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
        params: <<PostUpdateParams as ::uniffi::LiftRef<
            crate::UniFfiTag,
        >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::FfiType,
    ) -> ::uniffi::Handle {
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api::log(
                    format_args!("update"),
                    lvl,
                    &(
                        "wp_api::request::endpoint::posts_endpoint",
                        "wp_api::request::endpoint::posts_endpoint",
                        ::log::__private_api::loc(),
                    ),
                    (),
                );
            }
        };
        let uniffi_lifted_args = (move || ::std::result::Result::Ok((
            match <::std::sync::Arc<
                PostsRequestExecutor,
            > as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(uniffi_self_lowered) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("self", e));
                }
            },
            match <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(post_id) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("post_id", e));
                }
            },
            match <<PostUpdateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::Lift<crate::UniFfiTag>>::try_lift(params) {
                ::std::result::Result::Ok(v) => v,
                ::std::result::Result::Err(e) => {
                    return ::std::result::Result::Err(("params", e));
                }
            },
        )))();
        ::uniffi::rust_future_new::<
            _,
            Result<PostsRequestUpdateResponse, crate::WpApiError>,
            _,
        >(
            async move {
                match uniffi_lifted_args {
                    ::std::result::Result::Ok(uniffi_args) => {
                        let uniffi_result = uniffi_args
                            .0
                            .update(
                                <<PostId as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostId,
                                >>::borrow(&uniffi_args.1),
                                <<PostUpdateParams as ::uniffi::LiftRef<
                                    crate::UniFfiTag,
                                >>::LiftType as ::std::borrow::Borrow<
                                    PostUpdateParams,
                                >>::borrow(&uniffi_args.2),
                            )
                            .await;
                        Ok(uniffi_result)
                    }
                    ::std::result::Result::Err((arg_name, error)) => {
                        Err(::uniffi::LiftArgsError {
                            arg_name,
                            error,
                        })
                    }
                }
            },
            crate::UniFfiTag,
        )
    }
    const UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_UPDATE: ::uniffi::MetadataBuffer = ::uniffi::MetadataBuffer::from_code(
            ::uniffi::metadata::codes::METHOD,
        )
        .concat_str("wp_api")
        .concat_str("PostsRequestExecutor")
        .concat_str("update")
        .concat_bool(true)
        .concat_value(2u8)
        .concat_str("post_id")
        .concat(
            <<PostId as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat_str("params")
        .concat(
            <<PostUpdateParams as ::uniffi::LiftRef<
                crate::UniFfiTag,
            >>::LiftType as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_bool(false)
        .concat(
            <Result<
                PostsRequestUpdateResponse,
                crate::WpApiError,
            > as ::uniffi::TypeId<crate::UniFfiTag>>::TYPE_ID_META,
        )
        .concat_long_str("");
    #[no_mangle]
    #[doc(hidden)]
    pub static UNIFFI_META_WP_API_METHOD_POSTSREQUESTEXECUTOR_UPDATE: [u8; UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_UPDATE
        .size] = UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_UPDATE
        .into_array();
    #[doc(hidden)]
    #[no_mangle]
    pub extern "C" fn uniffi_wp_api_checksum_method_postsrequestexecutor_update() -> u16 {
        UNIFFI_META_CONST_WP_API_METHOD_POSTSREQUESTEXECUTOR_UPDATE.checksum()
    }
    impl DerivedRequest for PostsRequest {
        fn additional_query_pairs(&self) -> Vec<(&str, String)> {
            match self {
                PostsRequest::Delete => {
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([("force", true.to_string())]),
                    )
                }
                PostsRequest::Trash => {
                    <[_]>::into_vec(
                        #[rustc_box]
                        ::alloc::boxed::Box::new([("force", false.to_string())]),
                    )
                }
                _ => ::alloc::vec::Vec::new(),
            }
        }
        fn namespace() -> Namespace {
            Namespace::WpV2
        }
    }
    impl SparseField for SparsePostFieldWithEditContext {
        fn as_str(&self) -> &str {
            match self {
                Self::PostType => "type",
                _ => self.as_field_name(),
            }
        }
    }
    impl SparseField for SparsePostFieldWithEmbedContext {
        fn as_str(&self) -> &str {
            match self {
                Self::PostType => "type",
                _ => self.as_field_name(),
            }
        }
    }
    impl SparseField for SparsePostFieldWithViewContext {
        fn as_str(&self) -> &str {
            match self {
                Self::PostType => "type",
                _ => self.as_field_name(),
            }
        }
    }
}
