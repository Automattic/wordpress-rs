api_root_link_header_not_found = Link do WordPress REST API não encontrado na resposta do site

generic_error = Algo deu errado

site_error_message = Seu site enviou uma mensagem de erro: {$error_message}

url_parsing_error = URL inválida

response_parsing_error = Não foi possível processar a resposta: {$reason}

media_file_not_found = Arquivo de mídia não encontrado no caminho {$path}

invalid_http_status_code = Código de status HTTP inválido: {$status_code}

request_execution_failed = Falha ao enviar HTTP

just = {$message}

invalid_ssl_error = Certificado SSL inválido

non_existent_site_error = Não foi possível encontrar um servidor com o nome do computador especificado

http_authentication_required_error = O servidor em {$url} requer autenticação. Digite seu nome de usuário e senha

http_authentication_rejected_error = O servidor em {$url} rejeitou suas credenciais de login. Digite um nome de usuário e senha válidos.

misconfigured_http_authentication_error = O servidor está enviando dados de autenticação HTTP inválidos. Verifique a configuração de autenticação HTTP do seu site

misconfigured_rate_limit_error = O servidor está limitando solicitações de uma forma que nunca terá sucesso. Verifique a configuração de limite de taxa do seu site

oauth_response_url_error_missing_site_url = A URL fornecida não contém o parâmetro de consulta `site_url`
oauth_response_url_error_missing_username = A URL fornecida não contém o parâmetro de consulta `username`
oauth_response_url_error_missing_password = A URL fornecida não contém o parâmetro de consulta `password`
oauth_response_url_error_unsuccessful_login = Login mal sucedido

boolean_true_is_returned_when_string_is_expected = Espera-se um valor `String` para este campo, mas foi recebido um valor booleano `true`

invalid_header_name_error = Nome do cabeçalho inválido: {$header_name}

invalid_header_value_error = Valor do cabeçalho inválido: {$header_value}

http_auth_method_missing_nonce = Falta nonce no método de autenticação HTTP
http_auth_method_missing_qop = Falta QOP (Qualidade de Proteção) no método de autenticação HTTP
http_auth_method_missing_algorithm = Falta algoritmo no método de autenticação HTTP
http_auth_method_missing_opaque = Falta valor opaco no método de autenticação HTTP
http_auth_method_unknown = Método de autenticação HTTP desconhecido

uniffi_serialization_error_serde = Erro de serialização: {$reason}

uuid_parse_error_invalid_uuid = String UUID inválida
uuid_parse_error_not_version_4 = Não é um UUID versão 4

wordpress_org_api_client_error_request_encoding = Falha ao codificar a solicitação. Razão: {$reason}

http_forbidden_error = O servidor em {$url} negou acesso ao recurso solicitado. Verifique a configuração do seu site
