use crate::{
    WpAppNotifier, api_client_generate_api_client, api_client_generate_endpoint_impl,
    api_client_generate_request_builder,
    auth::WpAuthenticationProvider,
    middleware::WpApiMiddlewarePipeline,
    request::{
        RequestExecutor,
        endpoint::{
            ApiUrlResolver,
            api_root_endpoint::{ApiRootRequestBuilder, ApiRootRequestExecutor},
            application_passwords_endpoint::{
                ApplicationPasswordsRequestBuilder, ApplicationPasswordsRequestExecutor,
            },
            categories_endpoint::{CategoriesRequestBuilder, CategoriesRequestExecutor},
            comments_endpoint::{CommentsRequestBuilder, CommentsRequestExecutor},
            media_endpoint::{MediaRequestBuilder, MediaRequestExecutor},
            plugins_endpoint::{PluginsRequestBuilder, PluginsRequestExecutor},
            post_autosaves_endpoint::{AutosavesRequestBuilder, AutosavesRequestExecutor},
            post_revisions_endpoint::{PostRevisionsRequestBuilder, PostRevisionsRequestExecutor},
            post_statuses_endpoint::{PostStatusesRequestBuilder, PostStatusesRequestExecutor},
            post_types_endpoint::{PostTypesRequestBuilder, PostTypesRequestExecutor},
            posts_endpoint::{PostsRequestBuilder, PostsRequestExecutor},
            search_endpoint::{SearchRequestBuilder, SearchRequestExecutor},
            site_settings_endpoint::{SiteSettingsRequestBuilder, SiteSettingsRequestExecutor},
            tags_endpoint::{TagsRequestBuilder, TagsRequestExecutor},
            taxonomies_endpoint::{TaxonomiesRequestBuilder, TaxonomiesRequestExecutor},
            templates_endpoint::{TemplatesRequestBuilder, TemplatesRequestExecutor},
            themes_endpoint::{ThemesRequestBuilder, ThemesRequestExecutor},
            users_endpoint::{UsersRequestBuilder, UsersRequestExecutor},
            widget_types_endpoint::{WidgetTypesRequestBuilder, WidgetTypesRequestExecutor},
            widgets_endpoint::{WidgetsRequestBuilder, WidgetsRequestExecutor},
            wp_block_editor_endpoint::{WpBlockEditorRequestBuilder, WpBlockEditorRequestExecutor},
            wp_site_health_tests_endpoint::{
                WpSiteHealthTestsRequestBuilder, WpSiteHealthTestsRequestExecutor,
            },
        },
    },
};
use std::sync::Arc;

#[derive(uniffi::Object)]
struct UniffiWpApiRequestBuilder {
    inner: WpApiRequestBuilder,
}

pub struct WpApiRequestBuilder {
    api_root: Arc<ApiRootRequestBuilder>,
    application_passwords: Arc<ApplicationPasswordsRequestBuilder>,
    autosaves: Arc<AutosavesRequestBuilder>,
    categories: Arc<CategoriesRequestBuilder>,
    comments: Arc<CommentsRequestBuilder>,
    media: Arc<MediaRequestBuilder>,
    plugins: Arc<PluginsRequestBuilder>,
    post_revisions: Arc<PostRevisionsRequestBuilder>,
    post_statuses: Arc<PostStatusesRequestBuilder>,
    post_types: Arc<PostTypesRequestBuilder>,
    posts: Arc<PostsRequestBuilder>,
    search: Arc<SearchRequestBuilder>,
    site_settings: Arc<SiteSettingsRequestBuilder>,
    tags: Arc<TagsRequestBuilder>,
    taxonomies: Arc<TaxonomiesRequestBuilder>,
    templates: Arc<TemplatesRequestBuilder>,
    themes: Arc<ThemesRequestBuilder>,
    users: Arc<UsersRequestBuilder>,
    widget_types: Arc<WidgetTypesRequestBuilder>,
    widgets: Arc<WidgetsRequestBuilder>,
    wp_block_editor: Arc<WpBlockEditorRequestBuilder>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestBuilder>,
}

impl WpApiRequestBuilder {
    pub fn new(
        api_url_resolver: Arc<dyn ApiUrlResolver>,
        auth_provider: Arc<WpAuthenticationProvider>,
    ) -> Self {
        api_client_generate_request_builder!(
            api_url_resolver,
            auth_provider;
            api_root,
            application_passwords,
            autosaves,
            categories,
            comments,
            media,
            plugins,
            post_revisions,
            post_statuses,
            post_types,
            posts,
            search,
            site_settings,
            tags,
            taxonomies,
            templates,
            themes,
            users,
            widget_types,
            widgets,
            wp_block_editor,
            wp_site_health_tests
        )
    }
}

#[derive(uniffi::Object)]
struct UniffiWpApiClient {
    inner: WpApiClient,
}

#[uniffi::export]
impl UniffiWpApiClient {
    #[uniffi::constructor]
    fn new(api_url_resolver: Arc<dyn ApiUrlResolver>, delegate: WpApiClientDelegate) -> Self {
        Self {
            inner: WpApiClient::new(api_url_resolver, delegate),
        }
    }
}

