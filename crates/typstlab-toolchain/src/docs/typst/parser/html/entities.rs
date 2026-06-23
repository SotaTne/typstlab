pub fn decode_entities(input: &str) -> String {
    html_escape::decode_html_entities(input).into_owned()
}
