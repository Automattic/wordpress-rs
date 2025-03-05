api_root_link_header_not_found = Link do WordPress REST API não encontrado na resposta do site

generic_error = Algo deu errado

site_error_message = Seu site enviou uma mensagem de erro: {$error_message}

url_parsing_error = URL inválida

response_parsing_error = Não foi possível processar a resposta: {$reason}

media_file_not_found = Arquivo de mídia não encontrado no caminho {$path}

invalid_http_status_code = Código de status HTTP inválido: {$status_code}

request_execution_failed = A requisição HTTP falhou

just = {$message}

invalid_ssl_error = Certificado SSL inválido

non_existent_site_error = Servidor com o nome de computador especificado não encontrado

http_authentication_required_error = O servidor em {$url} requer autenticação. Por favor, insira seu nome de usuário e senha

http_authentication_rejected_error = O servidor em {$url} rejeitou suas credenciais de login. Por favor, insira o nome de usuário e senha corretos.

misconfigured_http_authentication_error = O servidor está enviando dados de autenticação HTTP inválidos. Verifique a configuração de autenticação HTTP do seu site

misconfigured_rate_limit_error = O servidor está limitando as requisições para que nunca tenham sucesso. Verifique os limites de taxa do seu site

oauth_response_url_error_missing_site_url = A URL fornecida não contém o parâmetro de solicitação `site_url`
oauth_response_url_error_missing_username = A URL fornecida não contém o parâmetro de solicitação `username`
oauth_response_url_error_missing_password = A URL fornecida não contém o parâmetro de solicitação `password`
oauth_response_url_error_unsuccessful_login = O login não foi bem-sucedido

boolean_true_is_returned_when_string_is_expected = Para este campo, é esperado um valor `String`, mas foi recebido um valor booleano `true`

invalid_header_name_error = Nome de cabeçalho inválido: {$header_name}

invalid_header_value_error = Valor de cabeçalho inválido: {$header_value}

http_auth_method_missing_nonce = O método de autenticação HTTP não possui nonce
http_auth_method_missing_qop = O método de autenticação HTTP não possui QOP (Quality of Protection)
http_auth_method_missing_algorithm = O método de autenticação HTTP não possui algoritmo
http_auth_method_missing_opaque = O método de autenticação HTTP não possui valor opaco
http_auth_method_unknown = Método de autenticação HTTP desconhecido

uniffi_serialization_error_serde = Erro de serialização: {$reason}

uuid_parse_error_invalid_uuid = String UUID inválida
uuid_parse_error_not_version_4 = A versão do UUID não é 4

wordpress_org_api_client_error_request_encoding = Não foi possível codificar a solicitação. Razão: {$reason}
