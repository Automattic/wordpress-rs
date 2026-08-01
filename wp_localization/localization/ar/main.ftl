already_logged_in = لقد سبق وأن سجّلت دخولك باسم المستخدم { $username }.

site_error_message = { $error_message }

http_cancellation_error = تم إلغاء الطلب.

parse_api_root_failure_reason_wordfence_blocking_access = Wordfence يمنع الوصول إلى API الخاصة بالموقع. يرجى التحقق من إعدادات Wordfence الخاصة بك.

parse_api_root_failure_reason_server_fatal_error = واجه خادمك خطأ لا يمكن استرداده وبالتالي تعذّر عليه معالجة الطلب. يرجى التحقق من سجلات أخطاء الخادم لديك للحصول على التفاصيل.

parse_api_root = لقد تعذّر تحليل استجابة جذر REST API للموقع.

application_passwords_not_supported = لا يدعم هذا الموقع كلمات مرور التطبيقات.

application_passwords_disabled_for_http_site = لم يتم تمكين «كلمات مرور التطبيقات» لهذا الموقع — ويرجع ذلك على الأرجح إلى عدم تمكننا من إنشاء اتصال آمن به. يرجى إضافة شهادة SSL إلى هذا الموقع ثم المحاولة مجددا.

site_is_local_development_environment = هذا الموقع هو بيئة تطوير محلية. عليك تمكين كلمات مرور التطبيقات للاتصال به عبر التطبيق.

application_password_blocked_by_multiple_plugins = تعذر تسجيل الدخول إلى { $url } - هناك العديد من الإضافات المثبتة التي قد تكون قد عطلت كلمات مرور التطبيقات. يرجى تعطيلها والمحاولة مرة أخرى.

application_password_blocked_by_plugin = تعذر تسجيل الدخول إلى { $url } - ربما تكون الإضافة { $plugin } قد عطّلت كلمات مرور التطبيقات. يرجى زيارة { $support_url } لمعرفة المزيد.

xmlrpc_endpoint_not_found = لم يتم العثور على نقطة نهاية XML-RPC الخاصة بالموقع. يرجى التحقق من إعدادات موقعك والمحاولة مرة أخرى.

xmlrpc_disabled_by_multiple_plugins = تم تعطيل بروتوكول XML-RPC الخاص بالموقع - هناك العديد من الإضافات المثبتة التي قد تكون قد عطلت بروتوكول XML-RPC. يرجى تعطيلها والمحاولة مرة أخرى.

xmlrpc_disabled_by_plugin = تم تعطيل بروتوكول XML-RPC في الموقع - ربما تكون الإضافة { $plugin } قد عطّلته. يُرجى زيارة { $support_url } لمعرفة المزيد.

xmlrpc_disabled_by_host = تم تعطيل بروتوكول XML-RPC الخاص بالموقع. يرجى الاتصال بمزود خدمة الاستضافة لحل هذه المشكلة.

rest_api_disabled = تم تعطيل REST API لموقعك. يرجى تحديث إعدادات موقعك لتمكين REST API.

wordpress_org_api_client_error_request_encoding = تعذّر ترميز الطلب. السبب: { $reason }.

uuid_parse_error_not_version_4 = ليس من النسخة 4 UUID.

uuid_parse_error_invalid_uuid = سلسلة UUID غير صالحة.

uniffi_serialization_error_serde = خطأ في التسلسل: { $reason }.

http_auth_method_unknown = طريقة مصادقة HTTP غير معروفة.

http_auth_method_missing_opaque = قيمة غير شفافة مفقودة في طريقة مصادقة HTTP.

http_auth_method_missing_algorithm = خوارزمية مفقودة في طريقة مصادقة HTTP.

http_auth_method_missing_qop = فقدان خاصية QOP (جودة الحماية) في طريقة مصادقة HTTP.

http_auth_method_missing_nonce = قيمة nonce مفقودة في طريقة مصادقة HTTP.

invalid_header_value_error = قيمة ترويسة غير صالحة: { $header_value }.

invalid_header_name_error = اسم ترويسة غير صالح: { $header_name }.

boolean_true_is_returned_when_string_is_expected = كان من المتوقع الحصول على قيمة من نوع `String` لهذا الحقل، ولكن تم استلام القيمة المنطقية `true` بدلاً من ذلك.

oauth_response_url_error_unsuccessful_login = تسجيل الدخول غير ناجح.

oauth_response_url_error_url_invalid = أرسل الموقع عنوان URL غير صالح لاستجابة المصادقة.

misconfigured_rate_limit_error = يقوم الخادم بتقييد معدل الطلبات بطريقة لن تنجح أبدًا. يرجى مراجعة إعدادات تقييد معدل الطلبات في موقعك.

misconfigured_http_authentication_error = يرسل الخادم معلومات مصادقة HTTP غير صالحة. يرجى التحقق من إعدادات مصادقة HTTP لموقعك.

http_server_error = تعذر الاتصال بالخادم: { $reason }. يرجى الاتصال بمزود خدمة الخادم الخاص بك.

http_authentication_rejected_error = رفض الخادم الموجود على { $url } بيانات اعتمادك. يرجى تقديم اسم مستخدم وكلمة مرور صحيحين.

http_timeout_error = انتهى وقت الاتصال

http_forbidden_error = رفض الخادم الموجود على الرابط { $url } الوصول إلى المورد المطلوب. يرجى التحقق من إعدادات موقعك.

http_authentication_required_error = الخادم الموجود على الرابط { $url } يتطلب المصادقة. يرجى تقديم اسم المستخدم وكلمة المرور.

non_existent_site_error = تعذر العثور على الخادم الذي يحمل اسم المضيف المحدد.

invalid_ssl_error_generic_ssl_error = تعذر إنشاء اتصال آمن بالخادم

invalid_ssl_error_certificate_not_valid_for_name = شهادة SSL غير صالحة

just = { $message }

request_execution_failed = فشل في إرسال HTTP.

invalid_http_status_code = كود حالة HTTP غير صالح: { $status_code }.

media_file_not_found = ملف الوسائط غير موجود في { $path }.

response_parsing_error = لا يمكن تحليل الاستجابة: { $reason }.

url_parsing_error = عنوان الموقع غير صالح.

wp_api_error_generic_error = حدث خطأ ما.

probably_not_wordpress_site = الموقع لا يبدو أنه موقع ووردبريس.