pub struct WpApiClient {
    api_root: Arc<ApiRootRequestExecutor>,
    application_passwords: Arc<ApplicationPasswordsRequestExecutor>,
    autosaves: Arc<AutosavesRequestExecutor>,
    categories: Arc<CategoriesRequestExecutor>,
    comments: Arc<CommentsRequestExecutor>,
    media: Arc<MediaRequestExecutor>,
    plugins: Arc<PluginsRequestExecutor>,
    post_revisions: Arc<PostRevisionsRequestExecutor>,
    post_statuses: Arc<PostStatusesRequestExecutor>,
    post_types: Arc<PostTypesRequestExecutor>,
    posts: Arc<PostsRequestExecutor>,
    search: Arc<SearchRequestExecutor>,
    site_settings: Arc<SiteSettingsRequestExecutor>,
    tags: Arc<TagsRequestExecutor>,
    taxonomies: Arc<TaxonomiesRequestExecutor>,
    templates: Arc<TemplatesRequestExecutor>,
    themes: Arc<ThemesRequestExecutor>,
    users: Arc<UsersRequestExecutor>,
    widget_types: Arc<WidgetTypesRequestExecutor>,
    widgets: Arc<WidgetsRequestExecutor>,
    wp_block_editor: Arc<WpBlockEditorRequestExecutor>,
    wp_site_health_tests: Arc<WpSiteHealthTestsRequestExecutor>,
}

impl WpApiClient {
    pub fn new(api_url_resolver: Arc<dyn ApiUrlResolver>, delegate: WpApiClientDelegate) -> Self {
        api_client_generate_api_client!(
            api_url_resolver,
            delegate;
            api_root,
            application_passwords,
            autosaves,
            categories,
            comments,
            media,
            plugins,
            post_revisions,
            post_statuses,
            post_types,
            posts,
            search,
            site_settings,
            tags,
            taxonomies,
            templates,
            themes,
            users,
            widget_types,
            widgets,
            wp_block_editor,
            wp_site_health_tests
        )
    }
}

// IMPORTANT: This type may be aggressively cloned, so all of its fields must be cheap to clone!
#[derive(Clone, uniffi::Record)]
pub struct WpApiClientDelegate {
    pub auth_provider: Arc<WpAuthenticationProvider>,
    pub request_executor: Arc<dyn RequestExecutor>,
    pub middleware_pipeline: Arc<WpApiMiddlewarePipeline>,
    pub app_notifier: Arc<dyn WpAppNotifier>,
}

pub trait IsWpApiClientDelegate {
    fn get_delegate(&self) -> &WpApiClientDelegate;
}

api_client_generate_endpoint_impl!(WpApi, api_root);
api_client_generate_endpoint_impl!(WpApi, application_passwords);
api_client_generate_endpoint_impl!(WpApi, autosaves);
api_client_generate_endpoint_impl!(WpApi, categories);
api_client_generate_endpoint_impl!(WpApi, comments);
api_client_generate_endpoint_impl!(WpApi, media);
api_client_generate_endpoint_impl!(WpApi, plugins);
api_client_generate_endpoint_impl!(WpApi, post_revisions);
api_client_generate_endpoint_impl!(WpApi, post_statuses);
api_client_generate_endpoint_impl!(WpApi, post_types);
api_client_generate_endpoint_impl!(WpApi, posts);
api_client_generate_endpoint_impl!(WpApi, search);
api_client_generate_endpoint_impl!(WpApi, site_settings);
api_client_generate_endpoint_impl!(WpApi, tags);
api_client_generate_endpoint_impl!(WpApi, taxonomies);
api_client_generate_endpoint_impl!(WpApi, templates);
api_client_generate_endpoint_impl!(WpApi, themes);
api_client_generate_endpoint_impl!(WpApi, users);
api_client_generate_endpoint_impl!(WpApi, widget_types);
api_client_generate_endpoint_impl!(WpApi, widgets);
api_client_generate_endpoint_impl!(WpApi, wp_block_editor);
api_client_generate_endpoint_impl!(WpApi, wp_site_health_tests);

#[macro_export]
macro_rules! api_client_generate_endpoint_impl {
    ($client_name_prefix: ident, $feature:ident) => {
        paste::paste! {
            #[uniffi::export]

            impl [<Uniffi $client_name_prefix RequestBuilder>] {
                fn $feature(&self) -> Arc<[<$feature:camel RequestBuilder>]> {
                    self.inner.$feature.clone()
                }
            }

            impl [<$client_name_prefix RequestBuilder>] {
                pub fn $feature(&self) -> &[<$feature:camel RequestBuilder>] {
                    self.$feature.as_ref()
                }
            }

            #[uniffi::export]
            impl [<Uniffi $client_name_prefix Client>] {
                fn $feature(&self) -> Arc<[<$feature:camel RequestExecutor>]> {
                    self.inner.$feature.clone()
                }
            }

            impl [<$client_name_prefix Client>] {
                pub fn $feature(&self) -> &[<$feature:camel RequestExecutor>] {
                    self.$feature.as_ref()
                }
            }
        }
    };
}

#[macro_export]
macro_rules! api_client_generate_request_builder {
    ($api_url_resolver:ident, $authentication:ident; $($element:expr),*) => {
        paste::paste! {
            Self {
                $($element: [<$element:camel RequestBuilder>]::new(
                    $api_url_resolver.clone(),
                    $authentication.clone(),
                )
                .into(),)*
            }
        }
    };
}

#[macro_export]
macro_rules! api_client_generate_api_client {
    ($api_url_resolver:ident, $delegate:ident; $($element:expr),*) => {
        paste::paste! {
            Self {
                $($element: [<$element:camel RequestExecutor>]::new(
                    $api_url_resolver.clone(),
                    $delegate.clone(),
                )
                .into(),)*
            }
        }
    };
}
