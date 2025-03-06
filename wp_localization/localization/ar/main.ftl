api_root_link_header_not_found = لم يتم العثور على رابط WordPress REST API في استجابة الموقع

generic_error = حدث خطأ ما

site_error_message = أرسل موقعك رسالة خطأ: {$error_message}

url_parsing_error = عنوان URL غير صالح

response_parsing_error = تعذر معالجة الاستجابة: {$reason}

media_file_not_found = لم يتم العثور على ملف الوسائط في {$path}

invalid_http_status_code = رمز حالة HTTP غير صالح: {$status_code}

request_execution_failed = فشل إرسال HTTP

just = {$message}

invalid_ssl_error = شهادة SSL غير صالحة

non_existent_site_error = تعذر العثور على خادم باسم المضيف المحدد

http_authentication_required_error = يتطلب الخادم في {$url} المصادقة. أدخل اسم المستخدم وكلمة المرور الخاصة بك

http_authentication_rejected_error = رفض الخادم في {$url} بيانات اعتمادك. أدخل اسم مستخدم وكلمة مرور صالحين.

misconfigured_http_authentication_error = يرسل الخادم معلومات مصادقة HTTP غير صالحة. تحقق من تكوين مصادقة HTTP لموقعك

misconfigured_rate_limit_error = يقيد الخادم الطلبات بطريقة لن تنجح أبداً. تحقق من تكوين حد معدل موقعك

oauth_response_url_error_missing_site_url = لا يحتوي عنوان URL المقدم على معامل استعلام `site_url`
oauth_response_url_error_missing_username = لا يحتوي عنوان URL المقدم على معامل استعلام `username`
oauth_response_url_error_missing_password = لا يحتوي عنوان URL المقدم على معامل استعلام `password`
oauth_response_url_error_unsuccessful_login = فشل تسجيل الدخول

boolean_true_is_returned_when_string_is_expected = يتوقع قيمة `String` لهذا الحقل، ولكن تم استلام قيمة منطقية `true`

invalid_header_name_error = اسم الترويسة غير صالح: {$header_name}

invalid_header_value_error = قيمة الترويسة غير صالحة: {$header_value}

http_auth_method_missing_nonce = مفقود nonce في طريقة مصادقة HTTP
http_auth_method_missing_qop = مفقود QOP (جودة الحماية) في طريقة مصادقة HTTP
http_auth_method_missing_algorithm = مفقود الخوارزمية في طريقة مصادقة HTTP
http_auth_method_missing_opaque = مفقود قيمة opaque في طريقة مصادقة HTTP
http_auth_method_unknown = طريقة مصادقة HTTP غير معروفة

uniffi_serialization_error_serde = خطأ في التسلسل: {$reason}

uuid_parse_error_invalid_uuid = سلسلة UUID غير صالحة
uuid_parse_error_not_version_4 = ليس UUID الإصدار 4

wordpress_org_api_client_error_request_encoding = فشل في ترميز الطلب. السبب: {$reason}
