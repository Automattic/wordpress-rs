site_error_message = { $error_message }

http_cancellation_error = Begäran avbröts.

parse_api_root_failure_reason_wordfence_blocking_access = Wordfence blockerar åtkomst till webbplatsens API. Kontrollera din Wordfence-konfiguration.

parse_api_root_failure_reason_server_fatal_error = Din server stötte på ett oåterkalleligt fel och kunde inte bearbeta begäran. Kontrollera din servers felloggar för detaljer.

application_passwords_not_supported = Webbplatsen stöder inte applikationslösenord.

application_passwords_disabled_for_http_site = Applikationslösenord är inte aktiverat för denna webbplats – detta beror troligen på att vi inte kan etablera en säker anslutning till den. Lägg till ett SSL-certifikat på denna webbplats och försök igen.

site_is_local_development_environment = Denna webbplats är en lokal utvecklingsmiljö. Du måste aktivera applikationslösenord för att ansluta till den med appen.

application_password_blocked_by_multiple_plugins = Kan inte logga in på { $url } – det finns flera installerade tillägg som kan ha inaktiverat applikationslösenord. Inaktivera dem och försök igen.

application_password_blocked_by_plugin = Kan inte logga in på { $url } – tillägget { $plugin } kan ha inaktiverat applikationslösenord. Besök { $support_url } för att lära dig mer.

xmlrpc_endpoint_not_found = Webbplatsens XML-RPC-ändpunkt kunde inte hittas. Kontrollera dina webbplatsinställningar och försök igen.

xmlrpc_disabled_by_multiple_plugins = Webbplatsens XML-RPC är inaktiverad – det finns flera installerade tillägg som kan ha inaktiverat XML-RPC. Inaktivera dem och försök igen.

xmlrpc_disabled_by_plugin = Webbplatsens XML-RPC är inaktiverad – tillägget { $plugin } kan ha inaktiverat XML-RPC. Besök { $support_url } för att lära dig mer.

xmlrpc_disabled_by_host = Webbplatsens XML-RPC är inaktiverat. Kontakta ditt webbhotell för att lösa detta problem.

rest_api_disabled = Webbplatsens REST API är inaktiverat. Uppdatera dina webbplatsinställningar för att aktivera REST API.

wordpress_org_api_client_error_request_encoding = Misslyckades att koda begäran. Orsak: { $reason }.

uuid_parse_error_not_version_4 = Inte en version 4 UUID.

uuid_parse_error_invalid_uuid = Ogiltig UUID-sträng.

uniffi_serialization_error_serde = Serialiseringsfel: { $reason }.

http_auth_method_unknown = Okänd HTTP-autentiseringsmetod.

http_auth_method_missing_algorithm = Saknar algoritm i HTTP-autentiseringsmetod.

http_auth_method_missing_qop = Saknar QOP (Quality of Protection) i HTTP-autentiseringsmetod.

http_auth_method_missing_nonce = Saknar engångskod i HTTP-autentiseringsmetod.

invalid_header_value_error = Ogiltigt header-värde { $header_value }.

invalid_header_name_error = Ogiltigt header-namn: { $header_name }.

oauth_response_url_error_unsuccessful_login = Inloggning misslyckades.

oauth_response_url_error_url_invalid = Webbplatsen skickade en ogiltig URL för autentiseringssvar.

misconfigured_http_authentication_error = Servern skickar ogiltig HTTP-autentiseringsinformation. Kontrollera din webbplats HTTP-autentiseringskonfiguration.

http_server_error = Kan inte ansluta till server: { $reason }. Kontakta din serverleverantör.

http_authentication_rejected_error = Servern på { $url } avvisade dina autentiseringsuppgifter. Ange ett giltigt användarnamn och lösenord.

http_timeout_error = Anslutning timeout

http_forbidden_error = Servern på { $url } nekade åtkomst till den begärda resursen. Kontrollera din webbplats konfiguration.

http_authentication_required_error = Servern på { $url } kräver autentisering. Ange ditt användarnamn och lösenord.

non_existent_site_error = En server med specificerat webbhotell kunde inte hittas.

invalid_ssl_error_generic_ssl_error = Kan inte etablera en säker anslutning till servern

invalid_ssl_error_certificate_not_valid_for_name = Ogiltigt SSL-ceritfikat

just = { $message }

request_execution_failed = Misslyckades att skicka HTTP.

invalid_http_status_code = Ogiltig HTTP-statuskod: { $status_code }.

media_file_not_found = Mediafil hittades inte på { $path }.

response_parsing_error = Svar kunde inte tolkas: { $reason }.

url_parsing_error = URL är ogiltig.

wp_api_error_generic_error = Något gick fel.

probably_not_wordpress_site = Webbplatsen verkar inte vara en WordPress-webbplats.
