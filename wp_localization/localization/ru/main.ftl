api_root_link_header_not_found = Ссылка WordPress REST API не найдена в ответе сайта

generic_error = Что-то пошло не так

site_error_message = Ваш сайт отправил сообщение об ошибке: {$error_message}

url_parsing_error = Недопустимый URL

response_parsing_error = Не удалось обработать ответ: {$reason}

media_file_not_found = Медиафайл не найден по пути {$path}

invalid_http_status_code = Недопустимый код состояния HTTP: {$status_code}

request_execution_failed = Не удалось отправить HTTP

just = {$message}

invalid_ssl_error = Недопустимый сертификат SSL

non_existent_site_error = Не удалось найти сервер с указанным именем компьютера

http_authentication_required_error = Сервер на {$url} требует аутентификации. Пожалуйста, введите имя пользователя и пароль

http_authentication_rejected_error = Сервер на {$url} отклонил ваши учетные данные для входа. Пожалуйста, введите действительное имя пользователя и пароль.

misconfigured_http_authentication_error = Сервер отправляет недопустимые данные аутентификации HTTP. Пожалуйста, проверьте конфигурацию аутентификации HTTP вашего сайта

misconfigured_rate_limit_error = Сервер ограничивает запросы таким образом, что они никогда не будут успешными. Пожалуйста, проверьте конфигурацию ограничения скорости вашего сайта

oauth_response_url_error_missing_site_url = Предоставленный URL не содержит параметра запроса `site_url`
oauth_response_url_error_missing_username = Предоставленный URL не содержит параметра запроса `username`
oauth_response_url_error_missing_password = Предоставленный URL не содержит параметра запроса `password`
oauth_response_url_error_unsuccessful_login = Неудачный вход

boolean_true_is_returned_when_string_is_expected = Ожидается значение типа `String` для этого поля, но получено булево значение `true`

invalid_header_name_error = Недопустимое имя заголовка: {$header_name}

invalid_header_value_error = Недопустимое значение заголовка: {$header_value}

http_auth_method_missing_nonce = Отсутствует nonce в методе аутентификации HTTP
http_auth_method_missing_qop = Отсутствует QOP (Качество защиты) в методе аутентификации HTTP
http_auth_method_missing_algorithm = Отсутствует алгоритм в методе аутентификации HTTP
http_auth_method_missing_opaque = Отсутствует непрозрачное значение в методе аутентификации HTTP
http_auth_method_unknown = Неизвестный метод аутентификации HTTP

uniffi_serialization_error_serde = Ошибка сериализации: {$reason}

uuid_parse_error_invalid_uuid = Недопустимая строка UUID
uuid_parse_error_not_version_4 = Не является UUID версии 4

wordpress_org_api_client_error_request_encoding = Не удалось закодировать запрос. Причина: {$reason}

http_forbidden_error = Сервер на {$url} отказал в доступе к запрошенному ресурсу. Пожалуйста, проверьте конфигурацию вашего сайта
