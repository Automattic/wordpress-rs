api_root_link_header_not_found = ไม่พบลิงก์ WordPress REST API ในการตอบกลับของไซต์

generic_error = เกิดข้อผิดพลาด

site_error_message = ไซต์ของคุณส่งข้อความผิดพลาด: {$error_message}

url_parsing_error = URL ไม่ถูกต้อง

response_parsing_error = ไม่สามารถประมวลผลการตอบกลับได้: {$reason}

media_file_not_found = ไม่พบไฟล์สื่อที่ {$path}

invalid_http_status_code = รหัสสถานะ HTTP ไม่ถูกต้อง: {$status_code}

request_execution_failed = การส่ง HTTP ล้มเหลว

just = {$message}

invalid_ssl_error = ใบรับรอง SSL ไม่ถูกต้อง

non_existent_site_error = ไม่พบเซิร์ฟเวอร์ที่มีชื่อโฮสต์ที่ระบุ

http_authentication_required_error = เซิร์ฟเวอร์ที่ {$url} ต้องการการตรวจสอบตัวตน กรุณากรอกชื่อผู้ใช้และรหัสผ่านของคุณ

http_authentication_rejected_error = เซิร์ฟเวอร์ที่ {$url} ปฏิเสธข้อมูลการเข้าสู่ระบบของคุณ กรุณากรอกชื่อผู้ใช้และรหัสผ่านที่ถูกต้อง

misconfigured_http_authentication_error = เซิร์ฟเวอร์ส่งข้อมูลการตรวจสอบตัวตน HTTP ที่ไม่ถูกต้อง กรุณาตรวจสอบการตั้งค่าการตรวจสอบตัวตน HTTP ของไซต์ของคุณ

misconfigured_rate_limit_error = เซิร์ฟเวอร์จำกัดการร้องขอในลักษณะที่ไม่สามารถสำเร็จได้ กรุณาตรวจสอบการตั้งค่าการจำกัดอัตราของไซต์ของคุณ

oauth_response_url_error_missing_site_url = URL ที่ให้มาไม่มีพารามิเตอร์การค้นหา `site_url`
oauth_response_url_error_missing_username = URL ที่ให้มาไม่มีพารามิเตอร์การค้นหา `username`
oauth_response_url_error_missing_password = URL ที่ให้มาไม่มีพารามิเตอร์การค้นหา `password`
oauth_response_url_error_unsuccessful_login = การเข้าสู่ระบบล้มเหลว

boolean_true_is_returned_when_string_is_expected = คาดหวังค่า `String` สำหรับฟิลด์นี้ แต่ได้รับค่า boolean `true`

invalid_header_name_error = ชื่อส่วนหัวไม่ถูกต้อง: {$header_name}

invalid_header_value_error = ค่าส่วนหัวไม่ถูกต้อง: {$header_value}

http_auth_method_missing_nonce = ขาด nonce ในวิธีการตรวจสอบตัวตน HTTP
http_auth_method_missing_qop = ขาด QOP (Quality of Protection) ในวิธีการตรวจสอบตัวตน HTTP
http_auth_method_missing_algorithm = ขาดอัลกอริทึมในวิธีการตรวจสอบตัวตน HTTP
http_auth_method_missing_opaque = ขาดค่า opaque ในวิธีการตรวจสอบตัวตน HTTP
http_auth_method_unknown = ไม่รู้จักวิธีการตรวจสอบตัวตน HTTP

uniffi_serialization_error_serde = ข้อผิดพลาดในการแปลงข้อมูล: {$reason}

uuid_parse_error_invalid_uuid = สตริง UUID ไม่ถูกต้อง
uuid_parse_error_not_version_4 = ไม่ใช่ UUID เวอร์ชัน 4

wordpress_org_api_client_error_request_encoding = ไม่สามารถเข้ารหัสคำขอได้ เหตุผล: {$reason}
